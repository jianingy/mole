// Jianing Yang <jianingy.yang@gmail.com> @ 22 Sep, 2016

use clap::ArgMatches;
use hyper;
use net2::TcpBuilder;
use serde_json::{self, Value};
use std::fs::File;
use std::io::prelude::*;
use std::io::{Result as IoResult, Error as IoError, ErrorKind as IoErrorKind};
use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std;

use db_api;
use iprange;
use detection;

#[derive(Debug, Clone)]
struct ScanOptions {
    reference: String,
    httpbin: String,
    num_workers: usize,
    timeout: Duration,
    gateway: Option<String>,
}

// XXX: 1. Verify "CONNECT" Ports
//      2. save vanilla and traceable bits
macro_rules! io_error {
    ( $( $a:expr ),* ) => {
        IoError::new(IoErrorKind::Other, format!( $( $a ),* ).as_str());
    };
}

pub fn run_scan(opts: ArgMatches) {
    info!("starting scanner ...");
    let network = iprange::Ipv4Network::from_str(opts.value_of("network").unwrap())
        .expect("you must specify a valid network expression for --network");
    let dbname = opts.value_of("database").unwrap().to_string();

    let ports = opts.values_of("ports").unwrap().collect::<Vec<_>>();

    // prepare workers' data
    let mut servers = Vec::new();
    for (ip, port) in iproduct!(network.iter(), ports.iter()) {
        let port = port.parse::<u16>().expect("one of ports is not a number");
        servers.push((ip, port));
    }
    let db = db_api::init_db(&dbname).unwrap();
    db_api::init_table(db.get().unwrap()).unwrap();

    scan(db,
         servers.into_iter(),
         ScanOptions {
             gateway: get_gateway_ip(),
             reference: opts.value_of("reference").unwrap().to_string(),
             httpbin: opts.value_of("httpbin").unwrap().to_string(),
             timeout: Duration::new(opts.value_of("timeout")
                                        .unwrap()
                                        .parse::<u64>()
                                        .unwrap(),
                                    0),
             num_workers: opts.value_of("workers")
                 .unwrap()
                 .parse::<usize>()
                 .expect("you must specify a number for --workers"),
         });
    info!("scan completed.");
}

pub fn run_verify(opts: ArgMatches) {
    info!("starting verification ...");
    let dbname = opts.value_of("database").unwrap().to_string();
    let db = db_api::init_db(&dbname).unwrap();

    db_api::init_table(db.get().unwrap()).unwrap();
    let servers = db_api::get_proxy_servers(db.get().unwrap()).unwrap();
    scan(db,
         servers.into_iter().map(|x| (x.host, x.port)),
         ScanOptions {
             gateway: get_gateway_ip(),
             reference: opts.value_of("reference").unwrap().to_string(),
             httpbin: opts.value_of("httpbin").unwrap().to_string(),
             timeout: Duration::new(opts.value_of("timeout")
                                        .unwrap()
                                        .parse::<u64>()
                                        .unwrap(),
                                    0),
             num_workers: opts.value_of("workers")
                 .unwrap()
                 .parse::<usize>()
                 .expect("you must specify a number for --workers"),
         });
    info!("verification completed.");
}

pub fn run_import(opts: ArgMatches) -> IoResult<()> {
    let dbname = opts.value_of("database").unwrap().to_string();
    let db = db_api::init_db(&dbname).unwrap();
    db_api::init_table(db.get().unwrap()).unwrap();

    let path = Path::new(opts.value_of("file").unwrap());
    let mut file = try!(File::open(&path));
    let mut content = String::new();
    try!(file.read_to_string(&mut content));

    let servers: Vec<(&str, u16)> = content.lines()
        .map(|x| {
            let mut s = x.split(':');
            match (s.next(), s.next()) {
                (Some(host), Some(port)) => {
                    match port.parse() {
                        Ok(port) => Some((host, port)),
                        _ => None,
                    }
                }
                _ => None,
            }
        })
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect();
    for (host, port) in servers {
        let conn = db.get().unwrap();
        if let Ok(x) = db_api::ProxyServer::new(host, port) {
            info!("adding server {}.", x);
            db_api::add_proxy(conn, x).unwrap();
        } else {
            warn!("server address/port incorrect");
        }
    }
    Ok(())
}

fn http_request(server: Ipv4Addr, port: u16, content: &str, timeout: Duration) -> IoResult<String> {
    let tcp = try!(TcpBuilder::new_v4());
    let mut stream = try!(tcp.connect((server, port)));
    try!(stream.set_read_timeout(Some(timeout)));
    try!(stream.set_write_timeout(Some(timeout)));
    let mut resp = String::new();
    let _ = stream.write(content.as_bytes());
    let _ = stream.read_to_string(&mut resp);
    Ok(resp)
}

fn get_gateway_ip() -> Option<String> {
    debug!("determing your gateway ip address ...");
    let client = hyper::client::Client::new();
    let mut resp = client.get("http://ifconfig.in/").send().unwrap();
    let ip = if resp.status == hyper::Ok {
        let mut ip = String::new();
        if let Err(_) = resp.read_to_string(&mut ip) {
            None
        } else {
            info!("found gateway ip {}. tracing detection enabled.", ip);
            Some(ip)
        }
    } else {
        None
    };
    if ip.is_none() {
        warn!("no gateway address found. tracing detection disabled.");
    }
    ip
}

fn verify_server(host: Ipv4Addr, port: u16, opts: &ScanOptions) -> IoResult<db_api::ProxyServer> {
    trace!("connecting {:?}:{:?} ...", host, port);
    // verify regular proxy request
    let request = format!("GET http://{host}/headers HTTP/1.0\r\n\
                           Host: {host}\r\n\r\n",
                          host = opts.httpbin);
    let resp = try!(http_request(host, port, request.as_str(), opts.timeout));
    let body = try!(resp.splitn(2, "\r\n\r\n")
        .last()
        .ok_or(io_error!("malformed HTTP response")));
    let data: Value = try!(serde_json::from_str(body)
        .map_err(|_| io_error!("httpbin returns malformed json")));
    let data = try!(data.find("headers")
        .ok_or(io_error!("httpbin returns incompleted data")));
    let headers = try!(data.as_object()
        .ok_or(io_error!("httpbin returns incompleted data")));
    let mut traceable = false;
    if opts.gateway.is_some() {
        let gateway = opts.gateway.clone().unwrap();
        for (_, val) in headers {
            trace!("matching header with gateway ip: {:?} vs {:?}",
                   val,
                   gateway);
            match val {
                &Value::String(ref x) if x.find(&gateway).is_some() => {
                    traceable = true;
                    break;
                }
                _ => {}
            }
        }
    }
    debug!("{:?}:{:?} returns {:?}", host, port, headers);
    let tags = detect_server(host, port, opts.timeout).ok();
    let lag = ping_reference(host, port, &opts.reference, opts.timeout);
    info!("{}/{}: {:?}", host, port, tags);
    Ok(db_api::ProxyServer {
        host: host,
        port: port,
        lag: lag.ok(),
        tags: tags,
        vanilla: Some(headers.len() == 1), // no headers been added
        traceable: Some(traceable),
    })
}

fn ping_reference(host: Ipv4Addr,
                  port: u16,
                  reference: &str,
                  timeout: Duration)
                  -> IoResult<Duration> {
    let started = Instant::now();
    let request = format!("GET http://{0} HTTP/1.0\r\nHost: {0}\r\n\r\n", reference);
    let _ = try!(http_request(host, port, request.as_str(), timeout));
    Ok(Instant::now() - started)
}

fn detect_server(host: Ipv4Addr, port: u16, timeout: Duration) -> IoResult<Vec<String>> {
    let mut tags = Vec::new();
    let rules = try!(detection::rules());
    for (tag, request, needle) in rules {
        let resp = match http_request(host, port, request.as_str(), timeout) {
            Ok(x) => x,
            _ => continue
        };
        info!("checking {} for {}/{}: {}", tag, host, port, resp);
        trace!("detect {}/{}/{} => {}", tag, request, needle, resp);
        if let Some(_) = resp.find(needle.as_str()) {
            tags.push(tag)
        }
    }
    Ok(tags)
}

fn scan<I>(db: db_api::Pool, servers: I, opts: ScanOptions)
    where I: 'static + Iterator<Item = (Ipv4Addr, u16)> + std::marker::Send
{
    let total_servers = match servers.size_hint() {
        (_, Some(high)) => Some(high),
        _ => None,
    };
    let queue = Arc::new(Mutex::new(servers));
    let mut workers = Vec::new();

    for _ in 0..opts.num_workers {
        let (db, opts, queue) = (db.clone(), opts.clone(), queue.clone());
        let worker = thread::spawn(move || {
            loop {
                let (host, port) = if let Ok(mut queue) = queue.lock() {
                    match queue.next() {
                        Some(x) => x,
                        _ => return,
                    }
                } else {
                    // lock has been poisioned. we quit here.
                    return;
                };
                match verify_server(host, port, &opts) {
                    Ok(server) => {
                        // XXX: Get rid of this unwrap, 'cause it will happen in runtime
                        let conn = db.get().unwrap();
                        db_api::add_proxy(conn, server).unwrap();
                    }
                    Err(e) => {
                        debug!("error on verifying server {:?}:{:?}: {:?}", host, port, e);
                        let server = db_api::ProxyServer::new(&host.to_string(), port).unwrap();
                        let conn = db.get().unwrap();
                        db_api::disable_proxy(conn, server).unwrap();
                    }
                }
            }
        });
        workers.push(worker);
    }
    info!("workers started (# = {}).", workers.len());

    // start status monitor
    let (tx, rx) = mpsc::channel();
    let monitor = thread::spawn(move || {
        loop {
            let remain_servers = match queue.lock() {
                Ok(x) => {
                    match x.size_hint() {
                        (_, Some(high)) => Some(high),
                        _ => None,
                    }
                }
                _ => continue,
            };

            if let (Some(total), Some(remain)) = (total_servers, remain_servers) {
                info!("progress {2:.2}% ({0}/{1}).",
                      total - remain,
                      total,
                      100f64 * ((total - remain) as f64) / (total as f64));
            }
            thread::sleep(Duration::new(15, 0));
            if let Ok(_) = rx.try_recv() {
                return;
            }
        }
    });

    while let Some(worker) = workers.pop() {
        let _ = worker.join();
    }
    tx.send(true).unwrap();
    let _ = monitor.join();
}

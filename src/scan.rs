/*

 This piece of code is written by
    Jianing Yang <jianingy.yang@gmail.com>
 with love and passion!

        H A P P Y    H A C K I N G !
              _____               ______
     ____====  ]OO|_n_n__][.      |    |
    [________]_|__|________)<     |YANG|
     oo    oo  'oo OOOO-| oo\\_   ~o~~o~
 +--+--+--+--+--+--+--+--+--+--+--+--+--+
                             11 Sep, 2016

 */
use clap::ArgMatches;
use iprange;
use serde_json::{self, Value};
use std::collections::VecDeque;
use std::io::prelude::*;
use std::io::{Result as IoResult, Error as IoError, ErrorKind as IoErrorKind};
use std::net::Ipv4Addr;
use std::net::TcpStream;
use net2::TcpBuilder;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use db_api;

macro_rules! io_error {
    ( $( $a:expr ),* ) => {
        IoError::new(IoErrorKind::Other, format!( $( $a ),* ).as_str());
    };
}

pub fn run_scan(opts: ArgMatches) {
    // command line arguments
    info!("starting program ...");
    let network =
        iprange::Ipv4Network::from_str(opts.value_of("network").unwrap())
        .expect("you must specify a valid network expression for --network");
    let dbname = opts.value_of("database").unwrap().to_string();
    let httpbin = Arc::new(opts.value_of("httpbin").unwrap().to_string());
    let reference = Arc::new(opts.value_of("reference").unwrap().to_string());
    let ports = opts.values_of("ports").unwrap().collect::<Vec<_>>();
    let timeout = Duration::new(opts.value_of("timeout").unwrap()
                                .parse::<u64>().unwrap(), 0);
    let num_workers =
        opts.value_of("workers").unwrap().parse::<usize>()
        .expect("you must specify a number for --workers");

    // prepare workers' data
    let mut work_queue = VecDeque::new();
    for (ip, port) in iproduct!(network.iter(), ports.iter()) {
        let port = port.parse::<u16>().expect("one of ports is not a number");
        work_queue.push_back((ip, port));
    }
    let total_servers = work_queue.len();
    let queue = Arc::new(Mutex::new(work_queue));
    let db_pool = db_api::init_db(&dbname);
    let mut workers = Vec::new();

    // spawn workers
    info!("spawning workers ...");
    db_api::init_table(db_pool.get().unwrap());
    for _ in 0..num_workers {
        let reference = reference.clone();
        let httpbin = httpbin.clone();
        let queue = queue.clone();
        let db_pool = db_pool.clone();
        let worker = thread::spawn(move || {
            loop {
                let (server, port) = if let Ok(mut queue) = queue.lock() {
                    match queue.pop_front() {
                        Some(x) => x,
                        _ => return
                    }
                } else {
                    // lock has been poisioned. we quit here.
                    return
                };
                match verify_server(server, port, &httpbin, timeout) {
                    Ok(_) => {
                        // do latency testing
                        let lag = evaluate_server(
                            server, port, timeout, &reference).ok();
                        // XXX: Get rid of this unwrap, 'cause it will happen in runtime
                        let db = db_pool.get().unwrap();
                        db_api::add_proxy(db, server, port, lag);
                        info!("found: {:?}:{:?} with lag {:?}", server, port, lag);
                    },
                    Err(e) => {
                        debug!("error on verifying server {:?}:{:?}: {:?}",
                               server, port, e);
                    }
                }
            }
        });
        workers.push(worker);
    }
    info!("{} workers spawned.", workers.len());

    // spawn status
    let monitor = thread::spawn(move || {
        loop {
            let remain_servers = match queue.lock() {
                Ok(x) => x.len(),
                _ => continue
            };
            info!("current progress: {}/{}.",
                  total_servers - remain_servers, total_servers);
            if remain_servers < 1 {
                info!("scan completed. waiting workers to terminate.");
                return;
            }
            thread::sleep(Duration::new(5, 0));
        }
    });

    while let Some(worker) = workers.pop() {
        let _ = worker.join();
    }
    let _ = monitor.join();
    info!("program exited.");
}

pub fn run_verify(opts: ArgMatches) {

}

fn verify_server(server: Ipv4Addr, port: u16, httpbin: &str, timeout: Duration)
                 -> IoResult<()>
{
    trace!("verifying {:?}:{:?} ...", server, port);
    // verify regular proxy request
    let tcp = try!(TcpBuilder::new_v4());
    let mut stream = try!(tcp.connect((server, port)));
    try!(stream.set_read_timeout(Some(timeout)));
    try!(stream.set_write_timeout(Some(timeout)));
    let mut resp = String::new();
    let _ = stream.write(
        format!("GET http://{host}/headers HTTP/1.0\r\n\
                 Host: {host}\r\n\r\n", host=httpbin).as_bytes());
    let _ = stream.read_to_string(&mut resp);
    let body = try!(resp.splitn(2, "\r\n\r\n").last()
                    .ok_or(io_error!("malformed HTTP response")));
    let data: Value = try!(serde_json::from_str(body)
            .map_err(|_| io_error!("httpbin returns malformed json")));
    let data = try!(data.find("headers")
                    .ok_or(io_error!("httpbin returns incompleted data")));
    let headers = try!(data.as_object()
                       .ok_or(io_error!("httpbin returns incompleted data")));
    debug!("found {:?}:{:?}: {:?}", server, port, headers);
    Ok(())
}

fn evaluate_server(server: Ipv4Addr, port: u16,
                   timeout: Duration, reference: &str)
                   -> IoResult<Duration>
{
    let started = Instant::now();
    let mut stream = try!(TcpStream::connect((server, port)));
    try!(stream.set_read_timeout(Some(timeout)));
    try!(stream.set_write_timeout(Some(timeout)));
    let mut resp = String::new();
    let _ = stream.write(format!("GET {} HTTP/1.0\r\n\r\n", reference).as_bytes());
    let _ = stream.read_to_string(&mut resp);
    Ok(Instant::now() - started)
}

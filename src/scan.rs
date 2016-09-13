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
use std::collections::{VecDeque, HashMap};
use std::io::prelude::*;
use std::net::Ipv4Addr;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use serde_json::{self, Value};
use iprange;


pub fn run_scan(opts: ArgMatches) {
    let network =
        iprange::Ipv4Network::from_str(opts.value_of("network").unwrap())
        .expect("you must specify a valid network expression for --network");
    let ports = opts.values_of("ports").unwrap().collect::<Vec<_>>();
    let num_workers =
        opts.value_of("workers").unwrap().parse::<usize>()
        .expect("you must specify a number for --workers");
    let mut work_queue = VecDeque::new();
    for (ip, port) in iproduct!(network.iter(), ports.iter()) {
        let port = port.parse::<u16>().expect("one of ports is not a number");
        work_queue.push_back((ip, port));
    }

    let queue = Arc::new(Mutex::new(work_queue));
    let mut workers = Vec::new();
    info!("spawning workers (max = {})", num_workers);
    for _ in 0..num_workers {
        let queue = queue.clone();
        let worker = thread::spawn(move || {
            loop {
                let (server, port) = if let Ok(mut queue) = queue.lock() {
                    if let Some(x) = queue.pop_front() {
                        x
                    } else {
                        // nothing left. just quit.
                        return ();
                    }
                } else {
                    // lock has been poisioned. we quit here.
                    return ();
                };
                scan_single(server, port);
            }
        });
        workers.push(worker);
    }

    while let Some(worker) = workers.pop() {
        let _ = worker.join();
    }

}

fn scan_single(server: Ipv4Addr, port: u16) {
    info!("verifying {:?}:{:?} ...", server, port);
    // verify regular proxy request
    let mut stream = match TcpStream::connect((server, port)) {
        Ok(stream) => stream,
        _ => {
            debug!("connection failed: {:?}:{:?}", server, port);
            return
        }
    };
    let mut resp = String::new();
    let _ = stream.write("GET http://httpbin.org/headers HTTP/1.0\r\n\r\n".as_bytes());
    let _ = stream.read_to_string(&mut resp);
    let body = match resp.splitn(2, "\r\n\r\n").last() {
        Some(body) => body,
        _ => return
    };
    let data: Value = match serde_json::from_str(body) {
        Ok(value) => value,
        _ => return
    };
    let headers = match data.find("headers") {
        Some(headers) => match headers.as_object() {
            Some(x) => x,
            _ => return
        },
        _ => return
    };
    println!("verfied: {:?}:{:?}: {:?}", server, port, headers);
}

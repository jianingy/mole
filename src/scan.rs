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
use std::net::Ipv4Addr;

use iprange;

pub fn run_scan(opts: ArgMatches) {
    let network =
        iprange::Ipv4Network::from_str(opts.value_of("network").unwrap())
        .expect("invalid network expression");
    for ip in network.iter() {
        println!("{:?}", ip);
    }
}

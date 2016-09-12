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

#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate clap;
extern crate env_logger;
extern crate regex;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use env_logger::LogBuilder;
use log::{LogRecord, LogLevelFilter};

mod iprange;
mod scan;

static VERSION: &'static str = "0.1.0";

lazy_static! {
    static ref OPTIONS: ArgMatches<'static> = {
        App::new("mole")
            .version(VERSION)
            .about("a tool for finding http proxies")
            .setting(AppSettings::SubcommandRequired)
            .setting(AppSettings::ColoredHelp)
            .arg(Arg::with_name("verbose")
                 .short("v")
                 .multiple(true))
            .subcommand(SubCommand::with_name("scan")
                        .about("find proxy servers in specified network")
                        .arg(Arg::with_name("workers")
                             .long("workers")
                             .takes_value(true)
                             .default_value("4")
                             .help("# of concurrent workers"))
                        .arg(Arg::with_name("ports")
                             .long("ports")
                             .takes_value(true)
                             .default_value("3128,8080")
                             .value_delimiter(",")
                             .help("ports to scan"))
                        .arg(Arg::with_name("network")
                             .required(true)
                             .takes_value(true)
                             .help("network to scan")))
            .get_matches()

    };
}

fn init_logger() {
    let log_format = |record: &LogRecord| {
        format!("[{}] {}", record.level(), record.args())
    };
    let mut builder = LogBuilder::new();
    builder.format(log_format)
        .filter(None, match OPTIONS.occurrences_of("verbose") {
            n if n > 1 => LogLevelFilter::Debug,
            n if n == 1 => LogLevelFilter::Info,
            _ => LogLevelFilter::Warn
        });
    builder.init().unwrap();
}

fn main() {
    init_logger();

    if let Some(subopts) = OPTIONS.subcommand_matches("scan") {
        scan::run_scan(subopts.clone());
    }
}

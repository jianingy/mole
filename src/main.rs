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
#[macro_use] extern crate itertools;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate ansi_term;
extern crate clap;
extern crate env_logger;
extern crate net2;
extern crate r2d2;
extern crate r2d2_sqlite;
extern crate libsqlite3_sys;
extern crate regex;
extern crate rusqlite;
extern crate serde;
extern crate serde_json;

use ansi_term::Colour as TermColor;
use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use env_logger::LogBuilder;
use log::{LogRecord, LogLevel, LogLevelFilter};

mod db_api;
mod iprange;
mod scan;

static VERSION: &'static str = "0.1.0";

lazy_static! {
    static ref OPTIONS: ArgMatches<'static> = {
        App::new("mole")
            .version(VERSION)
            .about("a tool for finding http proxy servers")
            .setting(AppSettings::SubcommandRequired)
            .global_setting(AppSettings::ColoredHelp)
            .arg(Arg::with_name("verbose")
                 .short("v")
                 .multiple(true))
            .subcommand(SubCommand::with_name("scan")
                        .about("find proxy servers in specified network")
                        .arg(Arg::with_name("timeout")
                             .long("timeout")
                             .takes_value(true)
                             .default_value("15")
                             .help("# of seconds before given up a verification"))
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
                        .arg(Arg::with_name("httpbin")
                             .long("httpbin")
                             .takes_value(true)
                             .default_value("httpbin.org")
                             .help("httpbin server for proxy verification"))
                        .arg(Arg::with_name("reference")
                             .long("reference")
                             .takes_value(true)
                             .default_value("http://www.baidu.com/")
                             .help("reference for calculating latency"))
                        .arg(Arg::with_name("database")
                             .long("database")
                             .takes_value(true)
                             .default_value(".mole.sqlite")
                             .help("path to database file"))
                        .arg(Arg::with_name("network")
                             .required(true)
                             .takes_value(true)
                             .help("network to scan")))
            .subcommand(SubCommand::with_name("verify")
                        .about("verify servers in the database")
                        .arg(Arg::with_name("timeout")
                             .long("timeout")
                             .takes_value(true)
                             .default_value("15")
                             .help("# of seconds before given up a verification"))
                        .arg(Arg::with_name("workers")
                             .long("workers")
                             .takes_value(true)
                             .default_value("4")
                             .help("# of concurrent workers"))
                        .arg(Arg::with_name("httpbin")
                             .long("httpbin")
                             .takes_value(true)
                             .default_value("httpbin.org")
                             .help("httpbin server for proxy verification"))
                        .arg(Arg::with_name("reference")
                             .long("reference")
                             .takes_value(true)
                             .default_value("http://www.baidu.com/")
                             .help("reference for calculating latency"))
                        .arg(Arg::with_name("database")
                             .long("database")
                             .takes_value(true)
                             .default_value(".mole.sqlite")
                             .help("path to database file")))
            .get_matches()

    };
}

fn init_logger() {
    let log_format = |record: &LogRecord| {
        let message = format!("[{}] {}", match record.level() {
            LogLevel::Error => "!",
            LogLevel::Warn => "*",
            LogLevel::Info => "+",
            LogLevel::Debug => "#",
            LogLevel::Trace => "~",
        }, record.args());
        match record.level() {
            LogLevel::Error  => TermColor::Red.paint(message),
            LogLevel::Warn   => TermColor::Yellow.paint(message),
            LogLevel::Info   => TermColor::Green.paint(message),
            LogLevel::Debug  => TermColor::Blue.paint(message),
            LogLevel::Trace  => TermColor::White.paint(message),
        }.to_string()
    };
    let mut builder = LogBuilder::new();
    builder.format(log_format)
        .filter(None, match OPTIONS.occurrences_of("verbose") {
            n if n > 2 => LogLevelFilter::Trace,
            n if n == 2 => LogLevelFilter::Debug,
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

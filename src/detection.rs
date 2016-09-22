// Jianing Yang <jianingy.yang@gmail.com> @ 22 Sep, 2016
use clap::ArgMatches;
use std::io::{Result as IoResult, Error as IoError, ErrorKind as IoErrorKind};

pub fn run_detection(args: ArgMatches) -> IoResult<()> {
    let text = include_str!("evaluation.inc");
    for line in text.lines() {
        println!("line {:?}", line);
    }
    Ok(())
}

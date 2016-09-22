// Jianing Yang <jianingy.yang@gmail.com> @ 22 Sep, 2016

use regex::{Regex, Captures};
use std::str::FromStr;
use std::result;
use std::net::Ipv4Addr;

lazy_static! {
    static ref RE_ADDR: Regex =
        Regex::new("^((?:[0-9]+[.]){3}[0-9]+)$").unwrap();
    static ref RE_CIDR: Regex =
        Regex::new("^((?:[0-9]+[.]){3}[0-9]+)/([0-9]+)$").unwrap();
    static ref RE_NETMASK: Regex =
        Regex::new("^((?:[0-9]+[.]){3}[0-9]+)/((?:[0-9]+[.]){3}[0-9]+)$").unwrap();
}

#[derive(Debug, PartialEq)]
pub enum Error {
    UnsupportedFormat(String),
    InvalidAddress(String),
    InvalidPrefix(String),
    InvalidNetmask(String),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub struct Ipv4NetworkIterator {
    current: u32,
    max: u32,
}

impl Iterator for Ipv4NetworkIterator {
    type Item = Ipv4Addr;

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = (self.max - self.current + 1) as usize;
        (0, Some(size))
    }

    fn next(&mut self) -> Option<Ipv4Addr> {
        if self.current <= self.max {
            let current = self.current;
            self.current = self.current + 1;
            Some(Ipv4Addr::from(current))
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Ipv4Network {
    network: u32,
    netmask: u32,
}

impl Ipv4Network {
    pub fn new(network: u32, netmask: u32) -> Ipv4Network {
        Ipv4Network {
            network: network,
            netmask: netmask,
        }
    }

    pub fn from_str(expr: &str) -> Result<Ipv4Network> {
        if let Some(found) = RE_NETMASK.captures(expr) {
            Ipv4Network::parse_netmask(&found)
        } else if let Some(found) = RE_CIDR.captures(expr) {
            Ipv4Network::parse_cidr(&found)
        } else if let Some(found) = RE_ADDR.captures(expr) {
            Ipv4Network::parse_address(&found)
        } else {
            Err(Error::UnsupportedFormat(expr.to_string()))
        }
    }

    pub fn iter(&self) -> Ipv4NetworkIterator {
        Ipv4NetworkIterator {
            current: self.network,
            max: self.network + !self.netmask,
        }
    }

    fn parse_address(found: &Captures) -> Result<Ipv4Network> {
        let s_ip = try!(found.at(1)
            .ok_or(Error::UnsupportedFormat("no ip address found".to_string())));
        let ip = try!(Ipv4Addr::from_str(s_ip)
            .map_err(|_| Error::InvalidAddress(s_ip.to_string())));
        let netmask = !(0);
        let network = ip.octets().iter().fold(0, |s, x| *x as u32 + (s << 8)) & netmask;
        Ok(Ipv4Network::new(network, netmask))
    }

    fn parse_netmask(found: &Captures) -> Result<Ipv4Network> {
        let s_ip = try!(found.at(1)
            .ok_or(Error::UnsupportedFormat("no ip address found".to_string())));
        let s_netmask = try!(found.at(2)
            .ok_or(Error::UnsupportedFormat("no netmask found".to_string())));
        let ip = try!(Ipv4Addr::from_str(s_ip)
            .map_err(|_| Error::InvalidAddress(s_ip.to_string())));
        let mut netmask = 0u32;
        for x in s_netmask.split('.') {
            let n = try!(x.parse::<u32>()
                .map_err(|_| Error::InvalidNetmask(s_netmask.to_string())));
            if n > 255 {
                return Err(Error::InvalidNetmask(s_netmask.to_string()));
            }
            netmask = n + (netmask << 8);
        }
        let network = ip.octets().iter().fold(0, |s, x| *x as u32 + (s << 8)) & netmask;
        Ok(Ipv4Network::new(network, netmask))
    }

    fn parse_cidr(found: &Captures) -> Result<Ipv4Network> {
        let s_ip = try!(found.at(1)
            .ok_or(Error::UnsupportedFormat("no ip address found".to_string())));
        let s_prefix = try!(found.at(2)
            .ok_or(Error::UnsupportedFormat("no ip prefix found".to_string())));
        let ip = try!(Ipv4Addr::from_str(s_ip)
            .map_err(|_| Error::InvalidAddress(s_ip.to_string())));
        let prefix = try!(s_prefix.parse::<u8>()
            .map_err(|_| Error::InvalidPrefix(s_prefix.to_string())));
        if prefix > 32 {
            return Err(Error::InvalidPrefix(s_prefix.to_string()));
        }
        let netmask = !((1 << (32 - prefix)) - 1);
        let network = ip.octets().iter().fold(0, |s, x| *x as u32 + (s << 8)) & netmask;
        Ok(Ipv4Network::new(network, netmask))
    }
}

#[test]
fn test_create_network_by_cidr() {
    assert_eq!(Ipv4Network::from_str("192.168.8.5/24").unwrap(),
               Ipv4Network {
                   network: 3232237568,
                   netmask: 4294967040,
               });
    assert_eq!(Ipv4Network::from_str("192.168.8.5/16").unwrap(),
               Ipv4Network {
                   network: 3232235520,
                   netmask: 4294901760,
               });
    assert_eq!(Ipv4Network::from_str("192.168.15.5/21").unwrap(),
               Ipv4Network {
                   network: 3232237568,
                   netmask: 4294965248,
               });
}

#[test]
fn test_create_network_by_netmask() {
    assert_eq!(Ipv4Network::from_str("192.168.8.5/255.255.255.0").unwrap(),
               Ipv4Network {
                   network: 3232237568,
                   netmask: 4294967040,
               });
    assert_eq!(Ipv4Network::from_str("192.168.8.5/255.255.0.0").unwrap(),
               Ipv4Network {
                   network: 3232235520,
                   netmask: 4294901760,
               });
    assert_eq!(Ipv4Network::from_str("192.168.15.5/255.255.248.0").unwrap(),
               Ipv4Network {
                   network: 3232237568,
                   netmask: 4294965248,
               });
}

#[test]
fn test_create_network_by_cidr_invalid() {
    assert_eq!(Ipv4Network::from_str("192.168.8.a/24").err(),
               Some(Error::UnsupportedFormat("192.168.8.a/24".to_string())));
    assert_eq!(Ipv4Network::from_str("192.168.8.5/2b").err(),
               Some(Error::UnsupportedFormat("192.168.8.5/2b".to_string())));
    assert_eq!(Ipv4Network::from_str("300.500.8.5/24").err(),
               Some(Error::InvalidAddress("300.500.8.5".to_string())));
    assert_eq!(Ipv4Network::from_str("192.168.8.5/99").err(),
               Some(Error::InvalidPrefix("99".to_string())));
}

#[test]
fn test_create_network_by_netmask_invalid() {
    assert_eq!(Ipv4Network::from_str("192.168.8.a/255.255.255.0").err(),
               Some(Error::UnsupportedFormat("192.168.8.a/255.255.255.0".to_string())));
    assert_eq!(Ipv4Network::from_str("192.168.8.5/255.255.25b.0").err(),
               Some(Error::UnsupportedFormat("192.168.8.5/255.255.25b.0".to_string())));
    assert_eq!(Ipv4Network::from_str("300.500.8.5/255.255.255.0").err(),
               Some(Error::InvalidAddress("300.500.8.5".to_string())));
    assert_eq!(Ipv4Network::from_str("192.168.8.5/300.400.500.600").err(),
               Some(Error::InvalidNetmask("300.400.500.600".to_string())));
}

#[test]
fn test_network_iter() {
    let network = Ipv4Network::from_str("192.168.1.0/29").unwrap();
    let result = ["192.168.1.0",
                  "192.168.1.1",
                  "192.168.1.2",
                  "192.168.1.3",
                  "192.168.1.4",
                  "192.168.1.5",
                  "192.168.1.6",
                  "192.168.1.7"];
    for (ip, t) in network.iter().zip(result.iter()) {
        assert_eq!(ip, Ipv4Addr::from_str(t).unwrap());
    }
}

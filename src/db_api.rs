// Jianing Yang <jianingy.yang@gmail.com> @ 22 Sep, 2016

use postgres::error;
use r2d2;
use r2d2_postgres::{SslMode, PostgresConnectionManager};
use serde_json::value::{ToJson, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::net::Ipv4Addr;
use std::result;
use std::str::FromStr;
use std::time::Duration;
use chrono::DateTime;
use chrono::offset::local::Local;

pub type Pool = r2d2::Pool<PostgresConnectionManager>;
type Connection = r2d2::PooledConnection<PostgresConnectionManager>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    fn new(kind: ErrorKind, message: &str) -> Error {
        Error {
            kind: kind,
            message: message.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    General,
    BadParameter,
}

#[derive(Debug)]
pub struct ProxyServer {
    pub host: Ipv4Addr,
    pub port: u16,
    pub lag: Option<Duration>,
    pub vanilla: Option<bool>,
    pub traceable: Option<bool>,
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Local>,
    pub updated_at: DateTime<Local>,
}

impl ProxyServer {
    pub fn new(host: &str,
               port: u16,
               lag: Option<Duration>,
               vanilla: Option<bool>,
               traceable: Option<bool>,
               tags: Option<Vec<String>>)
               -> Result<ProxyServer> {
        let host = try!(Ipv4Addr::from_str(host)
            .map_err(|_| Error::new(ErrorKind::BadParameter, "invalid ip adress")));
        Ok(ProxyServer {
            host: host,
            port: port,
            lag: lag,
            tags: tags,
            vanilla: vanilla,
            traceable: traceable,
            created_at: Local::now(),
            updated_at: Local::now(),
        })
    }
}

impl fmt::Display for ProxyServer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl ToJson for ProxyServer {
    fn to_json(&self) -> Value {
        let mut map = BTreeMap::new();
        map.insert("host".to_string(), self.host.to_json());
        map.insert("port".to_string(), self.port.to_json());
        if let Some(vanilla) = self.vanilla {
            map.insert("vanilla".to_string(), vanilla.to_json());
        }
        if let Some(traceable) = self.traceable {
            map.insert("traceable".to_string(), traceable.to_json());
        }
        if let Some(lag) = self.lag {
            map.insert("lag".to_string(), lag.as_secs().to_json());
        }
        if let &Some(ref tags) = &self.tags {
            map.insert("tags".to_string(), tags.to_json());
        }
        map.insert("created_at".to_string(), self.created_at.to_json());
        map.insert("updated_at".to_string(), self.updated_at.to_json());
        Value::Object(map)
    }
}

type Result<T> = result::Result<T, Error>;

macro_rules! db_try {
    ( $expr:expr ) => (
            match $expr {
                Ok(val) => val,
                Err(e) => {
                    let error = Error::new(ErrorKind::General,
                                           e.to_string().as_str());
                    return Err(error);
                }
            }
    );
}

pub fn init_db(name: &str) -> Result<Pool> {
    let config = r2d2::Config::default();
    let manager = match PostgresConnectionManager::new(name, SslMode::None) {
        Ok(m) => m,
        Err(_) => {
            return Err(Error::new(ErrorKind::BadParameter,
                                  "invalid database connection string"));
        }
    };
    r2d2::Pool::new(config, manager).map_err(|x| {
        Error::new(ErrorKind::General,
                   format!("cannot create tables: {}", x).as_str())
    })
}

pub fn init_table(db: Connection) -> Result<u64> {
    db.execute("CREATE TABLE IF NOT EXISTS proxy_servers (id SERIAL PRIMARY KEY, host VARCHAR \
                  NOT NULL, port INT NOT NULL, lag INT, vanilla BOOL, traceable BOOL, tags \
                  VARCHAR[], created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(), updated_at \
                  TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
                UNIQUE(host, port))",
                 &[])
        .map_err(|x| {
            Error::new(ErrorKind::General,
                       format!("cannot create tables: {}", x).as_str())
        })
}

pub fn add_proxy(conn: Connection, server: ProxyServer) -> Result<u64> {
    let host = server.host.to_string();
    let port = server.port as i32;
    let lag = match server.lag {
        Some(lag) => Some(lag.as_secs() as i32),
        _ => None,
    };
    match conn.execute("INSERT INTO proxy_servers(host, port, lag, vanilla, traceable, tags) \
                        VALUES($1, $2, $3, $4, $5, $6)",
                       &[&host, &port, &lag, &server.vanilla, &server.traceable, &server.tags]) {
        Ok(n) => {
            info!("server {} inserted.", server);
            Ok(n)
        }
        Err(error::Error::Db(ref error)) if error.code == error::SqlState::UniqueViolation => {
            // Try update
            let rows = db_try!(
                conn.execute("UPDATE proxy_servers SET lag=$3, vanilla=$4, \
                              traceable=$5, tags=$6, updated_at=NOW() \
                              WHERE host=$1 AND port=$2",
                    &[&host, &port, &lag,
                        &server.vanilla, &server.traceable,
                        &server.tags])
            );
            info!("server {} renewed.", server);
            Ok(rows)
        }
        Err(e) => Err(Error::new(ErrorKind::General, e.to_string().as_str())),
    }
}

pub fn disable_proxy(conn: Connection, server: ProxyServer) -> Result<u64> {
    let host = server.host.to_string();
    let port = server.port as i32;
    match conn.execute("UPDATE proxy_servers SET lag=NULL WHERE host=$1 AND port=$2",
                       &[&host, &port]) {
        Ok(n) => Ok(n),
        Err(e) => Err(Error::new(ErrorKind::General, e.to_string().as_str())),
    }
}

pub fn get_proxy_servers(db: Connection) -> Result<Vec<ProxyServer>> {
    let mut servers = Vec::new();
    let stmt = db_try!(db.prepare("SELECT host, port, lag, vanilla, traceable, tags, \
                                   created_at, updated_at
                                   FROM proxy_servers"));
    if let Ok(rows) = stmt.query(&[]) {
        for row in rows.into_iter() {
            let host: String = row.get(0);
            let port: i32 = row.get(1);
            let ip = match Ipv4Addr::from_str(host.as_str()) {
                Ok(ip) => ip,
                _ => continue,
            };
            servers.push(ProxyServer {
                host: ip,
                port: port as u16,
                lag: match row.get::<_, Option<i32>>(2) {
                    Some(x) => Some(Duration::new(x as u64, 0)),
                    _ => None,
                },
                vanilla: row.get(3),
                traceable: row.get(4),
                tags: row.get(5),
                created_at: row.get(6),
                updated_at: row.get(7),
            });
        }
    }
    Ok(servers)
}

pub fn search_proxy_servers(db: Connection,
                            max_lag: Option<i32>,
                            tags: Vec<&str>)
                            -> Result<Vec<ProxyServer>> {
    let mut servers = Vec::new();
    let stmt =
        db_try!(db.prepare("SELECT host, port, lag, vanilla, traceable, tags, created_at, updated_at \
                            FROM proxy_servers WHERE lag < $1 AND tags @> $2::VARCHAR[] \
                            ORDER BY updated_at, lag"));
    let lag = if let Some(x) = max_lag { x } else { 9999 };
    if let Ok(rows) = stmt.query(&[&lag, &tags]) {
        for row in rows.into_iter() {
            let host: String = row.get(0);
            let port: i32 = row.get(1);
            let ip = match Ipv4Addr::from_str(host.as_str()) {
                Ok(ip) => ip,
                _ => continue,
            };
            servers.push(ProxyServer {
                host: ip,
                port: port as u16,
                lag: match row.get::<_, Option<i32>>(2) {
                    Some(x) => Some(Duration::new(x as u64, 0)),
                    _ => None,
                },
                vanilla: row.get(3),
                traceable: row.get(4),
                tags: row.get(5),
                created_at: row.get(6),
                updated_at: row.get(7),
            });
        }
    }
    Ok(servers)
}

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
                             16 Sep, 2016

 */
use libsqlite3_sys as ffi;
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Error::SqliteFailure;
use std::net::Ipv4Addr;
use std::result;
use std::str::FromStr;

pub type Pool = r2d2::Pool<SqliteConnectionManager>;
type Connection = r2d2::PooledConnection<SqliteConnectionManager>;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: String,
}

impl Error {
    fn new(kind: ErrorKind, message: &str) -> Error {
        Error {kind: kind, message: message.to_string()}
    }
}

#[derive(Debug)]
pub enum ErrorKind {
    General
}

#[derive(Debug)]
pub struct ProxyServer {
    pub host: Ipv4Addr,
    pub port: u16,
    pub lag: Option<u16>,
    pub vanilla: Option<bool>,
    pub traceable: Option<bool>,

}

impl ProxyServer {
    pub fn new(host: &str, port: u16) -> ProxyServer {
        ProxyServer {
            host: Ipv4Addr::from_str(host).unwrap(),
            port: port,
            lag: None,
            vanilla: None,
            traceable: None,
        }
    }
}

type Result<T> = result::Result<T, Error>;

macro_rules! db_try {
    ( $expr:expr ) => (
            match $expr {
                Ok(val) => val,
                Err(e) => {
                    let error = Error::new(ErrorKind::General,
                                           format!("{}", e).as_str());
                    return Err(error);
                }
            }
    );
}

pub fn init_db(name: &str) -> Result<Pool> {
    let config = r2d2::Config::default();
    let manager = SqliteConnectionManager::new(name);
    r2d2::Pool::new(config, manager)
    .map_err(|x| Error::new(ErrorKind::General,
                            format!("cannot create tables: {}", x).as_str()))
}

pub fn init_table(db: Connection) -> Result<i32> {
    db.execute("CREATE TABLE IF NOT EXISTS \
                proxy_servers (id AUTO_INCREMENT PRIMARY KEY, \
                host VARCHAR(15), port INT, lag INT, \
                vanilla BOOL, traceable BOOL, UNIQUE(host, port))", &[])
    .map_err(|x| Error::new(ErrorKind::General,
                            format!("cannot create tables: {}", x).as_str()))
}

pub fn add_proxy(conn: Connection, server: ProxyServer) -> Result<i32> {
    let host = format!("{}", server.host);
    let port = server.port as i64;
    let lag = match server.lag {
        Some(lag) => Some(lag as i64),
        _ => None,
    };
    match conn.execute("INSERT INTO proxy_servers(host, port, lag, vanilla, traceable) \
                        VALUES($1, $2, $3, $4, $5)",
                       &[&host, &port, &lag, &server.vanilla, &server.traceable])
    {
        Ok(n) => {Ok(n)}
        Err(SqliteFailure(ffi::Error {
            code: ffi::ErrorCode::ConstraintViolation, .. }, Some(_))) =>
        {
            // Try update
            let rows = db_try!(
                conn.execute("UPDATE proxy_servers SET lag=$3, vanilla=$4, \
                              traceable=$5 \
                              WHERE host=$1 AND port=$2",
                             &[&host, &port, &lag,
                               &server.vanilla, &server.traceable])
            );
            Ok(rows)
        }
        Err(e) => { Err(Error::new(ErrorKind::General, format!("{}", e).as_str())) }
    }
}

pub fn get_proxy_servers(db: Connection) -> Result<Vec<ProxyServer>> {
    let mut stmt = db_try!(db.prepare("SELECT host, port, lag, vanilla, traceable \
                                       FROM proxy_servers"));
    let rows = db_try!(stmt.query_map(&[], |row| {
        let host: String = row.get(0);
        let port: i64 = row.get(1);
        let ip = match Ipv4Addr::from_str(host.as_str()) {
            Ok(ip) => ip,
            _ => return None,
        };
        Some(ProxyServer {
            host: ip,
            port: port as u16,
            lag: match row.get::<_, Option<i64>>(2) {
                Some(x) => Some(x as u16),
                _ => None
            },
            vanilla: row.get(3),
            traceable: row.get(3),
        })
    }));
    let mut servers = Vec::new();
    for server in rows {
        if let Ok(Some(server)) = server {
            servers.push(server)
        }
    }
    Ok(servers)
}

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
use r2d2;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Error::SqliteFailure;
use std::net;
use std::time::Duration;
use libsqlite3_sys as ffi;

type Pool = r2d2::Pool<SqliteConnectionManager>;
type Connection = r2d2::PooledConnection<SqliteConnectionManager>;

// XXX: add db_api custom error and remove unwraps

pub fn init_db(name: &str) -> Pool {
    let config = r2d2::Config::default();
    let manager = SqliteConnectionManager::new(name);
    r2d2::Pool::new(config, manager).unwrap()
}

pub fn init_table(db: Connection) {
    db.execute("CREATE TABLE IF NOT EXISTS \
                proxy_servers (id AUTO_INCREMENT PRIMARY KEY, \
                host VARCHAR(15), port INT, lag INT, UNIQUE(host, port))", &[])
    .expect("cannot create tables");
}

pub fn add_proxy(db: Connection, server: net::Ipv4Addr, port: u16, lag: Option<Duration>) {
    let lag = match lag {
        Some(x) => x.as_secs() as i64,
        None => 9999
    };
    let host = format!("{}", server);
    let port = port as i64;
    match db.execute("INSERT INTO proxy_servers(host, port, lag) \
                      VALUES($1, $2, $3)", &[&host, &port, &lag])
    {
        Ok(_) => {}
        Err(SqliteFailure(ffi::Error {
            code: ffi::ErrorCode::ConstraintViolation, .. }, Some(_))) =>
        {
            // Try update
            db.execute("UPDATE proxy_servers SET lag=$3 \
                        WHERE host=$1 AND port=$2", &[&host, &port, &lag])
            .unwrap();
        }
        Err(e) => { panic!(e); }
    }
}

pub fn get_proxy_servers(db: Connection) {
    db.execute("SELECT server, port FROM proxy_servers", &[]);
}

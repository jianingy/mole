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
19 Sep, 2016

 */
use clap::ArgMatches;
use nickel::{Nickel, QueryString};
use serde_json;
use serde_json::value::{ToJson, Value};

use db_api;

pub fn run_api(opts: ArgMatches) {
    let bind_address = opts.value_of("bind")
        .expect("no bind address found");
    let dbname = opts.value_of("database").unwrap().to_string();
    let db = db_api::init_db(&dbname).unwrap();
    db_api::init_table(db.get().unwrap()).unwrap();
    let mut server = Nickel::new();
    server.utilize(router! {
        get "/api/v1/servers" => |req| {
            let conn = db.get().unwrap();
            let lag = match req.query().get("lag") {
                Some(x) => match x.parse::<i32>() {
                    Ok(x) => x,
                    _ => 9999,
                },
                None => 9999
            };
            match db_api::search_proxy_servers(conn, lag) {
                Ok(servers) => {
                    let v = servers.iter().map(|x| x.to_json()).collect::<Vec<Value>>();
                    serde_json::to_string(&v).unwrap()
                },
                Err(e) => {
                    format!("error: {:?}", e)
                }
            }
        }
    });
    server.listen(bind_address);
}

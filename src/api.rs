// Jianing Yang <jianingy.yang@gmail.com> @ 22 Sep, 2016

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
                Some(x) => x.parse::<i32>().ok(),
                None => None
            };
            let tags = match req.query().get("tags") {
                Some(x) => x.split(",").collect::<Vec<&str>>(),
                None => Vec::new()
            };
            match db_api::search_proxy_servers(conn, lag, tags) {
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

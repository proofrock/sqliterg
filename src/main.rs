// Copyright (c) 2023-, Germano Rizzo <oss /AT/ germanorizzo /DOT/ it>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate eyre;

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    guard,
    web::{post, scope, Data},
    App, HttpServer, Scope,
};
use rusqlite::Connection;

pub mod auth;
mod backup;
pub mod commandline;
pub mod commons;
pub mod db_config;
mod logic;
mod macros;
pub mod main_config;
pub mod req_res;

use crate::{
    commons::{abort, resolve_tilde_opt},
    main_config::compose_db_map,
};

pub const CURRENT_PROTO_VERSION: u8 = 1;

pub static MUTEXES: OnceLock<HashMap<String, Mutex<Connection>>> = OnceLock::new();

fn get_sqlite_version() -> String {
    let conn: Connection = Connection::open_in_memory().unwrap();
    conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap()
}

// curl -X POST -H "Content-Type: application/json" -d '{"transaction":[{"statement":"DELETE FROM TBL"},{"query":"SELECT * FROM TBL"},{"statement":"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)","values":{"id":0,"val":"zero"}},{"statement":"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)","valuesBatch":[{"id":1,"val":"uno"},{"id":2,"val":"due"}]},{"query":"SELECT * FROM TBL WHERE ID=:id","values":{"id":1}},{"statement":"DELETE FROM TBL"}]}' http://localhost:12321/query
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(
        "{} v{}. based on SQLite v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        get_sqlite_version()
    );

    let cli = commandline::parse_cli();

    // side effect of compose_db_map: populate MUTEXES
    let db_map = match compose_db_map(&cli) {
        Ok(db_map) => db_map,
        Err(e) => abort(format!("{}", e.to_string())),
    };

    let dir = resolve_tilde_opt(&cli.serve_dir);

    let app_lambda = move || {
        let dir = dir.to_owned();
        let mut a = App::new();
        for (db_name, db_conf) in db_map.iter() {
            let scop: Scope = scope(format!("/{}", db_name.to_owned()).as_str())
                .app_data(Data::new(db_name.to_owned()))
                .app_data(Data::new(db_conf.to_owned()))
                .guard(guard::Header("content-type", "application/json"))
                .route("/exec", post().to(logic::handler))
                .route("/backup", post().to(backup::handler))
                .route("/macro/{macro_name}", post().to(macros::handler));
            match &db_conf.conf.cors_origin {
                Some(orig) => {
                    let mut cors = Cors::default().allowed_methods(vec!["POST"]);
                    if orig == "*" {
                        cors = cors.allow_any_origin();
                    } else {
                        cors = cors.allowed_origin(orig.as_str());
                    }
                    a = a.service(scop.wrap(cors))
                }
                None => a = a.service(scop),
            }
        }

        if dir.is_some() {
            a = a.service(Files::new("/", dir.unwrap()));
        };
        return a;
    };

    let bind_addr = format!("{}:{}", cli.bind_host, cli.port);
    println!("Listening on {}", &bind_addr);
    HttpServer::new(app_lambda).bind(bind_addr)?.run().await
}

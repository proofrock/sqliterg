// MIT License
//
// Copyright (c) 2023-, Germano Rizzo <oss /AT/ germanorizzo /DOT/ it>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate eyre;

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

use actix_files::Files;
use actix_web::{
    guard,
    web::{post, scope, Data},
    App, HttpServer,
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
        let dir = dir.clone();
        let mut a = App::new();
        for (db_name, db_conf) in db_map.iter() {
            let mut scope = scope(format!("/{}", db_name.clone()).as_str())
                .app_data(Data::new(db_name.clone()))
                .app_data(Data::new(db_conf.clone()))
                .guard(guard::Header("content-type", "application/json"))
                .route("/exec", post().to(logic::handler));
            if db_conf.conf.backup_endpoint.is_some() {
                scope = scope.route("/backup", post().to(backup::handler))
            }
            if db_conf.conf.macros_endpoint.is_some() {
                scope = scope.route("/macro/{macro_name}", post().to(macros::handler))
            }
            a = a.service(scope);
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

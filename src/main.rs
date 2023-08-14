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

use actix_files::Files;
use actix_web::{
    web::{self, Path},
    App, HttpServer, Responder,
};
use req_res::Response;
use rusqlite::Connection;
use std::{collections::HashMap, ops::DerefMut, panic, sync::OnceLock};

pub mod commandline;
pub mod commons;
pub mod db_config;
mod logic;
pub mod main_config;
pub mod req_res;

use crate::{
    logic::do_init,
    main_config::{compose_db_map, Db},
};

static DB_MAP: OnceLock<HashMap<String, Db>> = OnceLock::new();

async fn handle_query(query: web::Json<req_res::Request>, db_name: Path<String>) -> impl Responder {
    let map = DB_MAP.get().unwrap();
    let db_conf = map.get(db_name.as_str());
    match db_conf {
        Some(db_conf) => {
            let stored_statements = &db_conf.stored_statements;

            let db_lock = &db_conf.sqlite;
            let mut db_lock_guard = db_lock.lock().unwrap();
            let db = db_lock_guard.deref_mut();

            let result = logic::process(db, &query, stored_statements).unwrap();

            drop(db_lock_guard);

            result
        }
        None => Response {
            results: None,
            req_idx: Some(-1),
            message: Some(format!("Unknown database '{}'", db_name.as_str())),
        },
    }
}

fn get_sqlite_version() -> String {
    let conn: Connection = Connection::open_in_memory().unwrap();
    let version: String = conn
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap();
    let _ = conn.close();
    version
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

    let _ = DB_MAP.set(compose_db_map(&cli));

    match do_init() {
        Ok(()) => (),
        Err(e) => panic!("{}", e.to_string()),
    }

    let dir = cli.serve_dir.clone();

    let app_lambda = move || {
        let dir = dir.clone();
        let mut a = App::new().route("/db/{db_name}", web::post().to(handle_query));
        if dir.is_some() {
            a = a.service(Files::new("/", dir.unwrap()));
        };
        return a;
    };

    HttpServer::new(app_lambda)
        .bind(format!("{}:{}", cli.bind_host, cli.port))?
        .run()
        .await
}

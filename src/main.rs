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
use actix_web::{web::Data, App, HttpServer};
use rusqlite::Connection;
use std::panic;

pub mod auth;
pub mod commandline;
pub mod commons;
pub mod db_config;
mod logic;
pub mod main_config;
pub mod req_res;

use crate::{logic::handler, main_config::compose_db_map};

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

    let db_map = compose_db_map(&cli);
    let db_map = match db_map {
        Ok(db_map) => Data::new(db_map),
        Err(e) => panic!("{}", e.to_string()),
    };

    let dir = cli.serve_dir.clone();

    let app_lambda = move || {
        let dir = dir.clone();
        let mut a = App::new().app_data(db_map.clone()).service(handler);
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

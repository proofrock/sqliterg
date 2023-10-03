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
    ops::Deref,
    sync::{Mutex, OnceLock},
};

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{
    guard,
    web::{route, scope, Data},
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

use crate::{commandline::parse_cli, db_config::AuthMode, main_config::compose_db_map};

pub const CURRENT_PROTO_VERSION: u8 = 1;

pub static MUTEXES: OnceLock<HashMap<String, Mutex<Connection>>> = OnceLock::new();

fn get_sqlite_version() -> String {
    let conn: Connection = Connection::open_in_memory().unwrap();
    conn.query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!(
        "{} v{}. based on SQLite v{}\n",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        get_sqlite_version()
    );

    let cli = parse_cli();

    // side effect of compose_db_map: populate MUTEXES
    // aborts on error
    let db_map = compose_db_map(&cli);

    if let Some(sd) = &cli.serve_dir {
        println!("- serving directory: {}", sd);
        println!("  - with index file: {}", &cli.index_file);
    };

    let app_lambda = move || {
        let dir = cli.serve_dir.to_owned();
        let index_file = cli.index_file.to_owned();
        let mut a = App::new();
        for (db_name, db_conf) in db_map.iter() {
            let scop: Scope = scope(format!("/{}", db_name.to_owned()).deref())
                .app_data(Data::new(db_name.to_owned()))
                .app_data(Data::new(db_conf.to_owned()))
                .route(
                    "",
                    route()
                        .guard(guard::Post())
                        .guard(guard::Header("content-type", "application/json"))
                        .to(logic::handler),
                )
                .route(
                    "/macro/{macro_name}",
                    route()
                        .guard(guard::Any(guard::Get()).or(guard::Post()))
                        .to(macros::handler),
                )
                .route(
                    "/backup",
                    route()
                        .guard(guard::Any(guard::Get()).or(guard::Post()))
                        .to(backup::handler),
                );

            match &db_conf.conf.cors_origin {
                Some(orig) => {
                    let mut cors = Cors::default()
                        .allowed_methods(vec!["POST"])
                        .allowed_header("content-type");
                    if db_conf.conf.auth.is_some()
                        && matches!(
                            db_conf.conf.auth.as_ref().unwrap().mode,
                            AuthMode::HttpBasic
                        )
                    {
                        cors = cors.allowed_header("authorization");
                    }
                    if orig == "*" {
                        cors = cors.allow_any_origin();
                    } else {
                        cors = cors.allowed_origin(orig);
                    }
                    a = a.service(scop.wrap(cors))
                }
                None => a = a.service(scop),
            }
        }

        if let Some(dir) = dir {
            a = a.service(Files::new("/", dir).index_file(index_file));
        };

        a
    };

    let bind_addr = format!("{}:{}", cli.bind_host, cli.port);
    println!("- Listening on {}", &bind_addr);
    HttpServer::new(app_lambda).bind(bind_addr)?.run().await
}

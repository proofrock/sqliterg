#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate lazy_static;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use rusqlite::Connection;
use serde::Deserialize;
use std::{collections::HashMap, ops::Deref, sync::RwLock};

pub mod commandline;
pub mod db_config;
pub mod main_config;
mod logic;

use crate::main_config::{compose_db_map, Db};

lazy_static! {
    pub static ref GLOBAL_MAP: RwLock<HashMap<String, Db>> = RwLock::new(HashMap::new());
}

#[derive(Deserialize)]
struct Query {
    sql: String,
}

async fn handle_query(query: web::Json<Query>) -> impl Responder {
    let read_lock_guard = GLOBAL_MAP.read().unwrap();
    let map = read_lock_guard.deref();
    let db_conf = map.get("bubbu").unwrap();
    let db_lock = &db_conf.sqlite;
    let db_lock_guard = db_lock.lock().unwrap();
    let db = db_lock_guard.deref();

    let result = logic::query_first_field(db, query.sql.deref()).unwrap();
    // println!("Received SQL query: {}", query.sql);

    println!("Result: {}", result);

    drop(db_lock_guard);
    drop(read_lock_guard);

    HttpResponse::Ok().body(format!("Query received successfully: {}\n", result))
}

fn print_sqlite_version() {
    let conn: Connection = Connection::open("database.db").unwrap();
    let version: String = conn
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .unwrap();
    println!("SQLite version: {}", version);
}

// curl -X POST -H "Content-Type: application/json" -d '{"sql": "SELECT * FROM your_table;"}' http://localhost:12321/query
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    print_sqlite_version();

    let cli = commandline::parse_cli();

    compose_db_map(&cli);

    println!("{:#?}", GLOBAL_MAP.read().unwrap().get("bubbu"));

    HttpServer::new(|| App::new().route("/query", web::post().to(handle_query)))
        .bind(format!("{}:{}", (&cli).bind_host, (&cli).port))?
        .run()
        .await
}

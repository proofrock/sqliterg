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

use std::fs::remove_file;
use std::sync::Mutex;
use std::{collections::HashMap, path::Path};

use rusqlite::Connection;

use crate::backup::{bootstrap_backup, periodic_backup};
use crate::commandline::AppConfig;
use crate::commons::{
    abort, assert, file_exists, if_abort_rusqlite, resolve_tilde, split_on_first_column,
};
use crate::db_config::{parse_dbconf, DbConfig, Macro};
use crate::macros::{bootstrap_db_macros, parse_macros, periodic_macro};
use crate::MUTEXES;

#[derive(Debug, Clone)]
pub struct Db {
    pub is_mem: bool,
    pub path: String,
    pub conf: DbConfig,

    // calculated
    pub stored_statements: HashMap<String, String>,
    pub macros: HashMap<String, Macro>,
}

fn to_base_name(path: &String) -> String {
    let path = Path::new(&path);
    path.file_stem().unwrap().to_str().unwrap().to_string()
}

fn to_yaml_path(path: &String) -> String {
    let path = Path::new(&path);
    let file_stem = path.file_stem().unwrap().to_str().unwrap();
    let yaml_file_name = format!("{file_stem}.yaml");
    let yaml_path = path.with_file_name(yaml_file_name);
    yaml_path.to_str().unwrap().to_string()
}

fn compose_single_db(
    yaml: &String,
    conn_string: &String,
    db_name: &String,
    db_path: &String, // simple name if in-mem
    is_new_db: bool,
    is_mem: bool,
) -> (Db, Connection) {
    let mut dbconf = if yaml == "" || !file_exists(yaml) {
        println!("YAML file for db ({}) not found: assuming defaults", yaml);
        DbConfig::default()
    } else {
        match parse_dbconf(yaml) {
            Ok(dbc) => dbc,
            Err(e) => abort(format!("Parsing YAML file {}: {}", yaml, e.to_string())),
        }
    };

    let stored_statements = dbconf
        .to_owned()
        .stored_statements
        .map(|ss| {
            ss.iter()
                .map(|el| (el.id.to_owned(), el.sql.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    parse_macros(&mut dbconf, &stored_statements);

    let mut conn = if_abort_rusqlite(Connection::open(conn_string));

    let res = bootstrap_db_macros(is_new_db, &dbconf, db_name, &mut conn);
    if res.is_err() {
        let _ = conn.close();
        if !is_mem && is_new_db {
            let _ = remove_file(Path::new(db_path));
        }
        abort(res.err().unwrap().to_string());
    }

    let macros: HashMap<String, Macro> = dbconf
        .to_owned()
        .macros
        .map(|mv: Vec<Macro>| {
            mv.iter()
                .map(|el| (el.id.to_owned(), el.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    for macr in macros.values() {
        periodic_macro(macr.to_owned(), db_name.to_owned());
    }

    bootstrap_backup(is_new_db, &dbconf, db_name, db_path, &mut conn);

    periodic_backup(
        dbconf.to_owned(),
        db_name.to_owned(),
        conn_string.to_owned(),
    );

    if dbconf.read_only {
        if_abort_rusqlite(conn.execute("PRAGMA query_only = true", []));
    }

    if !dbconf.disable_wal_mode {
        if_abort_rusqlite(conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(())));
    }

    let db_conf = Db {
        is_mem,
        path: conn_string.to_owned(),
        conf: dbconf,
        stored_statements,
        macros,
    };
    (db_conf, conn)
}

fn check_db_name(db_name: &String, db_map: &HashMap<String, Db>) {
    assert(
        !db_map.contains_key(db_name),
        format!("database '{}' already defined", db_name),
    );
}

pub fn compose_db_map(cl: &AppConfig) -> HashMap<String, Db> {
    let mut db_map = HashMap::new();
    let mut mutexes = HashMap::new();
    for db_path in &cl.db {
        let db_path = resolve_tilde(db_path);
        let db_name = to_base_name(&db_path);
        check_db_name(&db_name, &db_map);

        let yaml = to_yaml_path(&db_path);
        let is_new_db = !file_exists(&db_path);

        let (db_cfg, conn) =
            compose_single_db(&yaml, &db_path, &db_name, &db_path, is_new_db, false);

        db_map.insert(db_name.to_owned(), db_cfg);
        mutexes.insert(db_name.to_owned(), Mutex::new(conn));
    }
    for db in &cl.mem_db {
        let (db_name, yaml) = split_on_first_column(db);
        check_db_name(&db_name, &db_map);

        let yaml = resolve_tilde(&yaml);
        let conn_string = format!("file:{}?mode=memory", db_name);

        let (db_cfg, conn) = compose_single_db(&yaml, &conn_string, &db_name, &db_name, true, true);

        db_map.insert(db_name.to_owned(), db_cfg);
        mutexes.insert(db_name.to_owned(), Mutex::new(conn));
    }
    let _ = MUTEXES.set(mutexes);
    db_map
}

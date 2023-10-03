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
    abort, assert, file_exists, if_abort_rusqlite, is_dir, is_file_in_directory, resolve_tilde,
    split_on_first_double_colon,
};
use crate::db_config::{parse_dbconf, DbConfig, Macro};
use crate::macros::{bootstrap_db_macros, count_macros, periodic_macro, resolve_macros};
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

fn split_path(path: &str) -> (String, String, String) {
    // returns (db_path, yaml, db_name)
    let (mut db_path, mut yaml) = split_on_first_double_colon(path);
    db_path = resolve_tilde(&db_path);
    let path = Path::new(&db_path);
    if yaml.is_empty() {
        let file_stem = path.file_stem().unwrap().to_str().unwrap();
        let yaml_file_name = format!("{file_stem}.yaml");
        let yaml_path = path.with_file_name(yaml_file_name);
        yaml = yaml_path.to_str().unwrap().to_string();
    }
    let yaml = resolve_tilde(&yaml);

    let db_name = path.file_stem().unwrap().to_str().unwrap().to_string();

    (db_path, yaml, db_name)
}

fn compose_single_db(
    yaml: &String,
    conn_string: &String,
    db_name: &String,
    db_path: &String, // simple name if in-mem
    is_new_db: bool,
    is_mem: bool,
) -> (Db, Connection) {
    println!("- Database '{}'", db_name);

    if is_mem {
        println!("  - in-memory database");
    } else {
        println!("  - from file '{}'", db_path);
        if is_new_db {
            println!("    - file not present, it will be created");
        }
    }

    let mut dbconf = if yaml.is_empty() || !file_exists(yaml) {
        println!("  - companion file not found: assuming defaults");
        DbConfig::default()
    } else {
        println!("  - parsing companion file '{}'", yaml);
        parse_dbconf(yaml).unwrap_or_else(|e| abort(format!("parsing YAML {}: {}", yaml, e)))
    };

    if let Some(orig) = &dbconf.cors_origin {
        println!("  - allowed CORS origin: {}", orig);
    }

    if let Some(b) = &mut dbconf.backup {
        assert(
            b.num_files > 0,
            "backup: num_files must be 1 or more".to_string(),
        );
        let bd = resolve_tilde(&b.backup_dir);
        assert(
            is_dir(&bd),
            format!("backup directory does not exist: {}", bd),
        );
        b.backup_dir = bd;
    }

    if let Some(a) = &dbconf.auth {
        assert(
            a.by_credentials.is_none() != a.by_query.is_none(),
            "auth: exactly one among by_credentials and by_query must be specified".to_string(),
        );
        if let Some(vc) = &a.by_credentials {
            for c in vc {
                assert(
                    c.password.is_some() || c.hashed_password.is_some(),
                    format!(
                        "auth: user '{}': password or hashedPassword must be specified",
                        &c.user
                    ),
                );
            }
        }
        println!("  - authentication set up");
    }

    let stored_statements: HashMap<String, String> = dbconf
        .to_owned()
        .stored_statements
        .map(|ss| {
            ss.iter()
                .map(|el| (el.id.to_owned(), el.sql.to_owned()))
                .collect()
        })
        .unwrap_or_default();

    if !stored_statements.is_empty() {
        println!(
            "  - {} stored statements configured",
            stored_statements.len()
        );
        if dbconf.use_only_stored_statements {
            println!("    - allowing only stored statements for requests")
        }
    }

    let macros: HashMap<String, Macro> = resolve_macros(&mut dbconf, &stored_statements);

    for macr in macros.values() {
        assert(
            !macr.statements.is_empty(),
            format!("Macro '{}' does not have any statement", macr.id),
        );
    }

    if !macros.is_empty() {
        println!("  - {} macro(s) configured", macros.len());
        let count = count_macros(macros.to_owned());
        if count[0] > 0 {
            println!("    - {} applied on database creation", count[0]);
        }
        if count[1] > 0 {
            println!("    - {} applied on server startup", count[1]);
        }
        if count[2] > 0 {
            println!("    - {} applied periodically", count[2]);
        }
        if count[3] > 0 {
            println!("    - {} callable via web service", count[3]);
        }
    }

    let mut conn = if_abort_rusqlite(Connection::open(conn_string));

    let res = bootstrap_db_macros(is_new_db, &dbconf, db_name, &mut conn);
    if res.is_err() {
        let _ = conn.close();
        if !is_mem && is_new_db {
            let _ = remove_file(Path::new(db_path));
        }
        abort(res.err().unwrap().to_string());
    }

    for macr in macros.values() {
        periodic_macro(macr.to_owned(), db_name.to_owned());
    }

    if let Some(backup) = dbconf.to_owned().backup {
        if !is_mem {
            assert(
                !is_file_in_directory(db_path, &backup.backup_dir),
                format!(
                    "Backup config for '{}': backup dir cannot be the same as db file dir",
                    db_name
                ),
            );
        }

        bootstrap_backup(is_new_db, &backup, db_name, db_path, &conn);

        periodic_backup(&backup, db_name.to_owned(), conn_string.to_owned());
    }

    if dbconf.read_only {
        if_abort_rusqlite(conn.execute("PRAGMA query_only = true", []));
        println!("  - read-only");
    }

    let jm = dbconf.journal_mode.to_owned().unwrap_or("WAL".to_string());
    if_abort_rusqlite(conn.query_row(&format!("PRAGMA journal_mode = {}", jm), [], |_| Ok(())));
    println!("  - journal mode: {}", jm);

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
        let (db_path, yaml, db_name) = split_path(db_path);
        check_db_name(&db_name, &db_map);

        let is_new_db = !file_exists(&db_path);

        let (db_cfg, conn) =
            compose_single_db(&yaml, &db_path, &db_name, &db_path, is_new_db, false);

        db_map.insert(db_name.to_owned(), db_cfg);
        mutexes.insert(db_name.to_owned(), Mutex::new(conn));
    }
    for db in &cl.mem_db {
        let (db_name, yaml) = split_on_first_double_colon(db);
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

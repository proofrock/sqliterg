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

use eyre::Result;
use rusqlite::Connection;

use crate::backup::do_backup;
use crate::commandline::AppConfig;
use crate::commons::{abort, file_exists, resolve_tilde, split_on_first_column};
use crate::db_config::{parse_dbconf, DbConfig};
use crate::macros::{exec_init_startup_macros, parse_macros, parse_stored_statements};
use crate::MUTEXES;

#[derive(Debug, Clone)]
pub struct Db {
    pub path: String,
    pub conf: DbConfig,

    // calculated
    pub stored_statements: HashMap<String, String>,
    pub macros: HashMap<String, Vec<String>>,
}

fn to_base_name(path: &String) -> String {
    let path = Path::new(&path);
    path.file_stem().unwrap().to_str().unwrap().to_string()
}

fn to_yaml_path(path: &String) -> String {
    let path = Path::new(&path);
    let file_stem: &str = path.file_stem().unwrap().to_str().unwrap();
    let yaml_file_name = format!("{file_stem}.yaml");
    let yaml_path = path.with_file_name(yaml_file_name);
    yaml_path.to_str().unwrap().to_string()
}

pub fn compose_db_map(cl: &AppConfig) -> Result<HashMap<String, Db>> {
    let mut db_map = HashMap::new();
    let mut mutexes = HashMap::new();
    for db in &cl.db {
        let db = resolve_tilde(db);

        let yaml = to_yaml_path(&db);
        let dbconf = if !file_exists(&yaml) {
            println!("YAML file for db ({}) not found: assuming defaults", &yaml);
            DbConfig::default()
        } else {
            match parse_dbconf(&yaml) {
                Ok(dbc) => dbc,
                Err(e) => abort(format!("Parsing YAML file {}: {}", &yaml, e.to_string())),
            }
        };

        let is_new_db = !file_exists(&db);

        let stored_statements = parse_stored_statements(&dbconf);

        let macros_def = parse_macros(&dbconf, &stored_statements)?;

        let mut conn = Connection::open(&db)?;

        let res = exec_init_startup_macros(
            is_new_db,
            dbconf.init_macros.clone(),
            dbconf.startup_macros.clone(),
            &macros_def,
            &mut conn,
        );
        if res.is_err() {
            let _ = conn.close();
            if is_new_db {
                let _ = remove_file(Path::new(&db));
            }
            return Result::Err(res.err().unwrap());
        }

        match &dbconf.backup {
            Some(bkp) => {
                if bkp.at_startup {
                    let res = do_backup(&bkp, &db, &conn);
                    if !res.success {
                        eprintln!("Cannot perform backup: {}", res.message.unwrap());
                    }
                }
            }
            None => (),
        }

        if dbconf.read_only {
            conn.execute("PRAGMA query_only = true", [])?;
        }

        if !dbconf.disable_wal_mode {
            conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        }

        let db_cfg = Db {
            path: db.clone(),
            conf: dbconf,
            stored_statements,
            macros: macros_def,
        };

        let db_name = to_base_name(&db);
        db_map.insert(db_name.clone(), db_cfg);
        mutexes.insert(db_name.clone(), Mutex::new(conn));
    }
    for db in &cl.mem_db {
        let (db_name, yaml) = split_on_first_column(db);
        let yaml = resolve_tilde(&yaml);

        let dbconf = if yaml == "" || !file_exists(&yaml) {
            println!("YAML file for mem db not specified or not found: assuming defaults",);
            DbConfig::default()
        } else {
            match parse_dbconf(&yaml) {
                Ok(dbc) => dbc,
                Err(e) => abort(format!("Parsing YAML file {}: {}", &yaml, e.to_string())),
            }
        };

        let stored_statements = parse_stored_statements(&dbconf);

        let macros_def = parse_macros(&dbconf, &stored_statements)?;

        let mut conn = Connection::open(format! {"file:{}?mode=memory", db_name})?;

        let res = exec_init_startup_macros(
            true, // in-mem db is always new
            dbconf.init_macros.clone(),
            dbconf.startup_macros.clone(),
            &macros_def,
            &mut conn,
        );
        if res.is_err() {
            let _ = conn.close();
            return Result::Err(res.err().unwrap());
        }

        match &dbconf.backup {
            Some(bkp) => {
                if bkp.at_startup {
                    let res = do_backup(&bkp, &format!("{}.db", db_name), &conn);
                    if !res.success {
                        eprintln!("Cannot perform backup: {}", res.message.unwrap());
                    }
                }
            }
            None => (),
        }

        if dbconf.read_only {
            conn.execute("PRAGMA query_only = true", [])?;
        }

        if !dbconf.disable_wal_mode {
            conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        }

        let db_cfg = Db {
            path: db.clone(),
            conf: dbconf,
            stored_statements,
            macros: macros_def,
        };

        let db_name = to_base_name(&db);
        db_map.insert(db_name.clone(), db_cfg);
        mutexes.insert(db_name.clone(), Mutex::new(conn));
    }
    match MUTEXES.set(mutexes) {
        Ok(_) => Ok(db_map),
        Err(_) => Result::Err(eyre!("Error setting mutexes".to_string())),
    }
}

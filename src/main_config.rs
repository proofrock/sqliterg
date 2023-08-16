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

use std::{collections::HashMap, path::Path, sync::Mutex};

use rusqlite::Connection;
use eyre::Result;

use crate::commandline::AppConfig;
use crate::commons::file_exists;
use crate::db_config::{parse_dbconf, DbConfig};

#[derive(Debug)]
pub struct Db {
    pub path: String,
    pub conf: DbConfig,

    // calculated
    pub mutex: Mutex<()>,
    pub stored_statements: HashMap<String, String>,
}

fn to_base_name(path: &String) -> String {
    let path = Path::new(&path);
    String::from(path.file_stem().unwrap().to_str().unwrap())
}

fn to_yaml_path(path: &String) -> String {
    let path = Path::new(&path);
    let file_stem: &str = path.file_stem().unwrap().to_str().unwrap();
    let yaml_file_name = format!("{file_stem}.yaml");
    let yaml_path = path.with_file_name(yaml_file_name);
    String::from(yaml_path.to_str().unwrap())
}

pub fn compose_db_map(cl: &AppConfig) -> Result<HashMap<String, Db>> {
    let mut db_map = HashMap::new();
    for db in &cl.db {
        let dbconf = parse_dbconf(to_yaml_path(&db)).unwrap();

        if !file_exists(&db) {
            match &dbconf.init_statements {
                Some(vec) => {
                    let mut conn = Connection::open(&db).unwrap();
    
                    let tx = conn.transaction()?;
            
                    for sql in vec.iter() {
                        tx.execute(sql, [])?;
                    }
            
                    tx.commit()?; // TODO rollback on error is implied?
                }
                None => (),
            }
        }

        let mut stored_statements = HashMap::new();
        match &dbconf.stored_statements {
            Some(ss) => {
                for el in ss.iter() {
                    stored_statements.insert(el.id.clone(), el.sql.clone());
                }
            },
            None => (),
        }

        let db_cfg = Db {
            path: db.clone(),
            conf: dbconf,
            mutex: Mutex::new(()),
            stored_statements,
        };

        db_map.insert(to_base_name(&db), db_cfg);
    }
    Ok(db_map)
}

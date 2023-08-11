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

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::commandline::AppConfig;
use crate::db_config::parse_dbconf;
use crate::db_config::DbConfig;
use crate::GLOBAL_MAP;

#[derive(Debug)]
pub struct Db {
    pub path: String,
    pub conf: DbConfig,
    pub sqlite: Mutex<Connection>,
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

pub fn compose_db_map(cl: &AppConfig) {
    for db in &cl.db {
        let conn = Connection::open(&db).unwrap();

        let db_cfg = Db {
            path: db.clone(),
            conf: parse_dbconf(to_yaml_path(&db)).unwrap(),
            sqlite: Mutex::new(conn),
        };

        GLOBAL_MAP
            .write()
            .unwrap()
            .insert(to_base_name(&db), db_cfg);
    }
}

use std::path::Path;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::GLOBAL_MAP;
use crate::commandline::AppConfig;
use crate::db_config::DbConfig;
use crate::db_config::parse_dbconf;

#[derive(Debug)]
pub struct Db {
    pub path: String,
    pub conf: DbConfig,
    pub sqlite: Mutex<Connection>
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
            sqlite: Mutex::new(conn)
        };

        GLOBAL_MAP.write().unwrap().insert(
            to_base_name(&db),
            db_cfg,
        );
    }
}

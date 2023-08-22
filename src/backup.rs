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

use std::{ops::DerefMut, path::Path as SysPath, time::Duration};

use actix_web::{
    rt::{
        spawn,
        time::{interval_at, Instant},
    },
    web, Responder,
};
use rusqlite::Connection;

use crate::{
    auth::process_creds,
    commons::{abort, delete_old_files, file_exists, resolve_tilde},
    db_config::{Backup, DbConfig},
    main_config::Db,
    req_res::{Response, Token},
    MUTEXES,
};

fn gen_bkp_file(directory: &str, filepath: &str) -> String {
    let path = SysPath::new(directory);
    let fpath = SysPath::new(filepath);
    let base_name = fpath.file_stem().unwrap().to_str().unwrap();
    let extension = fpath.extension();
    let intermission = crate::commons::now();

    let new_file_name = match extension {
        Some(e) => format!("{}_{}.{}", base_name, intermission, e.to_str().unwrap()),
        None => format!("{}_{}", base_name, intermission),
    };
    let new_file_path = path.join(new_file_name);

    new_file_path.into_os_string().into_string().unwrap()
}

fn do_backup(bkp: &Backup, db_path: &String, conn: &Connection) -> Response {
    let bkp_dir = resolve_tilde(&bkp.backup_dir);

    if !file_exists(&bkp_dir) {
        return Response::new_err(404, -1, format!("Backup dir '{}' not found", bkp_dir));
    }

    let file = gen_bkp_file(&bkp_dir, db_path);
    if file_exists(&file) {
        Response::new_err(409, -1, format!("File '{}' already exists", file))
    } else {
        match conn.execute("VACUUM INTO ?1", [&file]) {
            Ok(_) => match delete_old_files(&file, bkp.num_files) {
                Ok(_) => Response::new_ok(vec![]),
                Err(e) => Response::new_err(
                    500,
                    -1,
                    format!(
                        "Database backed up but error in deleting old files: {}",
                        e.to_string()
                    ),
                ),
            },
            Err(e) => Response::new_err(500, -1, e.to_string()),
        }
    }
}

pub async fn handler(
    db_conf: web::Data<Db>,
    db_name: web::Data<String>,
    token: web::Query<Token>,
) -> impl Responder {
    let db_name = db_name.to_string();
    match &db_conf.conf.backup {
        Some(bkp) => match &bkp.execution.web_service {
            Some(bkp_ws) => {
                if !process_creds(&token.token, &bkp_ws.auth_token, &bkp_ws.hashed_auth_token) {
                    return Response::new_err(
                        401,
                        -1,
                        format!("In database '{}', backup: token mismatch", db_name),
                    );
                }

                let db_lock = MUTEXES.get().unwrap().get(&db_name).unwrap();
                let mut db_lock_guard = db_lock.lock().unwrap();
                let conn = db_lock_guard.deref_mut();

                do_backup(bkp, &db_conf.path, &conn)
            }
            None => Response::new_err(
                404,
                -1,
                format!(
                    "Database '{}' doesn't have a backup.execution.webService node",
                    db_name
                ),
            ),
        },
        None => Response::new_err(
            404,
            -1,
            format!("Database '{}' doesn't have a backup node", db_name),
        ),
    }
}

pub fn bootstrap_backup(
    is_new_db: bool,
    db_conf: &DbConfig,
    db_name: &String,
    db_path: &String,
    conn: &Connection,
) {
    match &db_conf.backup {
        Some(bkp) => {
            let bex = &bkp.execution;
            if bex.on_startup || (is_new_db && bex.on_create) {
                let res = do_backup(bkp, db_path, conn);
                if !res.success {
                    abort(format!(
                        "Backup of database '{}': {}",
                        db_name,
                        res.message.unwrap()
                    ));
                }
            }
        }
        None => (),
    }
}

pub fn periodic_backup(db_conf: DbConfig, db_name: String, db_path: String) {
    match db_conf.backup {
        Some(bkp) => {
            let period = bkp.execution.period;
            let bkp_dir = bkp.backup_dir;
            if period > 0 {
                let period = period as u64 * 60;
                spawn(async move {
                    let p = Duration::from_secs(period);
                    let first_start = Instant::now().checked_add(p).unwrap();
                    let mut interval = interval_at(first_start, p);

                    interval.tick().await; // skip first execution

                    loop {
                        interval.tick().await;

                        let bkp_dir = resolve_tilde(&bkp_dir);

                        if !file_exists(&bkp_dir) {
                            eprintln!("Backup dir '{}' not found", bkp_dir);
                            return;
                        }

                        let file = gen_bkp_file(&bkp_dir, &db_path);
                        if file_exists(&file) {
                            eprintln!("File '{}' already exists", file);
                        } else {
                            let db_lock = MUTEXES.get().unwrap().get(&db_name).unwrap();
                            let mut db_lock_guard = db_lock.lock().unwrap();
                            let conn = db_lock_guard.deref_mut();

                            match conn.execute("VACUUM INTO ?1", [&file]) {
                                Ok(_) => match delete_old_files(&file, bkp.num_files) {
                                    Ok(_) => (),
                                    Err(e) => eprintln!(
                                        "Database backed up but error in deleting old files: {}",
                                        e.to_string()
                                    ),
                                },
                                Err(e) => eprintln!("Backing up '{}': {}", db_name, e.to_string()),
                            }
                        }
                    }
                });
            }
        }
        None => (),
    };
}

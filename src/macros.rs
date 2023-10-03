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

use std::{collections::HashMap, ops::DerefMut, time::Duration};

use actix_web::{
    rt::{
        spawn,
        time::{interval_at, sleep, Instant},
    },
    web::{self, Path},
    Responder,
};
use eyre::Result;
use rusqlite::Connection;

use crate::{
    auth::process_creds,
    commons::{check_stored_stmt, if_abort_eyre},
    db_config::{DbConfig, Macro},
    main_config::Db,
    req_res::{Response, ResponseItem, Token},
    MUTEXES,
};

/// Parses the macro list and substitutes the references to stored statements with the target sql
pub fn resolve_macros(
    dbconf: &mut DbConfig,
    stored_statements: &HashMap<String, String>,
) -> HashMap<String, Macro> {
    let mut ret: HashMap<String, Macro> = HashMap::new();
    if let Some(ms) = &mut dbconf.macros {
        for macr in ms {
            let mut statements: Vec<String> = vec![];
            #[allow(clippy::unnecessary_to_owned)]
            for statement in macr.statements.to_owned() {
                let statement =
                    if_abort_eyre(check_stored_stmt(&statement, stored_statements, false));
                statements.push(statement.to_owned());
            }
            macr.statements = statements;
            ret.insert(macr.id.to_owned(), macr.to_owned());
        }
    }
    ret
}

fn exec_macro_single_notrx(macr: &Macro, conn: &mut Connection) -> Response {
    let mut ret = vec![];
    for (i, statement) in macr.statements.iter().enumerate() {
        let changed_rows = conn.execute(statement, []);
        match changed_rows {
            Ok(cr) => {
                ret.push(ResponseItem {
                    success: true,
                    error: None,
                    result_set: None,
                    rows_updated: Some(cr),
                    rows_updated_batch: None,
                });
            }
            Err(e) => {
                return Response::new_err(500, i as isize, e.to_string());
            }
        }
    }

    Response::new_ok(ret)
}

fn exec_macro_single(macr: &Macro, conn: &mut Connection) -> Response {
    if macr.disable_transaction {
        return exec_macro_single_notrx(macr, conn);
    }

    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(_) => {
            return Response::new_err(
                500,
                -1,
                format!("Transaction open failed for macro '{}'", macr.id),
            )
        }
    };

    let mut ret = vec![];
    for (i, statement) in macr.statements.iter().enumerate() {
        let changed_rows = tx.execute(statement, []);
        match changed_rows {
            Ok(cr) => {
                ret.push(ResponseItem {
                    success: true,
                    error: None,
                    result_set: None,
                    rows_updated: Some(cr),
                    rows_updated_batch: None,
                });
            }
            Err(e) => {
                let _ = tx.rollback();
                return Response::new_err(500, i as isize, e.to_string());
            }
        }
    }

    match tx.commit() {
        Ok(_) => Response::new_ok(ret),
        Err(_) => Response::new_err(500, -1, format!("Commit failed for macro '{}'", macr.id)),
    }
}

pub fn bootstrap_db_macros(
    is_new_db: bool,
    db_conf: &DbConfig,
    db_name: &String,
    conn: &mut Connection,
) -> Result<()> {
    match &db_conf.macros {
        Some(macros) => {
            for macr in macros {
                if macr.execution.on_startup || (is_new_db && macr.execution.on_create) {
                    let res = exec_macro_single(macr, conn);
                    if !res.success {
                        return Result::Err(eyre!(
                            "In macro '{}' of db '{}', index {}: {}",
                            macr.id,
                            db_name,
                            res.req_idx.unwrap_or(-1),
                            res.message.unwrap_or("unknown error".to_string())
                        ));
                    };
                }
            }
        }
        None => (),
    }

    Result::Ok(())
}

pub async fn handler(
    db_conf: web::Data<Db>,
    db_name: web::Data<String>,
    macro_name: Path<String>,
    token: web::Query<Token>,
) -> impl Responder {
    let db_name = db_name.to_string();
    let macro_name = macro_name.to_string();

    match db_conf.macros.get(&macro_name) {
        Some(macr) => match &macr.execution.web_service {
            Some(mex_ws) => {
                if !process_creds(&token.token, &mex_ws.auth_token, &mex_ws.hashed_auth_token) {
                    sleep(Duration::from_millis(1000)).await;

                    return Response::new_err(
                        mex_ws.auth_error_code,
                        -1,
                        format!(
                            "In database '{}', macro '{}': token mismatch",
                            db_name, macro_name
                        ),
                    );
                }

                let db_lock = MUTEXES.get().unwrap().get(&db_name).unwrap();
                let mut db_lock_guard = db_lock.lock().unwrap();
                let conn = db_lock_guard.deref_mut();

                exec_macro_single(macr, conn)
            }
            None => Response::new_err(
                404,
                -1,
                format!(
                    "In database '{}', macro '{}' doesn't have an execution node",
                    db_name, macro_name
                ),
            ),
        },
        None => Response::new_err(
            404,
            -1,
            format!(
                "Database '{}' doesn't have a macro named '{}'",
                db_name, macro_name
            ),
        ),
    }
}

pub fn periodic_macro(macr: Macro, db_name: String) {
    if macr.execution.period > 0 {
        spawn(async move {
            let p = Duration::from_secs(macr.execution.period as u64 * 60);
            let first_start = Instant::now().checked_add(p).unwrap();
            let mut interval = interval_at(first_start, p);

            loop {
                interval.tick().await; // skip first execution

                let db_lock = MUTEXES.get().unwrap().get(&db_name).unwrap();
                let mut db_lock_guard = db_lock.lock().unwrap();
                let conn = db_lock_guard.deref_mut();

                let tx = match conn.transaction() {
                    Ok(tx) => tx,
                    Err(_) => {
                        eprintln!(
                            "Transaction open failed for db '{}', macro '{}'",
                            db_name, macr.id,
                        );
                        return;
                    }
                };

                for (i, statement) in macr.statements.iter().enumerate() {
                    match tx.execute(statement, []) {
                        Ok(_) => (),
                        Err(e) => {
                            let _ = tx.rollback();
                            eprintln!(
                                "In macro '{}' of db '{}', index {}: {}",
                                macr.id, db_name, i, e
                            );
                            return;
                        }
                    }
                }

                match tx.commit() {
                    Ok(_) => println!("Macro '{}' executed for db '{}'", macr.id, db_name),
                    Err(e) => {
                        eprintln!(
                            "Commit failed for startup macros in db '{}': {}",
                            db_name, e
                        );
                    }
                }
            }
        });
    }
}

pub fn count_macros(macros: HashMap<String, Macro>) -> [usize; 4] {
    // return [num_on_create, num_on_startup, num_periodic, num_exposed_via_webservice]
    let mut ret = [0, 0, 0, 0];
    for macr in macros.values() {
        let e = &macr.execution;
        if e.on_create {
            ret[0] += 1;
        }
        if e.on_startup {
            ret[1] += 1;
        }
        if e.period > 0 {
            ret[2] += 1;
        }
        if e.web_service.is_some() {
            ret[3] += 1;
        }
    }
    ret
}

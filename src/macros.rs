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

use std::{collections::HashMap, ops::DerefMut};

use actix_web::{
    web::{self, Path},
    Responder,
};
use eyre::Result;
use rusqlite::Connection;

use crate::{
    auth::process_creds,
    commons::check_stored_stmt,
    db_config::{DbConfig, Macro},
    main_config::Db,
    req_res::{Response, ResponseItem, Token},
    MUTEXES,
};

pub fn parse_stored_statements(dbconf: &DbConfig) -> HashMap<String, String> {
    let mut stored_statements = HashMap::new();
    match &dbconf.stored_statements {
        Some(ss) => {
            for el in ss {
                stored_statements.insert(el.id.clone(), el.sql.clone());
            }
        }
        None => (),
    }
    stored_statements
}

pub fn parse_macros(
    dbconf: &DbConfig,
    stored_statements: &HashMap<String, String>,
) -> Result<HashMap<String, Macro>> {
    let mut macros = HashMap::new();
    match &dbconf.macros {
        Some(ms) => {
            for el in ms {
                let mut statements: Vec<String> = vec![];
                for statement in el.statements.clone() {
                    let statement = check_stored_stmt(&statement, stored_statements, false)?;
                    statements.push(statement.clone());
                }
                macros.insert(
                    el.id.clone(),
                    Macro {
                        id: el.id.clone(),
                        statements: statements,
                        execution: el.execution.clone(),
                    },
                );
            }
        }
        None => (),
    }
    Ok(macros)
}

fn exec_macro_single(macr: &Macro, conn: &mut Connection) -> Response {
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
    macros_resolved_stored_statements: &HashMap<String, Macro>,
    conn: &mut Connection,
) -> Result<()> {
    match &db_conf.macros {
        Some(macros) => {
            let tx = match conn.transaction() {
                Ok(tx) => tx,
                Err(_) => return Result::Err(eyre!("Transaction open failed")),
            };

            for macr in macros {
                let macr = macros_resolved_stored_statements.get(&macr.id).unwrap();
                match &macr.execution {
                    Some(mex) => {
                        if mex.on_startup || (is_new_db && mex.on_create) {
                            for (i, statement) in macr.statements.iter().enumerate() {
                                let changed_rows = tx.execute(statement, []);
                                match changed_rows {
                                    Ok(_) => (),
                                    Err(e) => {
                                        let _ = tx.rollback();
                                        return Result::Err(eyre!(
                                            "In macro '{}' of db '{}', index {}: {}",
                                            macr.id,
                                            db_name,
                                            i,
                                            e
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    None => (),
                }
            }

            match tx.commit() {
                Ok(_) => (),
                Err(e) => {
                    return Result::Err(eyre!(
                        "Commit failed for startup macros in db '{}': {}",
                        db_name,
                        e.to_string()
                    ))
                }
            }
        }
        None => (),
    }

    Result::Ok(())
}

pub async fn handler(
    db_map: web::Data<HashMap<String, Db>>,
    db_name: Path<String>,
    macro_name: Path<String>,
    token: web::Query<Token>,
) -> impl Responder {
    let db_conf = db_map.get(db_name.as_str());
    match db_conf {
        Some(db_conf) => match db_conf.macros.get(&macro_name.to_string()) {
            Some(macr) => match &macr.execution {
                Some(mex) => match &mex.web_service {
                    Some(mex_ws) => {
                        if !process_creds(
                            &token.token,
                            &mex_ws.auth_token,
                            &mex_ws.hashed_auth_token,
                        ) {
                            return Response::new_err(
                                401,
                                -1,
                                format!(
                                    "In database '{}', macro '{}': token mismatch",
                                    db_name, macro_name
                                ),
                            );
                        }

                        let db_lock = MUTEXES.get().unwrap().get(&db_name.to_string()).unwrap();
                        let mut db_lock_guard = db_lock.lock().unwrap();
                        let conn = db_lock_guard.deref_mut();

                        exec_macro_single(&macr, conn)
                    }
                    None => Response::new_err(
                        404,
                        -1,
                        format!(
                            "In database '{}', macro '{}' doesn't have a backup.execution node",
                            db_name, macro_name
                        ),
                    ),
                },
                None => Response::new_err(
                    404,
                    -1,
                    format!("In database '{}', unknown macro '{}'", db_name, macro_name),
                ),
            },
            None => {
                return Response::new_err(
                    404,
                    -1,
                    format!(
                        "Database '{}' doesn't have a macro named '{}'",
                        db_name, macro_name
                    ),
                )
            }
        },
        None => Response::new_err(404, -1, format!("Unknown database '{}'", db_name.as_str())),
    }
}

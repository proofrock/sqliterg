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

use std::{collections::HashMap, ops::DerefMut};

use actix_web::{
    web::{self, Path},
    Responder,
};
use eyre::Result;
use rusqlite::{Connection, Transaction};

use crate::{
    auth::process_creds,
    commons::check_stored_stmt,
    db_config::DbConfig,
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
) -> Result<HashMap<String, Vec<String>>> {
    let mut macros = HashMap::new();
    match &dbconf.macros {
        Some(ms) => {
            for el in ms {
                let mut statements: Vec<String> = vec![];
                for statement in el.statements.clone() {
                    let statement = check_stored_stmt(&statement, stored_statements, false)?;
                    statements.push(statement.clone());
                }
                macros.insert(el.id.clone(), statements);
            }
        }
        None => (),
    }
    Ok(macros)
}

fn exec_macro_inner(
    id: &String,
    macros_def: &HashMap<String, Vec<String>>,
    tx: &Transaction,
) -> Response {
    let macr = macros_def.get(id);
    if macr.is_none() {
        return Response::new_err(400, -1, format!("Macro '{}' not found", id));
    }
    let macr = macr.unwrap();

    let mut changed_rows_s = vec![];
    for (i, statement) in macr.iter().enumerate() {
        let changed_rows = tx.execute(statement, []);
        match changed_rows {
            Ok(cr) => changed_rows_s.push(cr),
            Err(e) => return Response::new_err(500, i as isize, e.to_string()),
        }
    }

    let mut ret = vec![];
    for cr in changed_rows_s {
        ret.push(ResponseItem {
            success: true,
            error: None,
            result_set: None,
            rows_updated: Some(cr),
            rows_updated_batch: None,
        });
    }

    Response::new_ok(ret)
}

pub fn exec_macro(
    id: &String,
    macros_def: &HashMap<String, Vec<String>>,
    conn: &mut Connection,
) -> Response {
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(_) => {
            return Response::new_err(
                500,
                -1,
                format!("Transaction open failed for macro '{}'", id),
            )
        }
    };
    let ret = exec_macro_inner(id, macros_def, &tx);
    if ret.success {
        match tx.commit() {
            Ok(_) => (),
            Err(_) => {
                return Response::new_err(500, -1, format!("Commit failed for macro '{}'", id))
            }
        }
    } else {
        let _ = tx.rollback();
    }
    ret
}

pub fn exec_init_startup_macros(
    is_new_db: bool,
    init_macros: Option<Vec<String>>,
    startup_macros: Option<Vec<String>>,
    macros_def: &HashMap<String, Vec<String>>,
    conn: &mut Connection,
) -> Result<()> {
    let tx = conn.transaction()?;

    let mut result_so_far = Ok(());

    if is_new_db {
        match init_macros {
            Some(ims) => {
                for im in ims {
                    let res = exec_macro_inner(&im, macros_def, &tx);
                    if !res.success {
                        result_so_far = Err(("Init", im, res.message.unwrap()));
                        break;
                    }
                }
            }
            None => (),
        }
    }

    if result_so_far.is_ok() {
        match startup_macros {
            Some(sms) => {
                for sm in sms {
                    let res = exec_macro_inner(&sm, macros_def, &tx);
                    if !res.success {
                        result_so_far = Err(("Startup", sm, res.message.unwrap()));
                        break;
                    }
                }
            }
            None => (),
        }
    }

    match result_so_far {
        Ok(_) => match tx.commit() {
            Ok(_) => Ok(()),
            Err(_) => Err(eyre!("Commit failed for init/startup macro(s)")),
        },
        Err(er) => {
            let _ = tx.rollback();
            Err(eyre!("{} macro '{}' failed: {}", er.0, er.1, er.2))
        }
    }
}

pub async fn handler(
    db_map: web::Data<HashMap<String, Db>>,
    db_name: Path<String>,
    macro_name: Path<String>,
    token: web::Query<Token>,
) -> impl Responder {
    let db_conf = db_map.get(db_name.as_str());
    match db_conf {
        Some(db_conf) => {
            match &db_conf.conf.macros_endpoint {
                Some(me) => {
                    if !process_creds(&token.token, &me.auth_token, &me.hashed_auth_token) {
                        return Response::new_err(401, -1, "Token mismatch".to_string());
                    }
                }
                None => {
                    return Response::new_err(
                        404,
                        -1,
                        format!(
                            "Database '{}' doesn't have a macrosEndpoint",
                            db_name.as_str()
                        ),
                    )
                }
            }

            let db_lock = MUTEXES.get().unwrap().get(&db_name.to_string()).unwrap();
            let mut db_lock_guard = db_lock.lock().unwrap();
            let conn = db_lock_guard.deref_mut();

            exec_macro(&macro_name, &db_conf.macros, conn)
        }
        None => Response::new_err(404, -1, format!("Unknown database '{}'", db_name.as_str())),
    }
}

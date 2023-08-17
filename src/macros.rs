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
    post,
    web::{self, Path},
    Responder,
};
use eyre::Result;
use rusqlite::{Connection, Transaction};

use crate::{
    commons::check_stored_stmt,
    db_config::DbConfig,
    main_config::Db,
    req_res::{Response, ResponseItem},
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
        return Response {
            results: None,
            req_idx: Some(-1),
            message: Some(format!("Macro '{}' not found", id)),
            status_code: 400,
            success: false,
        };
    }
    let macr = macr.unwrap();

    let mut changed_rows_s = vec![];
    for (i, statement) in macr.iter().enumerate() {
        let changed_rows = tx.execute(statement, []);
        match changed_rows {
            Ok(cr) => changed_rows_s.push(cr),
            Err(e) => {
                return Response {
                    results: None,
                    req_idx: Some(i as isize),
                    message: Some(e.to_string()),
                    status_code: 500,
                    success: false,
                }
            }
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

    Response {
        results: Some(ret),
        req_idx: None,
        message: None,
        status_code: 200,
        success: true,
    }
}

pub fn exec_macro(
    id: &String,
    macros_def: &HashMap<String, Vec<String>>,
    conn: &mut Connection,
) -> Response {
    let tx = match conn.transaction() {
        Ok(tx) => tx,
        Err(_) => {
            return Response {
                results: None,
                req_idx: Some(-1),
                message: Some(format!("Transaction open failed for macro '{}'", id)),
                status_code: 500,
                success: false,
            }
        }
    };
    let ret = exec_macro_inner(id, macros_def, &tx);
    if ret.success {
        match tx.commit() {
            Ok(_) => (),
            Err(_) => {
                return Response {
                    results: None,
                    req_idx: Some(-1),
                    message: Some(format!("Commit failed for macro '{}'", id)),
                    status_code: 500,
                    success: false,
                }
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
                    if !exec_macro_inner(&im, macros_def, &tx).success {
                        result_so_far = Err(("Init", im));
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
                    if !exec_macro_inner(&sm, macros_def, &tx).success {
                        result_so_far = Err(("Startup", sm));
                        break;
                    }
                }
            }
            None => (),
        }
    }

    match result_so_far {
        Ok(_) => Ok(()),
        Err(err_elem) => Err(eyre!("{} macro '{}' failed", err_elem.0, err_elem.1)),
    }
}

#[post("/macro/{db_name}/{macro_name}")]
pub async fn handler(
    db_map: web::Data<HashMap<String, Db>>,
    db_name: Path<String>,
    macro_name: Path<String>,
) -> impl Responder {
    let db_conf = db_map.get(db_name.as_str());
    match db_conf {
        Some(db_conf) => {
            let db_lock = &db_conf.mutex;
            let mut db_lock_guard = db_lock.lock().unwrap();
            let conn = db_lock_guard.deref_mut();

            exec_macro(&macro_name, &db_conf.macros, conn)
        }
        None => Response {
            results: None,
            req_idx: Some(-1),
            message: Some(format!("Unknown database '{}'", db_name.as_str())),
            status_code: 404,
            success: false,
        },
    }
}

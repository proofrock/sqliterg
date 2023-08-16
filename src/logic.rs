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
use rusqlite::{types::Value, Connection, ToSql, Transaction};
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::{
    commons::{prepend_column, NamedParamsContainer},
    req_res::{self, ReqTransactionItem, Response, ResponseItem},
    main_config::Db,
};

fn val_db2val_json(val: Value) -> JsonValue {
    match val {
        Value::Null => JsonValue::Null,
        Value::Integer(v) => json!(v),
        Value::Real(v) => json!(v),
        Value::Text(v) => json!(v),
        Value::Blob(v) => json!(v),
    }
}

fn calc_named_params(params: &JsonMap<String, JsonValue>) -> NamedParamsContainer {
    let mut named_params: Vec<(String, Box<dyn ToSql>)> = Vec::new();

    params
        .iter()
        .for_each(|(k, v)| named_params.push((prepend_column(k), Box::new(v.clone()))));

    NamedParamsContainer::from(named_params)
}

fn check_stored_stmt<'a>(
    sql: &'a String,
    stored_statements: &'a HashMap<String, String>,
) -> Result<&'a String> {
    match sql.strip_prefix("#") {
        Some(s) => match stored_statements.get(&s.to_string()) {
            Some(s) => return Ok(s),
            None => return Err(eyre!("Stored statement '{}' not found", sql)),
        },
        None => return Ok(sql),
    }
}

fn do_query(
    tx: &Transaction,
    sql: &String,
    values: &Option<JsonValue>,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    let stmt = tx.prepare(&sql)?;
    let column_names = (&stmt).column_names();
    let mut stmt = tx.prepare(&sql)?; // FIXME statement is calculated two times :-(
    let mut rows = match values {
        Some(p) => {
            let map = p.as_object().unwrap();
            let params = calc_named_params(map);
            stmt.query(params.slice().as_slice())?
        }
        None => stmt.query([])?,
    };
    let mut response = vec![];
    while let Some(row) = rows.next().unwrap() {
        let mut map: JsonMap<String, JsonValue> = JsonMap::new();
        for (i, col_name) in column_names.iter().enumerate() {
            let value: Value = row.get_unwrap(i);
            map.insert(col_name.to_string(), val_db2val_json(value));
        }
        response.push(JsonValue::Object(map));
    }
    Ok((Some(response), None, None))
}

fn do_statement(
    tx: &Transaction,
    sql: &String,
    values: &Option<JsonValue>,
    values_batch: &Option<Vec<JsonValue>>,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    let mut params = vec![];
    if values.is_some() {
        params.push(values.as_ref().unwrap());
    }
    if values_batch.is_some() {
        for p in values_batch.as_ref().unwrap().iter() {
            params.push(p);
        }
    }

    Ok(match params.len() {
        0 => {
            let changed_rows = tx.execute(sql, [])?;
            (None, Some(changed_rows), None)
        }
        1 => {
            let map = params.get(0).unwrap().as_object().unwrap();
            let params = calc_named_params(&map);
            let changed_rows = tx.execute(sql, params.slice().as_slice())?;
            (None, Some(changed_rows), None)
        }
        _ => {
            let mut stmt = tx.prepare(&sql)?;
            let mut ret = vec![];
            for p in params.iter() {
                let map = p.as_object().unwrap();
                let params = calc_named_params(&map);
                let changed_rows = stmt.execute(params.slice().as_slice())?;
                ret.push(changed_rows);
            }
            (None, None, Some(ret))
        }
    })
}

fn process(
    conn: &mut Connection,
    http_req: web::Json<req_res::Request>,
    stored_statements: &HashMap<String, String>,
) -> Result<Response> {
    let tx = conn.transaction()?;
    let mut results = vec![];
    let mut failed = None;

    for (idx, trx_item) in http_req.transaction.iter().enumerate() {
        let mut _no_fail: bool;
        let ret = match trx_item {
            ReqTransactionItem::Query {
                no_fail,
                query,
                values,
            } => {
                _no_fail = *no_fail;
                do_query(&tx, check_stored_stmt(query, stored_statements)?, values)
            }
            ReqTransactionItem::Stmt {
                no_fail,
                statement,
                values,
                values_batch,
            } => {
                _no_fail = *no_fail;
                do_statement(
                    &tx,
                    check_stored_stmt(statement, stored_statements)?,
                    values,
                    values_batch,
                )
            }
        };

        if !ret.is_ok() && !_no_fail {
            failed = Some((idx, ret.unwrap_err().to_string()));
            break;
        }

        results.push(match ret {
            Ok(val) => ResponseItem {
                success: true,
                error: None,
                result_set: val.0,
                rows_updated: val.1,
                rows_updated_batch: val.2,
            },
            Err(err) => ResponseItem {
                success: false,
                error: Some(err.to_string()),
                result_set: None,
                rows_updated: None,
                rows_updated_batch: None,
            },
        });
    }

    Ok(match failed {
        Some(f) => {
            tx.rollback()?;
            Response {
                results: None,
                req_idx: Some(f.0 as isize),
                message: Some(f.1),
            }
        }
        None => {
            tx.commit()?;
            Response {
                results: Some(results),
                req_idx: None,
                message: None,
            }
        }
    })
}

pub async fn handler(
    db_map:  web::Data<HashMap<String, Db>>,
    http_req: web::Json<req_res::Request>,
    db_name: Path<String>,
) -> impl Responder {
    let db_conf = db_map.get(db_name.as_str());
    match db_conf {
        Some(db_conf) => {
            let db_lock = &db_conf.mutex;
            let mut db_lock_guard = db_lock.lock().unwrap();
            let conn = db_lock_guard.deref_mut();

            let result = process(conn, http_req, &db_conf.stored_statements).unwrap();

            result
        }
        None => Response {
            results: None,
            req_idx: Some(-1),
            message: Some(format!("Unknown database '{}'", db_name.as_str())),
        },
    }
}

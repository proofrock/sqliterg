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

use eyre::Result;
use rusqlite::{types::Value, Connection, ToSql, Transaction};
use serde_json::{json, Map as JsonMap, Value as JsonValue};

use crate::{
    commons::{prepend_column, NamedParamsContainer},
    req_res::{self, ReqTransactionItem, Response, ResponseItem},
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

// TODO queries cannot have a valuesBatch
fn do_query(
    tx: &Transaction,
    req: &ReqTransactionItem,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    if req.values_batch.is_some() {
        return Err(eyre!("A query cannot have a valuesBatch argument"));
    }

    let sql = req.query.as_ref().unwrap();
    let stmt = tx.prepare(&sql)?;
    let column_names = stmt.column_names();
    let mut stmt = tx.prepare(&sql)?; // FIXME statement is calculated two times :-(
    let params_ref: Option<&JsonValue> = req.values.as_ref();
    let mut rows = match params_ref {
        Some(p) => {
            let map = p.as_object().unwrap();
            let params = calc_named_params(map); // TODO manage the error!
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

// rusqlite cannot use "real" batching, maybe it doesn't even make sense in sqlite
fn do_statement(
    tx: &Transaction,
    req: &ReqTransactionItem,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    let sql = req.statement.as_ref().unwrap();
    let mut stmt = tx.prepare(&sql)?;
    let params_ref: Option<&JsonValue> = req.values.as_ref();
    let changed_rows = match params_ref {
        Some(p) => {
            let map = p.as_object().unwrap();
            let params = calc_named_params(&map); // TODO manage the error!
            stmt.execute(params.slice().as_slice())?
        }
        None => stmt.execute([])?,
    };
    Ok((None, Some(changed_rows), None))
}

fn do_statement_batch(
    tx: &Transaction,
    req: &ReqTransactionItem,
) -> Result<(Option<Vec<JsonValue>>, Option<usize>, Option<Vec<usize>>)> {
    let sql = req.statement.as_ref().unwrap();
    let mut stmt = tx.prepare(&sql)?;
    let mut ret = vec![];
    match req.values.as_ref() {
        // if there are both values and values_batch, values goes first
        Some(p) => {
            let map = p.as_object().unwrap();
            let params = calc_named_params(&map);
            let changed_rows = stmt.execute(params.slice().as_slice())?;
            ret.push(changed_rows);
        }
        None => (),
    };
    for params in req.values_batch.as_ref().unwrap() {
        match params {
            Some(p) => {
                let map = p.as_object().unwrap();
                let params = calc_named_params(&map);
                let changed_rows = stmt.execute(params.slice().as_slice())?;
                ret.push(changed_rows);
            }
            None => continue,
        };
    }
    Ok((None, None, Some(ret)))
}

pub fn process(conn: &mut Connection, query: &req_res::Request) -> Result<Response> {
    let tx = conn.transaction()?;
    let mut results = vec![];
    let mut failed = None;

    for (idx, trx_item) in query.transaction.iter().enumerate() {
        let ret = match trx_item.query {
            Some(_) => do_query(&tx, &trx_item),
            None => match trx_item.statement {
                Some(_) => match trx_item.values_batch {
                    Some(_) => do_statement_batch(&tx, &trx_item),
                    None => do_statement(&tx, &trx_item),
                },
                None => Err(eyre!("Neither a query nor a statement is specified")),
            },
        };

        if !ret.is_ok() && !trx_item.no_fail {
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

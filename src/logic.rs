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

use rusqlite::{types::Value, Connection, Error, Transaction};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use serde_rusqlite::to_params_named; // test serde_json option in Cargo.toml

use crate::req_res::{self, ReqTransaction, Response, ResponseItemQuery};

fn val_db2val_json(val: Value) -> JsonValue {
    match val {
        Value::Null => JsonValue::Null,
        Value::Integer(v) => json!(v),
        Value::Real(v) => json!(v),
        Value::Text(v) => json!(v),
        Value::Blob(v) => json!(v),
    }
}

// {"transaction": [{"query": "SELECT * FROM TBL WHERE ID=:id", "values":["id": 1],}]}
// TODO queries cannot have a valuesBatch
fn do_query(tx: &Transaction, req: &ReqTransaction) -> Result<Vec<JsonValue>, Error> {
    let sql = req.query.as_ref().unwrap();
    let stmt = tx.prepare(&sql)?;
    let column_names = stmt.column_names();
    let mut stmt = tx.prepare(&sql)?;
    let params_ref: Option<&JsonValue> = req.values.as_ref();
    let mut rows = match params_ref {
        Some(p) => {
            let map = p.as_object().unwrap();
            let params = to_params_named(&map).unwrap(); // TODO manage the error!
            stmt.query(params.to_slice().as_slice())?
        } // TODO
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
    Ok(response)
}

pub fn process(conn: &mut Connection, query: &req_res::Request) -> Result<Response, Error> {
    let tx = conn.transaction()?;
    let mut results = vec![];
    for (_, trx) in query.transaction.iter().enumerate() {
        let ret = do_query(&tx, trx);
        let ret = match ret {
            Ok(val) => ResponseItemQuery {
                success: true,
                error: None,
                result_set: Some(val),
            },
            Err(err) => ResponseItemQuery {
                success: false,
                error: Some(err.to_string()),
                result_set: None,
            },
        };
        results.push(ret);
    }
    tx.commit()?;

    Ok(Response { results })
}

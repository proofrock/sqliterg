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

use rusqlite::types::Value;
use rusqlite::{Connection, Error};

use serde_json::json;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;

use crate::req_res::{self, ReqTransaction, Response, ResponseItemQuery};

fn val2val(val: Value) -> JsonValue {
    match val {
        Value::Null => JsonValue::Null,
        Value::Integer(v) => json!(v),
        Value::Real(v) => json!(v),
        Value::Text(v) => json!(v),
        Value::Blob(v) => json!(v),
    }
}

fn do_query(conn: &Connection, req: &ReqTransaction) -> Result<Vec<JsonValue>, Error> {
    let sql = req.query.as_ref().unwrap();
    let stmt = conn.prepare(&sql)?;
    let column_names = stmt.column_names();
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut response = vec![];
    while let Some(row) = rows.next().unwrap() {
        let mut map: JsonMap<String, JsonValue> = JsonMap::new();
        for (i, col_name) in column_names.iter().enumerate() {
            let value: Value = row.get_unwrap(i);

            map.insert(col_name.to_string(), val2val(value));
        }
        response.push(JsonValue::Object(map));
    }
    Ok(response)
}

pub fn process(conn: &Connection, query: &req_res::Request) -> Result<Response, Error> {
    let mut results = vec![];
    for (_, trx) in query.transaction.iter().enumerate() {
        let ret = do_query(conn, trx);
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

    Ok(Response { results })
}

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

use actix_web::{
    body::BoxBody,
    http::{header::ContentType, StatusCode},
    HttpRequest, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};

use serde_json::Value as JsonValue;

use crate::commons::default_as_false;

#[derive(Debug, Deserialize)]
pub struct ReqCredentials {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]

pub enum ReqTransactionItem {
    Query {
        #[serde(rename = "noFail")]
        #[serde(default = "default_as_false")]
        no_fail: bool,
        query: String,
        values: Option<JsonValue>,
    },
    Stmt {
        #[serde(rename = "noFail")]
        #[serde(default = "default_as_false")]
        no_fail: bool,
        statement: String,
        values: Option<JsonValue>,
        #[serde(rename = "valuesBatch")]
        values_batch: Option<Vec<JsonValue>>,
    },
}

#[derive(Debug, Deserialize)]
pub struct Request {
    pub credentials: Option<ReqCredentials>,
    pub transaction: Vec<ReqTransactionItem>,
}

#[derive(Debug, Serialize)]
pub struct ResponseItem {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(rename = "resultSet")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_set: Option<Vec<JsonValue>>,
    #[serde(rename = "rowsUpdated")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_updated: Option<usize>,
    #[serde(rename = "rowsUpdatedBatch")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_updated_batch: Option<Vec<usize>>,
}

#[derive(Debug, Serialize)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<ResponseItem>>,
    #[serde(rename = "reqIdx")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub req_idx: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing)]
    pub status_code: u16,
    #[serde(skip_serializing)]
    pub success: bool,
}

impl Responder for Response {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        HttpResponse::Ok()
            .status(StatusCode::from_u16(self.status_code).unwrap())
            .content_type(ContentType::json())
            .body(body)
    }
}

impl Response {
    pub fn new_ok(results: Vec<ResponseItem>) -> Response {
        Response {
            results: Some(results),
            req_idx: None,
            message: None,
            status_code: 200,
            success: true,
        }
    }

    pub fn new_err(status_code: u16, req_idx: isize, msg: String) -> Response {
        Response {
            results: None,
            req_idx: Some(req_idx),
            message: Some(msg),
            status_code: status_code,
            success: false,
        }
    }
}

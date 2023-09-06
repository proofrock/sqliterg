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

use crate::commons::default_as_false;
use actix_web::{
    body::BoxBody,
    http::{header::ContentType, StatusCode},
    HttpRequest, HttpResponse, Responder,
};
use serde::{Deserialize, Serialize};

use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize)]
pub struct ReqCredentials {
    pub user: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ReqTransactionItem {
    #[serde(rename = "noFail")]
    #[serde(default = "default_as_false")]
    pub no_fail: bool,
    pub query: Option<String>,
    pub statement: Option<String>,
    pub values: Option<JsonValue>,
    #[serde(rename = "valuesBatch")]
    pub values_batch: Option<Vec<JsonValue>>,
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
            status_code,
            success: false,
        }
    }
}

#[derive(Deserialize)]
pub struct Token {
    pub token: Option<String>,
}

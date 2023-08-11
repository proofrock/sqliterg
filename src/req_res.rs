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

use actix_web::{body::BoxBody, http::header::ContentType, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use serde_json::Value as JsonValue;

use crate::commons::default_as_false;

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
    pub values_batch: Option<Vec<Option<JsonValue>>>,
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
}

impl Responder for Response {
    type Body = BoxBody;

    fn respond_to(self, _req: &HttpRequest) -> HttpResponse<Self::Body> {
        let body = serde_json::to_string(&self).unwrap();

        // Create response and set content type
        HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body)
    }
}
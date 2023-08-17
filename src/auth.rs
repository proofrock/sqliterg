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

use actix_web_httpauth::headers::authorization::{Authorization, Basic};
use rusqlite::{named_params, Connection};

use crate::{
    commons::{equal_case_insensitive, sha256},
    db_config::Auth,
    db_config::{AuthMode, Credentials},
    req_res::ReqCredentials,
};

fn auth_by_credentials(user: String, password: String, creds: &Vec<Credentials>) -> bool {
    let mut reg_pass: Option<String> = None;
    let mut reg_pass_format = 0; // 0=plaintext, 1=hashed
    for c in creds.iter() {
        // TODO hash table lookup
        if equal_case_insensitive(&user, &c.user) {
            match &c.password {
                Some(p) => reg_pass = Some(p.clone()),
                None => match &c.hashed_password {
                    Some(hp) => {
                        reg_pass = Some(hp.clone());
                        reg_pass_format = 1;
                    }
                    None => (),
                },
            }
            break;
        }
    }
    match reg_pass {
        Some(p) => {
            if reg_pass_format == 0 {
                p == password
            } else {
                let shap = sha256(password);
                equal_case_insensitive(&p, &shap)
            }
        }
        None => false,
    }
}

fn auth_by_query(user: String, password: String, query: &String, conn: &mut Connection) -> bool {
    let res = conn.query_row(
        &query,
        named_params! {":user": user, ":password":password},
        |_| Ok(()),
    );
    res.is_ok()
}

fn creds_from_http(auth_header: &Authorization<Basic>) -> (String, String) {
    let user = auth_header.as_ref().user_id();
    let password = auth_header.as_ref().password().unwrap();
    (String::from(user), String::from(password))
}

fn creds_from_inline(auth_inline: &ReqCredentials) -> (String, String) {
    let user = auth_inline.user.as_str();
    let password = auth_inline.password.as_str();
    (String::from(user), String::from(password))
}

pub fn process_auth(
    auth_config: &Auth,
    conn: &mut Connection,
    auth_inline: &Option<ReqCredentials>,
    auth_header: &Option<Authorization<Basic>>,
) -> bool {
    let (user, password) = match auth_config.mode {
        AuthMode::HttpBasic => match auth_header {
            Some(auth_header) => creds_from_http(auth_header),
            None => return false,
        },
        AuthMode::Inline => match auth_inline {
            Some(auth_inline) => creds_from_inline(auth_inline),
            None => return false,
        },
    };

    match &auth_config.by_credentials {
        Some(creds) => auth_by_credentials(user, password, creds),
        None => match &auth_config.by_query {
            Some(query) => auth_by_query(user, password, query, conn),
            None => false,
        },
    }
}

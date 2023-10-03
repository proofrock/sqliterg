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

use std::ops::DerefMut;

use actix_web_httpauth::headers::authorization::{Authorization, Basic};
use rusqlite::named_params;

use crate::{
    commons::{equal_case_insensitive, sha256},
    db_config::Auth,
    db_config::{AuthMode, Credentials},
    req_res::ReqCredentials,
    MUTEXES,
};

/// Given the provided password and the expected ones (unhashed and hashed), returns if
/// the passwords match. All three passwords may be Options.
pub fn process_creds(
    given_password: &Option<String>,
    password: &Option<String>,
    hashed_password: &Option<String>,
) -> bool {
    match (given_password, password, hashed_password) {
        (_, None, None) => true,
        (Some(gp), Some(p), _) => gp == p,
        (Some(gp), None, Some(hp)) => equal_case_insensitive(hp, &sha256(gp)),
        _ => false,
    }
}

fn auth_by_credentials(user: String, password: String, creds: &Vec<Credentials>) -> bool {
    for c in creds {
        // TODO hash table lookup? I don't expect the credentials list to grow very much, so it may be overkill (and use memory)
        if equal_case_insensitive(&user, &c.user) {
            return process_creds(&Some(password), &c.password, &c.hashed_password);
        }
    }
    false
}

fn auth_by_query(user: String, password: String, query: &str, db_name: &str) -> bool {
    let db_lock = MUTEXES.get().unwrap().get(db_name).unwrap();
    let mut db_lock_guard = db_lock.lock().unwrap();
    let conn = db_lock_guard.deref_mut();

    let res = conn.query_row(
        query,
        named_params! {":user": user, ":password":password},
        |_| Ok(()),
    );
    res.is_ok()
}

pub fn process_auth(
    auth_config: &Auth,
    auth_inline: &Option<ReqCredentials>,
    auth_header: &Option<Authorization<Basic>>,
    db_name: &str,
) -> bool {
    let (user, password) = match auth_config.mode {
        AuthMode::HttpBasic => match auth_header {
            Some(auth_header) => (
                auth_header.as_ref().user_id().to_string(),
                auth_header.as_ref().password().unwrap().to_string(),
            ),
            None => return false,
        },
        AuthMode::Inline => match auth_inline {
            Some(auth_inline) => (auth_inline.user.to_owned(), auth_inline.password.to_owned()),
            None => return false,
        },
    };

    match &auth_config.by_credentials {
        Some(creds) => auth_by_credentials(user, password, creds),
        None => match &auth_config.by_query {
            Some(query) => auth_by_query(user, password, query, db_name),
            None => false,
        },
    }
}

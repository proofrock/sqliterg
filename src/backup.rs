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

use crate::{
    commons::{file_exists, now},
    main_config::Db,
    req_res::Response,
};

#[post("/db/{db_name}")]
pub async fn handler(
    db_map: web::Data<HashMap<String, Db>>,
    db_name: Path<String>,
) -> impl Responder {
    let db_conf = db_map.get(db_name.as_str());
    match db_conf {
        Some(db_conf) => match &db_conf.conf.backup {
            Some(bkp) => {
                let file = bkp.backup_template.replace("%s", &now());
                if file_exists(&file) {
                    Response::new_err(409, -1, format!("File '{}' already exists", file))
                } else {
                    let db_lock = &db_conf.mutex;
                    let mut db_lock_guard = db_lock.lock().unwrap();
                    let conn = db_lock_guard.deref_mut();

                    match conn.execute("BACKUP TO ?1", [file]) {
                        Ok(_) => Response::new_ok(vec![]),
                        Err(e) => Response::new_err(500, -1, e.to_string()),
                    }
                }
            }
            None => Response::new_err(
                404,
                -1,
                format!("Database '{}' don't have a backup plan", db_name.as_str()),
            ),
        },
        None => Response::new_err(404, -1, format!("Unknown database '{}'", db_name.as_str())),
    }
}

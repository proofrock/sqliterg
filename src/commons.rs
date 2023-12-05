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

use chrono::{Datelike, Local, Timelike};
use eyre::Result;
use ring::digest::{Context, SHA256};
use std::{
    borrow::Borrow,
    collections::HashMap,
    fs::{read_dir, remove_file},
    ops::Deref,
    path::Path,
    process::exit,
};

// General utils

pub fn abort(str: String) -> ! {
    eprintln!("FATAL: {}", str);
    exit(1);
}

// https://github.com/serde-rs/serde/issues/1030#issuecomment-522278006
pub fn default_as_false() -> bool {
    false
}

pub fn default_as_true() -> bool {
    true
}

pub fn default_as_zero() -> i32 {
    0
}

pub fn file_exists(path: &str) -> bool {
    let path = Path::new(path);
    Path::new(path).exists()
}

pub fn is_dir(path: &String) -> bool {
    let path = Path::new(path);
    Path::new(path).is_dir()
}

pub fn is_file_in_directory(file_path: &str, dir_path: &str) -> bool {
    let file_path = Path::new(file_path);
    let dir_path = Path::new(dir_path);

    file_path.starts_with(dir_path)
}

pub fn sha256(input: &String) -> String {
    let digest = {
        let mut context = Context::new(&SHA256);
        context.update(input.as_bytes()); // UTF-8
        context.finish()
    };

    hex::encode(digest.as_ref())
}

pub fn equal_case_insensitive(s1: &str, s2: &str) -> bool {
    s1.to_lowercase() == s2.to_lowercase()
}

pub fn now() -> String {
    let current_datetime = Local::now();

    let year = current_datetime.year();
    let month = current_datetime.month();
    let day = current_datetime.day();
    let hour = current_datetime.hour();
    let minute = current_datetime.minute();

    format!("{:04}{:02}{:02}-{:02}{:02}", year, month, day, hour, minute)
}

pub fn delete_old_files(dir: &str, files_to_keep: usize) -> Result<()> {
    let path = Path::new(dir);
    let dir = path.parent().map(|parent| parent.to_path_buf()).unwrap();

    let mut entries: Vec<_> = read_dir(dir)?.filter_map(|entry| entry.ok()).collect();

    entries.sort_by(|a, b| {
        let a_meta = a.metadata().unwrap();
        let b_meta = b.metadata().unwrap();
        a_meta.modified().unwrap().cmp(&b_meta.modified().unwrap())
    });

    let num_entries = entries.len();

    if num_entries > files_to_keep {
        for entry in entries.iter().take(num_entries - files_to_keep) {
            if entry.path().is_file() {
                remove_file(entry.path())?;
            }
        }
    }

    Ok(())
}

pub fn check_stored_stmt<'a>(
    sql: &'a String,
    stored_statements: &'a HashMap<String, String>,
    use_only_stored_statements: bool,
) -> Result<&'a String> {
    match sql.strip_prefix('^') {
        Some(s) => match stored_statements.get(&s.to_string()) {
            Some(s) => Ok(s),
            None => Err(eyre!("Stored statement '{}' not found", sql)),
        },
        None => {
            if use_only_stored_statements {
                Err(eyre!(
                    "UseOnlyStoredStatement set but a stored statement wasn't used"
                ))
            } else {
                Ok(sql)
            }
        }
    }
}

pub fn resolve_tilde(p: &String) -> String {
    shellexpand::tilde(p).into_owned()
}

pub fn split_on_first_double_colon(input: &str) -> (String, String) {
    let mut parts = input.splitn(2, "::");
    let first_part = parts.next().unwrap_or_default().to_string();
    let second_part = parts.next().unwrap_or_default().to_string();

    (first_part, second_part)
}

pub fn if_abort_eyre<T>(result: eyre::Result<T>) -> T {
    result.unwrap_or_else(|e| abort(e.to_string()))
}

pub fn if_abort_rusqlite<T>(result: rusqlite::Result<T, rusqlite::Error>) -> T {
    result.unwrap_or_else(|e| abort(e.to_string()))
}

pub fn assert(condition: bool, msg: String) {
    if !condition {
        abort(msg);
    }
}

// Utils to convert serde structs to slices accepted by rusqlite as named params
// adapted from serde-rusqlite, https://github.com/twistedfall/serde_rusqlite/blob/master/LICENSE

pub struct NamedParamsContainer(Vec<(String, Box<dyn rusqlite::types::ToSql>)>);

impl NamedParamsContainer {
    pub fn slice(&self) -> Vec<(&str, &dyn rusqlite::types::ToSql)> {
        self.0
            .iter()
            .map(|el| (el.0.deref(), el.1.borrow()))
            .collect()
    }
}

impl From<Vec<(String, Box<dyn rusqlite::types::ToSql>)>> for NamedParamsContainer {
    fn from(src: Vec<(String, Box<dyn rusqlite::types::ToSql>)>) -> Self {
        Self(src)
    }
}

pub struct PositionalParamsContainer(Vec<Box<dyn rusqlite::types::ToSql>>);

impl PositionalParamsContainer {
    pub fn slice(&self) -> Vec<&dyn rusqlite::types::ToSql> {
        self.0.iter().map(|el| (el.borrow())).collect()
    }
}

impl From<Vec<Box<dyn rusqlite::types::ToSql>>> for PositionalParamsContainer {
    fn from(src: Vec<Box<dyn rusqlite::types::ToSql>>) -> Self {
        Self(src)
    }
}

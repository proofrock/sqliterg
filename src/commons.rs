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
use ring::digest::{Context, SHA256};
use std::{borrow::Borrow, collections::HashMap, path::Path};

// General utils

pub fn prepend_column(str: &String) -> String {
    let mut ret = ":".to_string();
    ret.push_str(str);
    return ret;
}

// https://github.com/serde-rs/serde/issues/1030#issuecomment-522278006
pub fn default_as_false() -> bool {
    false
}

pub fn default_as_true() -> bool {
    true
}

pub fn file_exists(path: &String) -> bool {
    let path = Path::new(path);
    Path::new(path).exists()
}

pub fn sha256(input: String) -> String {
    let digest = {
        let mut context = Context::new(&SHA256);
        context.update(input.as_bytes()); // UTF-8
        context.finish()
    };

    hex::encode(digest.as_ref())
}

pub fn equal_case_insensitive(s1: &String, s2: &String) -> bool {
    s1.to_lowercase() == s2.to_lowercase()
}

pub fn check_stored_stmt<'a>(
    sql: &'a String,
    stored_statements: &'a HashMap<String, String>,
    use_only_stored_statements: bool,
) -> Result<&'a String> {
    match sql.strip_prefix("^") {
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

// Utils to convert serde structs to slices accepted by rusqlite as named params
pub struct NamedParamsContainer(Vec<(String, Box<dyn rusqlite::types::ToSql>)>);

impl NamedParamsContainer {
    pub fn slice(&self) -> Vec<(&str, &dyn rusqlite::types::ToSql)> {
        self.0
            .iter()
            .map(|el| (el.0.as_str(), el.1.borrow()))
            .collect()
    }
}

impl From<Vec<(String, Box<dyn rusqlite::types::ToSql>)>> for NamedParamsContainer {
    fn from(src: Vec<(String, Box<dyn rusqlite::types::ToSql>)>) -> Self {
        Self(src)
    }
}

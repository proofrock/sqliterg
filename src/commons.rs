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

use std::borrow::Borrow;

// General utils

pub fn prepend_column(str: &String) -> String {
    let mut ret = String::from(":");
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

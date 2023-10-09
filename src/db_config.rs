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

use eyre::Result;
use std::fs::File;
use std::io::Read;

use crate::commons::{default_as_false, default_as_zero};

#[derive(Debug, Deserialize, Clone)]
pub enum AuthMode {
    #[serde(rename = "HTTP_BASIC")]
    HttpBasic,
    #[serde(rename = "INLINE")]
    Inline,
}

fn default_401() -> u16 {
    401
}

#[derive(Debug, Deserialize, Clone)]
pub struct Auth {
    #[serde(rename = "authErrorCode")]
    #[serde(default = "default_401")]
    pub auth_error_code: u16,
    pub mode: AuthMode,
    #[serde(rename = "byQuery")]
    pub by_query: Option<String>,
    #[serde(rename = "byCredentials")]
    pub by_credentials: Option<Vec<Credentials>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Credentials {
    pub user: String,
    pub password: Option<String>,
    #[serde(rename = "hashedPassword")]
    pub hashed_password: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StoredStatement {
    pub id: String,
    pub sql: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecutionWebService {
    #[serde(rename = "authErrorCode")]
    #[serde(default = "default_401")]
    pub auth_error_code: u16,
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
    #[serde(rename = "hashedAuthToken")]
    pub hashed_auth_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExecutionMode {
    #[serde(rename = "onCreate")]
    #[serde(default = "default_as_false")]
    pub on_create: bool,
    #[serde(rename = "onStartup")]
    #[serde(default = "default_as_false")]
    pub on_startup: bool,
    #[serde(default = "default_as_zero")]
    pub period: i32,
    #[serde(rename = "webService")]
    pub web_service: Option<ExecutionWebService>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Macro {
    pub id: String,
    #[serde(rename = "disableTransaction")]
    #[serde(default = "default_as_false")]
    pub disable_transaction: bool,
    pub statements: Vec<String>,
    pub execution: ExecutionMode,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Backup {
    #[serde(rename = "backupDir")]
    pub backup_dir: String,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    pub execution: ExecutionMode,
}

#[derive(Debug, Default, Deserialize, Clone)]
pub struct DbConfig {
    pub auth: Option<Auth>,
    #[serde(rename = "journalMode")]
    pub journal_mode: Option<String>,
    #[serde(rename = "readOnly")]
    #[serde(default = "default_as_false")]
    pub read_only: bool,
    #[serde(rename = "corsOrigin")]
    pub cors_origin: Option<String>,
    #[serde(rename = "useOnlyStoredStatements")]
    #[serde(default = "default_as_false")]
    pub use_only_stored_statements: bool,
    #[serde(rename = "storedStatements")]
    pub stored_statements: Option<Vec<StoredStatement>>,
    pub macros: Option<Vec<Macro>>,
    pub backup: Option<Backup>,
}

pub fn parse_dbconf(filename: &String) -> Result<DbConfig> {
    let mut file = File::open(filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let ret = serde_yaml::from_str(&content)?;

    Ok(ret)
}

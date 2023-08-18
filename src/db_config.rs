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

use std::fs::File;
use std::io::Read;

use crate::commons::{default_as_false, default_as_true};

#[derive(Debug, Deserialize)]
pub enum AuthMode {
    #[serde(rename = "HTTP_BASIC")]
    HttpBasic,
    #[serde(rename = "INLINE")]
    Inline,
}

#[derive(Debug, Deserialize)]
pub struct Auth {
    pub mode: AuthMode,
    #[serde(rename = "byQuery")]
    pub by_query: Option<String>,
    #[serde(rename = "byCredentials")]
    pub by_credentials: Option<Vec<Credentials>>,
}

#[derive(Debug, Deserialize)]
pub struct Credentials {
    pub user: String,
    pub password: Option<String>,
    #[serde(rename = "hashedPassword")]
    pub hashed_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Macro {
    pub id: String,
    pub statements: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct MacrosEndpoint {
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
    #[serde(rename = "hashedAuthToken")]
    pub hashed_auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Backup {
    #[serde(rename = "backupDir")]
    pub backup_dir: String,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    #[serde(rename = "atStartup")]
    pub at_startup: bool,
}

#[derive(Debug, Deserialize)]
pub struct BackupEndpoint {
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
    #[serde(rename = "hashedAuthToken")]
    pub hashed_auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DbConfig {
    pub auth: Option<Auth>,
    #[serde(rename = "disableWALMode")]
    #[serde(default = "default_as_false")]
    pub disable_wal_mode: bool,
    #[serde(rename = "readOnly")]
    #[serde(default = "default_as_false")]
    pub read_only: bool,
    #[serde(rename = "persistentConnection")]
    #[serde(default = "default_as_true")]
    pub persistent_connection: bool,
    #[serde(rename = "corsOrigin")]
    pub cors_origin: Option<String>,
    #[serde(rename = "useOnlyStoredStatements")]
    #[serde(default = "default_as_false")]
    pub use_only_stored_statements: bool,
    #[serde(rename = "storedStatements")]
    pub stored_statements: Option<Vec<StoredStatement>>,
    pub macros: Option<Vec<Macro>>,
    #[serde(rename = "initMacros")]
    pub init_macros: Option<Vec<String>>,
    #[serde(rename = "startupMacros")]
    pub startup_macros: Option<Vec<String>>,
    #[serde(rename = "macrosEndpoint")]
    pub macros_endpoint: Option<MacrosEndpoint>,
    pub backup: Option<Backup>,
    #[serde(rename = "backupEndpoint")]
    pub backup_endpoint: Option<BackupEndpoint>,
}

impl DbConfig {
    pub fn default() -> DbConfig {
        DbConfig {
            auth: None,
            disable_wal_mode: false,
            read_only: false,
            persistent_connection: true,
            cors_origin: None,
            use_only_stored_statements: false,
            stored_statements: None,
            macros: None,
            init_macros: None,
            startup_macros: None,
            macros_endpoint: None,
            backup: None,
            backup_endpoint: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StoredStatement {
    pub id: String,
    pub sql: String,
}

pub fn parse_dbconf(filename: &String) -> Result<DbConfig, Box<dyn std::error::Error>> {
    let mut file = File::open(filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(serde_yaml::from_str(&content)?)
}

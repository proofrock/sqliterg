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

use std::fs::File;
use std::io::Read;

use crate::commons::{default_as_false, default_as_true};

#[derive(Debug, Deserialize, Clone)]
pub enum AuthMode {
    #[serde(rename = "HTTP_BASIC")]
    HttpBasic,
    #[serde(rename = "INLINE")]
    Inline,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Auth {
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
pub struct Macro {
    pub id: String,
    pub statements: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MacrosEndpoint {
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
    #[serde(rename = "hashedAuthToken")]
    pub hashed_auth_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Backup {
    #[serde(rename = "backupDir")]
    pub backup_dir: String,
    #[serde(rename = "numFiles")]
    pub num_files: usize,
    #[serde(rename = "atStartup")]
    pub at_startup: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackupEndpoint {
    #[serde(rename = "authToken")]
    pub auth_token: Option<String>,
    #[serde(rename = "hashedAuthToken")]
    pub hashed_auth_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
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

#[derive(Debug, Deserialize, Clone)]
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

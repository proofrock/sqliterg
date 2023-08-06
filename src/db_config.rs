use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize)]
pub enum AuthMode {
    HTTP,
    INLINE,
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
pub struct ScheduledTask {
    #[serde(rename = "hashedPassword")]
    pub schedule: Option<String>,
    #[serde(rename = "doVacuum")]
    pub do_vacuum: Option<bool>,
    #[serde(rename = "doBackup")]
    pub do_backup: Option<bool>,
    #[serde(rename = "backupTemplate")]
    pub backup_template: Option<String>,
    #[serde(rename = "numFiles")]
    pub num_files: Option<i32>,
    pub statements: Option<Vec<String>>,
    #[serde(rename = "atStartup")]
    pub at_startup: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DbConfig {
    pub auth: Option<Auth>,
    #[serde(rename = "disableWALMode")]
    pub disable_wal_mode: Option<bool>,
    #[serde(rename = "readOnly")]
    pub read_only: Option<bool>,
    #[serde(rename = "scheduledTasks")]
    pub scheduled_tasks: Option<Vec<ScheduledTask>>,
    #[serde(rename = "corsOrigin")]
    pub cors_origin: Option<String>,
    #[serde(rename = "useOnlyStoredStatements")]
    pub use_only_stored_statements: Option<bool>,
    #[serde(rename = "storedStatements")]
    pub stored_statements: Option<Vec<StoredStatement>>,
    #[serde(rename = "initStatements")]
    pub init_statements: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct StoredStatement {
    pub id: String,
    pub sql: String,
}

pub fn parse_dbconf(filename: String) -> Result<DbConfig, Box<dyn std::error::Error>> {
    let mut file = File::open(filename)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let config: DbConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

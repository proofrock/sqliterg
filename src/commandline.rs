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

use clap::Parser;

use crate::commons::{assert, is_dir, resolve_tilde};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(
    help_template = "{name} {version}\n {about-section}\n {usage-heading} {usage}\n {all-args} {tab}"
)]
pub struct AppConfig {
    #[arg(
        long,
        value_name = "HOST",
        default_value = "0.0.0.0",
        help = "The host to bind"
    )]
    pub bind_host: String,
    #[arg(long, value_name = "DB_PATH", help = "Repeatable; paths of file-based databases [format: \"dbFilePath[::configFilePath]\"]", num_args = 0..)]
    pub db: Vec<String>,
    #[arg(long, value_name = "MEM_DB", help = "Repeatable; config for memory-based databases [format: \"ID[::configFilePath]\"]", num_args = 0..)]
    pub mem_db: Vec<String>,
    #[arg(
        short,
        long,
        value_name = "PORT",
        default_value = "12321",
        help = "Port for the web service"
    )]
    pub port: u16,
    #[arg(
        long,
        value_name = "DIR",
        help = "A directory to serve with builtin HTTP server"
    )]
    pub serve_dir: Option<String>,
    #[arg(
        long,
        value_name = "FILE",
        help = "If --serve-dir is configured, the file to treat as index.",
        default_value = "index.html"
    )]
    pub index_file: String,
}

pub fn parse_cli() -> AppConfig {
    let mut ret = AppConfig::parse();

    assert(
        ret.db.len() + ret.mem_db.len() > 0 || ret.serve_dir.is_some(),
        "no database and no dir to serve specified".to_string(),
    );

    if let Some(sd) = ret.serve_dir {
        let sd = resolve_tilde(&sd);
        assert(
            is_dir(&sd),
            format!("directory to serve does not exist: {}", sd),
        );
        ret.serve_dir = Some(sd.to_owned());
    }

    ret
}

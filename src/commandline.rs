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

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(
    help_template = "{name} {version}\n {about-section}\n {usage-heading} {usage}\n {all-args} {tab}"
)]
pub struct AppConfig {
    #[arg(
        short,
        long,
        value_name = "HOST",
        default_value = "0.0.0.0",
        help = "The host to bind"
    )]
    pub bind_host: String,
    #[arg(short, long, value_name = "DB_PATH", help = "Repeatable; paths of file-based databases", num_args = 0..)]
    pub db: Vec<String>,
    #[arg(short, long, value_name = "MEM_DB", help = "Repeatable; config for memory-based databases (format: ID[:configFilePath])", num_args = 0..)]
    pub mem_db: Vec<String>,
    #[arg(
        short,
        long,
        value_name = "PORT",
        default_value = "12321",
        help = "Port for the web service"
    )]
    pub port: i32,
    #[arg(
        short,
        long,
        value_name = "DIR",
        help = "A directory to serve with builtin HTTP server"
    )]
    pub serve_dir: Option<String>,
}

pub fn parse_cli() -> AppConfig {
    AppConfig::parse()
}

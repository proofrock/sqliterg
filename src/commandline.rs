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

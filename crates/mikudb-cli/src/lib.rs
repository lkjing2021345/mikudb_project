pub mod cli;
pub mod repl;
pub mod highlighter;
pub mod completer;
pub mod formatter;
pub mod client;

pub use cli::Cli;
pub use repl::Repl;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub database: Option<String>,
    pub format: String,
    pub color: bool,
    pub quiet: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 3939,
            user: "miku".to_string(),
            password: "mikumiku3939".to_string(),
            database: None,
            format: "table".to_string(),
            color: true,
            quiet: false,
        }
    }
}

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Server error: {0}")]
    Server(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Interrupted")]
    Interrupted,

    #[error("{0}")]
    Other(String),
}

pub type CliResult<T> = Result<T, CliError>;

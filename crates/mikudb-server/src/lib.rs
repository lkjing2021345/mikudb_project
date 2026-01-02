pub mod config;
pub mod server;
pub mod network;
pub mod protocol;
pub mod handler;
pub mod auth;
pub mod session;

#[cfg(target_os = "linux")]
pub mod openeuler;

#[cfg(feature = "tls")]
pub mod tls;

pub use config::ServerConfig;
pub use server::Server;
pub use session::{Session, SessionManager};
pub use auth::{UserManager, Privilege, RoleAssignment};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(#[from] mikudb_storage::StorageError),

    #[error("Query error: {0}")]
    Query(#[from] mikudb_query::QueryError),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(u64),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ServerResult<T> = Result<T, ServerError>;

pub fn init_logging(level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true))
        .with(filter)
        .init();
}

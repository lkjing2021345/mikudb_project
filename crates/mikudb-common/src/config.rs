use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub wal_dir: PathBuf,
    pub page_size: usize,
    pub cache_size: usize,
    pub compression: CompressionType,
    pub sync_writes: bool,
    pub max_open_files: i32,
    pub write_buffer_size: usize,
    pub max_write_buffer_number: i32,
    #[cfg(target_os = "linux")]
    pub use_direct_io: bool,
    #[cfg(target_os = "linux")]
    pub use_io_uring: bool,
    #[cfg(target_os = "linux")]
    pub use_huge_pages: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("/var/lib/mikudb/data"),
            wal_dir: PathBuf::from("/var/lib/mikudb/wal"),
            page_size: 16 * 1024,
            cache_size: 1024 * 1024 * 1024,
            compression: CompressionType::Lz4,
            sync_writes: true,
            max_open_files: 10000,
            write_buffer_size: 64 * 1024 * 1024,
            max_write_buffer_number: 4,
            #[cfg(target_os = "linux")]
            use_direct_io: true,
            #[cfg(target_os = "linux")]
            use_io_uring: false,
            #[cfg(target_os = "linux")]
            use_huge_pages: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Lz4,
    Zstd,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Lz4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind: String,
    pub port: u16,
    pub unix_socket: Option<PathBuf>,
    pub max_connections: usize,
    pub timeout_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".to_string(),
            port: 3939,
            unix_socket: Some(PathBuf::from("/var/run/mikudb/mikudb.sock")),
            max_connections: 10000,
            timeout_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub default_user: String,
    pub default_password: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_user: "miku".to_string(),
            default_password: "mikumiku3939".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MikuConfig {
    pub storage: StorageConfig,
    pub server: ServerConfig,
    pub auth: AuthConfig,
}

impl Default for MikuConfig {
    fn default() -> Self {
        Self {
            storage: StorageConfig::default(),
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
        }
    }
}

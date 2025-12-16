use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use crate::ServerError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_bind")]
    pub bind: String,

    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default)]
    pub unix_socket: Option<String>,

    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    #[serde(default)]
    pub storage: StorageConfig,

    #[serde(default)]
    pub auth: AuthConfig,

    #[serde(default)]
    pub tls: TlsConfig,

    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub openeuler: OpenEulerConfig,
}

fn default_bind() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 3939 }
fn default_data_dir() -> PathBuf { PathBuf::from("./data") }
fn default_max_connections() -> usize { 10000 }
fn default_timeout() -> u64 { 30000 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    #[serde(default = "default_page_size")]
    pub page_size: usize,

    #[serde(default = "default_cache_size")]
    pub cache_size: String,

    #[serde(default = "default_compression")]
    pub compression: String,

    #[serde(default)]
    pub wal_dir: Option<PathBuf>,

    #[serde(default = "default_sync_writes")]
    pub sync_writes: bool,
}

fn default_page_size() -> usize { 16384 }
fn default_cache_size() -> String { "1GB".to_string() }
fn default_compression() -> String { "lz4".to_string() }
fn default_sync_writes() -> bool { false }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default = "default_auth_enabled")]
    pub enabled: bool,

    #[serde(default = "default_user")]
    pub default_user: String,

    #[serde(default = "default_password")]
    pub default_password: String,
}

fn default_auth_enabled() -> bool { true }
fn default_user() -> String { "miku".to_string() }
fn default_password() -> String { "mikumiku3939".to_string() }

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: default_auth_enabled(),
            default_user: default_user(),
            default_password: default_password(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub cert_file: Option<PathBuf>,

    #[serde(default)]
    pub key_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default)]
    pub file: Option<PathBuf>,

    #[serde(default = "default_rotation")]
    pub rotation: String,

    #[serde(default = "default_max_files")]
    pub max_files: usize,
}

fn default_log_level() -> String { "info".to_string() }
fn default_rotation() -> String { "daily".to_string() }
fn default_max_files() -> usize { 7 }

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file: None,
            rotation: default_rotation(),
            max_files: default_max_files(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenEulerConfig {
    #[serde(default)]
    pub enable_huge_pages: bool,

    #[serde(default)]
    pub huge_pages_size_mb: usize,

    #[serde(default)]
    pub enable_numa: bool,

    #[serde(default)]
    pub numa_node: Option<usize>,

    #[serde(default)]
    pub enable_io_uring: bool,

    #[serde(default)]
    pub cpu_affinity: Vec<usize>,

    #[serde(default)]
    pub enable_direct_io: bool,

    #[serde(default = "default_tcp_cork")]
    pub tcp_cork: bool,

    #[serde(default = "default_tcp_nodelay")]
    pub tcp_nodelay: bool,
}

fn default_tcp_cork() -> bool { true }
fn default_tcp_nodelay() -> bool { true }

impl Default for OpenEulerConfig {
    fn default() -> Self {
        Self {
            enable_huge_pages: false,
            huge_pages_size_mb: 0,
            enable_numa: false,
            numa_node: None,
            enable_io_uring: false,
            cpu_affinity: vec![],
            enable_direct_io: false,
            tcp_cork: default_tcp_cork(),
            tcp_nodelay: default_tcp_nodelay(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: default_bind(),
            port: default_port(),
            unix_socket: None,
            data_dir: default_data_dir(),
            max_connections: default_max_connections(),
            timeout_ms: default_timeout(),
            storage: StorageConfig::default(),
            auth: AuthConfig::default(),
            tls: TlsConfig::default(),
            log: LogConfig::default(),
            openeuler: OpenEulerConfig::default(),
        }
    }
}

impl ServerConfig {
    pub fn from_file(path: &Path) -> Result<Self, ServerError> {
        let content = fs::read_to_string(path)
            .map_err(|e| ServerError::Config(format!("Failed to read config: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| ServerError::Config(format!("Failed to parse config: {}", e)))
    }

    pub fn to_toml(&self) -> Result<String, ServerError> {
        toml::to_string_pretty(self)
            .map_err(|e| ServerError::Config(format!("Failed to serialize config: {}", e)))
    }

    pub fn parse_cache_size(&self) -> usize {
        let s = self.storage.cache_size.to_uppercase();
        let (num, mult) = if s.ends_with("GB") {
            (s.trim_end_matches("GB").trim(), 1024 * 1024 * 1024)
        } else if s.ends_with("MB") {
            (s.trim_end_matches("MB").trim(), 1024 * 1024)
        } else if s.ends_with("KB") {
            (s.trim_end_matches("KB").trim(), 1024)
        } else {
            (s.as_str(), 1)
        };
        num.parse::<usize>().unwrap_or(1024 * 1024 * 1024) * mult
    }
}

//! 服务器配置模块
//!
//! 本模块定义了 MikuDB 服务器的所有配置选项:
//! - 服务器网络配置(绑定地址、端口、Unix Socket)
//! - 存储引擎配置(页大小、缓存、压缩)
//! - 认证配置(用户、密码)
//! - TLS 加密配置
//! - 日志配置
//! - OpenEuler 系统优化配置(NUMA, io_uring, Direct I/O)
//!
//! 支持从 TOML 文件加载配置。

use crate::ServerError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 服务器主配置
///
/// 包含服务器运行所需的所有配置项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// 绑定地址 (默认: 0.0.0.0)
    #[serde(default = "default_bind")]
    pub bind: String,

    /// 端口号 (默认: 3939)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Unix Socket 路径 (Linux 上可用)
    #[serde(default)]
    pub unix_socket: Option<String>,

    /// 数据存储目录 (默认: ./data)
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// 最大并发连接数 (默认: 10000)
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// 连接超时时间(毫秒) (默认: 30000)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,

    /// 存储引擎配置
    #[serde(default)]
    pub storage: StorageConfig,

    /// 认证配置
    #[serde(default)]
    pub auth: AuthConfig,

    /// TLS 配置
    #[serde(default)]
    pub tls: TlsConfig,

    /// 日志配置
    #[serde(default)]
    pub log: LogConfig,

    /// OpenEuler 系统优化配置
    #[serde(default)]
    pub openeuler: OpenEulerConfig,
}

fn default_bind() -> String { "0.0.0.0".to_string() }
fn default_port() -> u16 { 3939 }
fn default_data_dir() -> PathBuf { PathBuf::from("./data") }
fn default_max_connections() -> usize { 10000 }
fn default_timeout() -> u64 { 30000 }

/// 存储引擎配置
///
/// RocksDB 存储引擎的详细配置项。
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

/// 认证配置
///
/// 用户认证相关配置。
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

/// TLS/SSL 配置
///
/// HTTPS/TLS 加密连接配置。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// 是否启用 TLS (默认: false)
    #[serde(default)]
    pub enabled: bool,

    /// 服务器证书文件路径 (PEM 格式)
    #[serde(default)]
    pub cert_file: Option<PathBuf>,

    /// 服务器私钥文件路径 (PEM 格式)
    #[serde(default)]
    pub key_file: Option<PathBuf>,

    /// CA 证书文件路径 (用于客户端证书验证,可选)
    #[serde(default)]
    pub ca_file: Option<PathBuf>,

    /// 是否要求客户端证书认证 (默认: false)
    #[serde(default)]
    pub require_client_cert: bool,

    /// TLS 最低协议版本 (默认: "TLS1.2")
    #[serde(default = "default_tls_min_version")]
    pub min_protocol_version: String,

    /// TLS 最高协议版本 (默认: "TLS1.3")
    #[serde(default = "default_tls_max_version")]
    pub max_protocol_version: String,
}

fn default_tls_min_version() -> String {
    "TLS1.2".to_string()
}

fn default_tls_max_version() -> String {
    "TLS1.3".to_string()
}

impl TlsConfig {
    /// 验证 TLS 配置是否有效
    pub fn validate(&self) -> Result<(), ServerError> {
        if !self.enabled {
            return Ok(());
        }

        if self.cert_file.is_none() {
            return Err(ServerError::Config(
                "TLS enabled but cert_file not specified".to_string(),
            ));
        }

        if self.key_file.is_none() {
            return Err(ServerError::Config(
                "TLS enabled but key_file not specified".to_string(),
            ));
        }

        // 检查证书文件是否存在
        if let Some(ref cert) = self.cert_file {
            if !cert.exists() {
                return Err(ServerError::Config(format!(
                    "Certificate file not found: {}",
                    cert.display()
                )));
            }
        }

        // 检查私钥文件是否存在
        if let Some(ref key) = self.key_file {
            if !key.exists() {
                return Err(ServerError::Config(format!(
                    "Private key file not found: {}",
                    key.display()
                )));
            }
        }

        // 检查 CA 文件是否存在(如果需要客户端证书)
        if self.require_client_cert {
            if let Some(ref ca) = self.ca_file {
                if !ca.exists() {
                    return Err(ServerError::Config(format!(
                        "CA file not found: {}",
                        ca.display()
                    )));
                }
            } else {
                return Err(ServerError::Config(
                    "require_client_cert is true but ca_file not specified".to_string(),
                ));
            }
        }

        Ok(())
    }
}

/// 日志配置
///
/// 日志级别、输出文件和轮转策略。
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

/// OpenEuler 系统优化配置
///
/// Linux 上的性能优化选项,包括 NUMA, io_uring, Direct I/O 等。
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
    /// # Brief
    /// 从 TOML 文件加载配置
    ///
    /// # Arguments
    /// * `path` - 配置文件路径
    ///
    /// # Returns
    /// 解析后的配置实例
    pub fn from_file(path: &Path) -> Result<Self, ServerError> {
        // 读取文件内容
        let content = fs::read_to_string(path)
            .map_err(|e| ServerError::Config(format!("Failed to read config: {}", e)))?;

        // 使用 TOML 解析器
        toml::from_str(&content)
            .map_err(|e| ServerError::Config(format!("Failed to parse config: {}", e)))
    }

    /// # Brief
    /// 将配置序列化为 TOML 字符串
    ///
    /// # Returns
    /// TOML 格式的配置字符串
    pub fn to_toml(&self) -> Result<String, ServerError> {
        toml::to_string_pretty(self)
            .map_err(|e| ServerError::Config(format!("Failed to serialize config: {}", e)))
    }

    /// # Brief
    /// 解析缓存大小字符串
    ///
    /// 支持 GB/MB/KB 后缀,例如 "1GB", "512MB"。
    ///
    /// # Returns
    /// 缓存大小(字节数)
    pub fn parse_cache_size(&self) -> usize {
        let s = self.storage.cache_size.to_uppercase();
        // 判断单位后缀
        let (num, mult) = if s.ends_with("GB") {
            (s.trim_end_matches("GB").trim(), 1024 * 1024 * 1024)
        } else if s.ends_with("MB") {
            (s.trim_end_matches("MB").trim(), 1024 * 1024)
        } else if s.ends_with("KB") {
            (s.trim_end_matches("KB").trim(), 1024)
        } else {
            (s.as_str(), 1)  // 无后缀默认为字节
        };
        // 解析数字并乘以单位,失败则使用默认值 1GB
        num.parse::<usize>().unwrap_or(1024 * 1024 * 1024) * mult
    }
}

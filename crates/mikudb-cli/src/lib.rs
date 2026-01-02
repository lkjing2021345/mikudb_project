//! MikuDB CLI 库
//!
//! 本模块提供 MikuDB 命令行客户端的核心功能:
//! - 交互式 REPL 环境
//! - 非交互式命令执行
//! - 语法高亮和自动补全
//! - 多种输出格式(Table, JSON, CSV, Line)
//! - 连接管理和认证

pub mod cli;
pub mod repl;
pub mod highlighter;
pub mod completer;
pub mod formatter;
pub mod client;

pub use cli::Cli;
pub use repl::Repl;

use thiserror::Error;

/// CLI 配置
///
/// 包含连接参数、认证信息和输出选项。
#[derive(Debug, Clone)]
pub struct Config {
    /// 服务器主机名
    pub host: String,
    /// 服务器端口
    pub port: u16,
    /// 用户名
    pub user: String,
    /// 密码
    pub password: String,
    /// 默认数据库
    pub database: Option<String>,
    /// 输出格式("table", "json", "csv", "line")
    pub format: String,
    /// 是否启用颜色
    pub color: bool,
    /// 是否静默模式
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

/// CLI 错误类型
///
/// 定义所有可能的错误情况。
#[derive(Error, Debug)]
pub enum CliError {
    /// 连接错误
    #[error("Connection error: {0}")]
    Connection(String),

    /// 认证失败
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// 查询错误
    #[error("Query error: {0}")]
    Query(String),

    /// 服务器错误
    #[error("Server error: {0}")]
    Server(String),

    /// IO 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 解析错误
    #[error("Parse error: {0}")]
    Parse(String),

    /// 用户中断(Ctrl+C)
    #[error("Interrupted")]
    Interrupted,

    /// 其他错误
    #[error("{0}")]
    Other(String),
}

/// CLI 结果类型
pub type CliResult<T> = Result<T, CliError>;

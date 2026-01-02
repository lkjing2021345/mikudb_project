//! 错误类型定义模块
//!
//! 定义 MikuDB 的统一错误类型 MikuError 和 Result 别名。

use thiserror::Error;

/// MikuDB 错误类型
///
/// 包含所有可能的错误情况。
#[derive(Error, Debug)]
pub enum MikuError {
    /// I/O 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 序列化错误
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// 反序列化错误
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// 存储层错误
    #[error("Storage error: {0}")]
    Storage(String),

    /// 索引错误
    #[error("Index error: {0}")]
    Index(String),

    /// 查询错误
    #[error("Query error: {0}")]
    Query(String),

    /// 事务错误
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// 文档不存在
    #[error("Document not found: {0}")]
    NotFound(String),

    /// 文档已存在
    #[error("Document already exists: {0}")]
    AlreadyExists(String),

    /// BOML 格式错误
    #[error("Invalid BOML: {0}")]
    InvalidBoml(String),

    /// 类型不匹配
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },

    /// ObjectId 无效
    #[error("Invalid ObjectId: {0}")]
    InvalidObjectId(String),

    /// 验证错误
    #[error("Validation error: {0}")]
    Validation(String),

    /// 权限不足
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// 连接错误
    #[error("Connection error: {0}")]
    Connection(String),

    /// 超时
    #[error("Timeout: {0}")]
    Timeout(String),

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),

    /// 平台相关错误
    #[error("Platform error: {0}")]
    Platform(String),
}

/// MikuDB Result 类型别名
pub type MikuResult<T> = Result<T, MikuError>;

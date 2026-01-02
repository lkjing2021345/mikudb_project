//! 集群错误类型定义

use thiserror::Error;

/// 集群操作错误
#[derive(Error, Debug)]
pub enum ClusterError {
    /// Raft 共识错误
    #[error("Raft error: {0}")]
    Raft(String),

    /// 复制错误
    #[error("Replication error: {0}")]
    Replication(String),

    /// 网络错误
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    /// 配置错误
    #[error("Configuration error: {0}")]
    Config(String),

    /// 节点未找到
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    /// 超时错误
    #[error("Timeout: {0}")]
    Timeout(String),

    /// 序列化错误
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 集群操作结果类型
pub type ClusterResult<T> = Result<T, ClusterError>;

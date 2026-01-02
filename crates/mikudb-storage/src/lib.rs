//! 存储层模块
//!
//! 本模块提供 MikuDB 的底层存储功能:
//! - **StorageEngine**: 基于 RocksDB 的存储引擎
//! - **Collection**: 文档集合管理
//! - **WAL**: 预写式日志,保证持久性和崩溃恢复
//! - **Cache**: LRU 缓存系统(文档缓存、查询缓存)
//! - **Compaction**: LSM-tree 压缩配置和统计
//!
//! # OpenEuler 适配亮点
//!
//! - **Direct I/O 优化**: 减少内存拷贝,提升 I/O 性能
//! - **鲲鹏 CPU 优化**: 自动检测并调整写缓冲区大小
//! - **NUMA 感知**: 支持多 NUMA 节点的内存分配优化
//! - **ARM64 优化**: 针对 ARM64 架构的块大小配置

pub mod engine;
pub mod collection;
pub mod wal;
pub mod cache;
pub mod compaction;

pub use collection::Collection;
pub use engine::{StorageEngine, StorageOptions};

use thiserror::Error;

/// 存储层错误类型
///
/// 定义所有存储操作可能产生的错误。
#[derive(Error, Debug)]
pub enum StorageError {
    /// I/O 错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "rocksdb")]
    /// RocksDB 错误
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    /// BOML 编解码错误
    #[error("BOML error: {0}")]
    Boml(#[from] mikudb_boml::BomlError),

    /// 集合不存在
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    /// 集合已存在
    #[error("Collection already exists: {0}")]
    CollectionExists(String),

    /// 文档不存在
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// 文档已存在
    #[error("Document already exists: {0}")]
    DocumentExists(String),

    /// 无效的键
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// 数据损坏
    #[error("Corruption: {0}")]
    Corruption(String),

    /// 事务错误
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// 写冲突(MVCC)
    #[error("Write conflict")]
    WriteConflict,

    /// 存储空间已满
    #[error("Storage full")]
    StorageFull,

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 存储操作结果类型
pub type StorageResult<T> = Result<T, StorageError>;

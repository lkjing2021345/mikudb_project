pub mod engine;
pub mod collection;
pub mod wal;
pub mod cache;
pub mod compaction;

pub use engine::{StorageEngine, StorageOptions};
pub use collection::Collection;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "rocksdb")]
    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    #[error("BOML error: {0}")]
    Boml(#[from] mikudb_boml::BomlError),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Collection already exists: {0}")]
    CollectionExists(String),

    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    #[error("Document already exists: {0}")]
    DocumentExists(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Corruption: {0}")]
    Corruption(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Write conflict")]
    WriteConflict,

    #[error("Storage full")]
    StorageFull,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type StorageResult<T> = Result<T, StorageError>;

//! # BOML - Binary Object Markup Language
//!
//! BOML 是 MikuDB 的自研二进制文档格式，对标 MongoDB 的 BSON。
//! 相比 BSON，BOML 具有以下优势：
//!
//! - **更紧凑的编码**：小整数、短字符串使用特殊标记，减少存储空间
//! - **更快的解析**：使用变长整数编码，减少内存拷贝
//! - **校验和支持**：内置 xxHash3 校验，保证数据完整性
//! - **Serde 集成**：完整支持 Rust 的 Serde 序列化框架
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use mikudb_boml::{Document, BomlValue, encode_to_vec, decode};
//!
//! // 创建文档
//! let mut doc = Document::new();
//! doc.insert("name", "MikuDB");
//! doc.insert("version", 1);
//!
//! // 序列化
//! let bytes = encode_to_vec(&doc.to_boml_value()).unwrap();
//!
//! // 反序列化
//! let value = decode(&bytes).unwrap();
//! ```
//!
//! ## OpenEuler 适配亮点
//!
//! - 使用 xxHash3 进行校验和计算，在 ARM64 (鲲鹏) 上有优秀性能
//! - 内存对齐优化，适配 OpenEuler 的内存分配器

pub mod value;
pub mod document;
pub mod codec;
pub mod ser;
pub mod de;
pub mod spec;

pub use codec::{decode, encode, encode_to_vec};
pub use document::Document;
pub use value::BomlValue;

use thiserror::Error;

/// BOML 操作的错误类型
///
/// 包含序列化、反序列化过程中可能出现的所有错误情况
#[derive(Error, Debug)]
pub enum BomlError {
    /// IO 操作错误
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 无效的类型标记字节
    #[error("Invalid type marker: {0}")]
    InvalidTypeMarker(u8),

    /// 字符串不是有效的 UTF-8 编码
    #[error("Invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),

    /// 缓冲区空间不足
    #[error("Buffer too small: need {need}, have {have}")]
    BufferTooSmall { need: usize, have: usize },

    /// 意外的输入结束
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// 文档格式无效
    #[error("Invalid document: {0}")]
    InvalidDocument(String),

    /// 嵌套层级过深
    #[error("Nesting too deep: max {0}")]
    NestingTooDeep(usize),

    /// 文档体积超出限制
    #[error("Document too large: max {0} bytes")]
    DocumentTooLarge(usize),

    /// ObjectId 格式无效
    #[error("Invalid ObjectId")]
    InvalidObjectId,

    /// 序列化过程错误
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// 反序列化过程错误
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

/// BOML 操作的 Result 类型别名
pub type BomlResult<T> = Result<T, BomlError>;

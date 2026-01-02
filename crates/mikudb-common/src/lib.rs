//! MikuDB 公共模块
//!
//! 提供 MikuDB 各组件共享的类型、错误定义和平台抽象:
//! - **类型**: ObjectId, DocumentId, CollectionName, DatabaseName, Timestamp
//! - **错误**: 统一的错误类型和 Result 别名
//! - **配置**: 压缩类型等配置选项
//! - **平台**: 平台检测和 OpenEuler 优化配置

pub mod error;
pub mod types;
pub mod config;
pub mod platform;

pub use error::{MikuError, MikuResult};
pub use types::*;

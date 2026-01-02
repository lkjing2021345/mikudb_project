//! MikuDB 查询模块
//!
//! 本模块实现 MQL (Miku Query Language) 查询引擎:
//! - 词法分析和语法解析
//! - AST (抽象语法树)
//! - 查询计划和优化
//! - 查询执行器
//! - 过滤器和索引
//!
//! MQL 支持:
//! - CRUD 操作 (FIND, INSERT, UPDATE, DELETE)
//! - DDL 操作 (CREATE/DROP COLLECTION/INDEX)
//! - 聚合管道 (AGGREGATE)
//! - 事务 (BEGIN/COMMIT/ROLLBACK)
//! - 用户管理 (CREATE USER, GRANT, REVOKE)

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod planner;
pub mod executor;
pub mod filter;
pub mod index;

pub use ast::*;
pub use executor::{QueryExecutor, QueryResponse};
pub use parser::Parser;

use thiserror::Error;

/// 查询错误类型
///
/// 定义查询处理过程中可能出现的所有错误。
#[derive(Error, Debug)]
pub enum QueryError {
    /// 语法错误
    #[error("Syntax error: {0}")]
    Syntax(String),

    /// 解析错误(带位置信息)
    #[error("Parse error at {position}: {message}")]
    Parse { position: usize, message: String },

    /// 未知关键字
    #[error("Unknown keyword: {0}")]
    UnknownKeyword(String),

    /// 无效字段路径
    #[error("Invalid field path: {0}")]
    InvalidFieldPath(String),

    /// 类型错误
    #[error("Type error: {0}")]
    TypeError(String),

    /// 无效操作符
    #[error("Invalid operator: {0}")]
    InvalidOperator(String),

    /// 集合不存在
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    /// 索引不存在
    #[error("Index not found: {0}")]
    IndexNotFound(String),

    /// 执行错误
    #[error("Execution error: {0}")]
    Execution(String),

    /// 存储层错误
    #[error("Storage error: {0}")]
    Storage(#[from] mikudb_storage::StorageError),

    /// BOML 编解码错误
    #[error("BOML error: {0}")]
    Boml(#[from] mikudb_boml::BomlError),

    /// 查询超时
    #[error("Timeout")]
    Timeout,

    /// 内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

/// 查询结果类型
pub type QueryResult<T> = Result<T, QueryError>;

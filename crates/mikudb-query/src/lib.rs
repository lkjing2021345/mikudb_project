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

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Syntax error: {0}")]
    Syntax(String),

    #[error("Parse error at {position}: {message}")]
    Parse { position: usize, message: String },

    #[error("Unknown keyword: {0}")]
    UnknownKeyword(String),

    #[error("Invalid field path: {0}")]
    InvalidFieldPath(String),

    #[error("Type error: {0}")]
    TypeError(String),

    #[error("Invalid operator: {0}")]
    InvalidOperator(String),

    #[error("Collection not found: {0}")]
    CollectionNotFound(String),

    #[error("Index not found: {0}")]
    IndexNotFound(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Storage error: {0}")]
    Storage(#[from] mikudb_storage::StorageError),

    #[error("BOML error: {0}")]
    Boml(#[from] mikudb_boml::BomlError),

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type QueryResult<T> = Result<T, QueryError>;

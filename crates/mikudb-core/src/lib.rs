//! MikuDB 核心库
//!
//! 提供 MikuDB 的高级 API 和核心功能:
//! - **Database**: 数据库和集合管理
//! - **Transaction**: 事务和会话管理
//! - **Client**: 异步客户端 API
//! - **Cursor**: 查询结果游标
//! - **Pipeline**: 聚合管道构建器
//! - **Connection**: 连接字符串解析和选项
//! - **Builder**: 流式构建器模式
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use mikudb_core::{Client, ClientOptions};
//!
//! let client = Client::connect("mikudb://localhost:3939").await?;
//! let db = client.database("test");
//! let coll = db.collection("users");
//!
//! coll.insert_one(doc! { "name": "Miku", "age": 16 }).await?;
//! ```

pub mod database;
pub mod transaction;
pub mod client;
pub mod builder;
pub mod connection;
pub mod cursor;
pub mod pipeline;

pub use mikudb_boml as boml;
pub use mikudb_common as common;
pub use mikudb_query as query;
pub use mikudb_storage as storage;

pub use builder::{DatabaseBuilder, StorageOptionsBuilder};
pub use client::{AsyncCollection, AsyncDatabase, Client, ClientOptions};
pub use connection::{
    AuthMechanism, ConnectionMode, ConnectionOptions,
    ConnectionString, Credentials, Host, ReadConcern,
    ReadPreference, TlsOptions, WriteConcern,
};
pub use cursor::{Cursor, CursorBuilder, CursorInfo, CursorIterator, CursorManager, CursorOptions};
pub use database::{Collection, Database, DatabaseStats};
pub use pipeline::{GroupBuilder, LookupBuilder, MatchBuilder, Pipeline, ProjectBuilder, SortBuilder};
pub use transaction::{
    IsolationLevel, Session, SessionManager, Transaction,
    TransactionOptions, TransactionState,
};

pub use boml::{BomlValue, Document};
pub use common::{MikuError, MikuResult, ObjectId};
pub use query::{Parser, QueryExecutor, QueryResponse, Statement};
pub use storage::{StorageEngine, StorageOptions};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DEFAULT_PORT: u16 = 3939;
pub const DEFAULT_USER: &str = "miku";
pub const DEFAULT_PASSWORD: &str = "mikumiku3939";

pub fn init_logging() {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(filter)
        .init();
}

pub fn print_banner() {
    println!(r#"
  __  __ _ _          ____  ____
 |  \/  (_) | ___   _|  _ \| __ )
 | |\/| | | |/ / | | | | | |  _ \
 | |  | | |   <| |_| | |_| | |_) |
 |_|  |_|_|_|\_\\__,_|____/|____/

 MikuDB v{} - High-performance Document Database
 Optimized for OpenEuler
 Default port: {} | User: {}
"#, VERSION, DEFAULT_PORT, DEFAULT_USER);
}

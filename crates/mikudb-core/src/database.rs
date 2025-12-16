//! MikuDB 核心模块
//!
//! 提供 MikuDB 的高级 API，包括数据库操作、集合管理和查询执行。
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use mikudb_core::{Database, Document};
//!
//! let db = Database::open("mydb", "/var/lib/mikudb/data")?;
//! db.execute("CREATE COLLECTION users")?;
//!
//! let collection = db.collection("users")?;
//! let mut doc = Document::new();
//! doc.insert("name", "Miku");
//! collection.insert(&mut doc)?;
//! ```

use crate::query::{Parser, QueryExecutor, QueryResponse, Statement};
use crate::storage::{StorageEngine, StorageOptions};
use crate::transaction::{Session, SessionManager};
use mikudb_common::{MikuError, MikuResult};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// 数据库实例
///
/// MikuDB 的主要入口点，封装了存储引擎和查询执行器
pub struct Database {
    name: String,
    storage: Arc<StorageEngine>,
    executor: QueryExecutor,
    session_manager: SessionManager,
}

impl Database {
    /// 打开数据库
    ///
    /// # Brief
    /// 使用默认配置打开或创建数据库
    ///
    /// # Arguments
    /// * `name` - 数据库名称
    /// * `data_dir` - 数据存储目录
    ///
    /// # Returns
    /// 成功返回 Database 实例
    pub fn open(name: impl Into<String>, data_dir: impl AsRef<Path>) -> MikuResult<Self> {
        let name = name.into();
        let data_path = data_dir.as_ref().join(&name);

        let options = StorageOptions {
            data_dir: data_path,
            ..StorageOptions::default()
        };

        Self::open_with_options(name, options)
    }

    /// 使用自定义配置打开数据库
    ///
    /// # Brief
    /// 使用指定的 StorageOptions 打开数据库
    ///
    /// # Arguments
    /// * `name` - 数据库名称
    /// * `options` - 存储引擎配置
    ///
    /// # Returns
    /// 成功返回 Database 实例
    pub fn open_with_options(name: impl Into<String>, options: StorageOptions) -> MikuResult<Self> {
        let name = name.into();

        info!("Opening database: {}", name);

        let storage = StorageEngine::open(options)
            .map_err(|e| MikuError::Storage(e.to_string()))?;

        let storage = Arc::new(storage);
        let executor = QueryExecutor::new(storage.clone());

        let session_manager = SessionManager::new(storage.clone());

        Ok(Self {
            name,
            storage,
            executor,
            session_manager,
        })
    }

    pub(crate) fn open_with_storage(name: String, storage: Arc<StorageEngine>) -> Self {
        let executor = QueryExecutor::new(storage.clone());
        let session_manager = SessionManager::new(storage.clone());

        Self {
            name,
            storage,
            executor,
            session_manager,
        }
    }

    /// 使用 OpenEuler 优化配置打开数据库
    ///
    /// # Brief
    /// 使用针对 OpenEuler 平台优化的配置打开数据库
    ///
    /// # OpenEuler 适配亮点
    /// - 自动检测鲲鹏 CPU 并优化参数
    /// - 启用 Direct I/O 提升性能
    ///
    /// # Arguments
    /// * `name` - 数据库名称
    /// * `data_dir` - 数据存储目录
    ///
    /// # Returns
    /// 成功返回 Database 实例
    pub fn open_for_openeuler(name: impl Into<String>, data_dir: impl AsRef<Path>) -> MikuResult<Self> {
        let name = name.into();
        let data_path = data_dir.as_ref().join(&name);

        let mut options = StorageOptions::for_openeuler();
        options.data_dir = data_path;

        Self::open_with_options(name, options)
    }

    /// 获取数据库名称
    ///
    /// # Brief
    /// 返回数据库的名称
    ///
    /// # Returns
    /// 数据库名称的字符串切片
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 执行 MQL 查询
    ///
    /// # Brief
    /// 解析并执行 MQL 查询字符串
    ///
    /// # Arguments
    /// * `query` - MQL 查询字符串
    ///
    /// # Returns
    /// 查询结果 QueryResponse
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = db.execute("FIND users WHERE age > 18")?;
    /// ```
    pub fn execute(&self, query: &str) -> MikuResult<QueryResponse> {
        debug!("Executing query: {}", query);

        let stmt = Parser::parse(query)
            .map_err(|e| MikuError::Query(e.to_string()))?;

        self.executor
            .execute(&stmt)
            .map_err(|e| MikuError::Query(e.to_string()))
    }

    /// 执行已解析的语句
    ///
    /// # Brief
    /// 直接执行 Statement 对象，跳过解析步骤
    ///
    /// # Arguments
    /// * `stmt` - 已解析的 Statement
    ///
    /// # Returns
    /// 查询结果 QueryResponse
    pub fn execute_statement(&self, stmt: &Statement) -> MikuResult<QueryResponse> {
        self.executor
            .execute(stmt)
            .map_err(|e| MikuError::Query(e.to_string()))
    }

    /// 创建集合
    ///
    /// # Brief
    /// 创建新的文档集合
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// 成功返回 Ok(()), 如果集合已存在则返回错误
    pub fn create_collection(&self, name: &str) -> MikuResult<()> {
        self.storage
            .create_collection(name)
            .map_err(|e| MikuError::Storage(e.to_string()))?;
        Ok(())
    }

    /// 删除集合
    ///
    /// # Brief
    /// 删除指定的集合及其所有文档
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// 成功返回 Ok(()), 如果集合不存在则返回错误
    pub fn drop_collection(&self, name: &str) -> MikuResult<()> {
        self.storage
            .drop_collection(name)
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    /// 列出所有集合
    ///
    /// # Brief
    /// 返回数据库中所有集合的名称
    ///
    /// # Returns
    /// 集合名称列表
    pub fn list_collections(&self) -> MikuResult<Vec<String>> {
        self.storage
            .list_collections()
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    /// 获取集合
    ///
    /// # Brief
    /// 获取指定名称的集合，如果不存在则自动创建
    ///
    /// # Arguments
    /// * `name` - 集合名称
    ///
    /// # Returns
    /// Collection 实例
    pub fn collection(&self, name: &str) -> MikuResult<Collection> {
        let inner = self
            .storage
            .get_or_create_collection(name)
            .map_err(|e| MikuError::Storage(e.to_string()))?;
        Ok(Collection { inner })
    }

    /// 压缩数据库
    ///
    /// # Brief
    /// 触发数据库压缩以回收空间
    ///
    /// # Returns
    /// 成功返回 Ok(())
    pub fn compact(&self) -> MikuResult<()> {
        self.storage
            .compact()
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    /// 刷新数据到磁盘
    ///
    /// # Brief
    /// 确保所有数据已持久化到磁盘
    ///
    /// # Returns
    /// 成功返回 Ok(())
    pub fn flush(&self) -> MikuResult<()> {
        self.storage
            .flush()
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    /// 获取数据库统计信息
    ///
    /// # Brief
    /// 返回数据库的统计信息快照
    ///
    /// # Returns
    /// DatabaseStats 实例
    pub fn stats(&self) -> DatabaseStats {
        DatabaseStats {
            name: self.name.clone(),
            size: self.storage.get_approximate_size(),
            collections: self.list_collections().unwrap_or_default(),
        }
    }

    /// 创建会话
    ///
    /// # Brief
    /// 创建一个新的数据库会话，用于事务操作
    ///
    /// # Returns
    /// 新的 Session Arc 引用
    pub fn start_session(&self) -> Arc<Session> {
        self.session_manager.create_session()
    }

    /// 获取会话管理器
    ///
    /// # Brief
    /// 返回会话管理器的引用
    ///
    /// # Returns
    /// SessionManager 引用
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// 获取存储引擎
    ///
    /// # Brief
    /// 返回存储引擎的 Arc 引用
    ///
    /// # Returns
    /// StorageEngine 的 Arc 引用
    pub fn storage(&self) -> &Arc<StorageEngine> {
        &self.storage
    }
}

/// 集合包装器
///
/// 提供文档集合的高级 API
pub struct Collection {
    inner: Arc<crate::storage::Collection>,
}

impl Collection {
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn insert(&self, doc: &mut crate::boml::Document) -> MikuResult<crate::common::ObjectId> {
        self.inner
            .insert(doc)
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn insert_many(&self, docs: &mut [crate::boml::Document]) -> MikuResult<Vec<crate::common::ObjectId>> {
        self.inner
            .insert_many(docs)
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn find_one(&self, id: &crate::common::ObjectId) -> MikuResult<Option<crate::boml::Document>> {
        self.inner
            .get(id)
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn find_all(&self) -> MikuResult<Vec<crate::boml::Document>> {
        self.inner
            .find_all()
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn update(&self, id: &crate::common::ObjectId, doc: &crate::boml::Document) -> MikuResult<()> {
        self.inner
            .update(id, doc)
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn delete(&self, id: &crate::common::ObjectId) -> MikuResult<bool> {
        self.inner
            .delete(id)
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn count(&self) -> MikuResult<u64> {
        self.inner
            .count()
            .map_err(|e| MikuError::Storage(e.to_string()))
    }

    pub fn clear(&self) -> MikuResult<u64> {
        self.inner
            .clear()
            .map_err(|e| MikuError::Storage(e.to_string()))
    }
}

/// 数据库统计信息
///
/// 包含数据库的基本统计数据
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub name: String,
    pub size: u64,
    pub collections: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_open() {
        let dir = tempdir().unwrap();
        let db = Database::open("test", dir.path()).unwrap();
        assert_eq!(db.name(), "test");
    }

    #[test]
    fn test_execute_query() {
        let dir = tempdir().unwrap();
        let db = Database::open("test", dir.path()).unwrap();

        let result = db.execute("CREATE COLLECTION users").unwrap();
        assert!(db.list_collections().unwrap().contains(&"users".to_string()));
    }

    #[test]
    fn test_collection_operations() {
        let dir = tempdir().unwrap();
        let db = Database::open("test", dir.path()).unwrap();

        let collection = db.collection("test_collection").unwrap();

        let mut doc = crate::boml::Document::new();
        doc.insert("name", "Alice");
        doc.insert("age", 30);

        let id = collection.insert(&mut doc).unwrap();

        let retrieved = collection.find_one(&id).unwrap().unwrap();
        assert_eq!(retrieved.get_str("name"), Some("Alice"));
        assert_eq!(retrieved.get_i32("age"), Some(30));

        assert!(collection.delete(&id).unwrap());
        assert!(collection.find_one(&id).unwrap().is_none());
    }
}

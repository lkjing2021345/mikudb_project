//! 异步客户端模块
//!
//! 提供异步的数据库客户端，支持连接池和自动重连。
//!
//! # 示例
//!
//! ```rust,ignore
//! use mikudb_core::{Client, ClientOptions};
//!
//! #[tokio::main]
//! async fn main() {
//!     let client = Client::connect("mikudb://localhost:3939/mydb").await?;
//!     let db = client.database("mydb");
//!
//!     let result = db.execute("FIND users").await?;
//! }
//! ```

use crate::common::{MikuError, MikuResult};
use crate::query::QueryResponse;
use crate::storage::{StorageEngine, StorageOptions};
use crate::transaction::{Session, SessionManager};
use crate::{Database, DatabaseBuilder};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct ClientOptions {
    pub data_dir: PathBuf,
    pub max_pool_size: usize,
    pub min_pool_size: usize,
    pub connect_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_idle_time: Duration,
    pub retry_writes: bool,
    pub retry_reads: bool,
    pub server_selection_timeout: Duration,
    pub heartbeat_frequency: Duration,
    pub app_name: Option<String>,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("/var/lib/mikudb/data"),
            max_pool_size: 100,
            min_pool_size: 0,
            connect_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(60),
            max_idle_time: Duration::from_secs(300),
            retry_writes: true,
            retry_reads: true,
            server_selection_timeout: Duration::from_secs(30),
            heartbeat_frequency: Duration::from_secs(10),
            app_name: None,
        }
    }
}

impl ClientOptions {
    pub fn parse(uri: &str) -> MikuResult<Self> {
        let mut options = Self::default();

        if uri.starts_with("mikudb://") || uri.starts_with("miku://") {
            let without_scheme = uri
                .strip_prefix("mikudb://")
                .or_else(|| uri.strip_prefix("miku://"))
                .unwrap_or(uri);

            if let Some(path_start) = without_scheme.find('/') {
                let path_part = &without_scheme[path_start + 1..];
                if let Some(query_start) = path_part.find('?') {
                    let query = &path_part[query_start + 1..];
                    for pair in query.split('&') {
                        if let Some((key, value)) = pair.split_once('=') {
                            match key {
                                "maxPoolSize" => {
                                    if let Ok(v) = value.parse() {
                                        options.max_pool_size = v;
                                    }
                                }
                                "minPoolSize" => {
                                    if let Ok(v) = value.parse() {
                                        options.min_pool_size = v;
                                    }
                                }
                                "connectTimeoutMS" => {
                                    if let Ok(v) = value.parse::<u64>() {
                                        options.connect_timeout = Duration::from_millis(v);
                                    }
                                }
                                "retryWrites" => {
                                    options.retry_writes = value == "true";
                                }
                                "retryReads" => {
                                    options.retry_reads = value == "true";
                                }
                                "appName" => {
                                    options.app_name = Some(value.to_string());
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        } else {
            options.data_dir = PathBuf::from(uri);
        }

        Ok(options)
    }

    pub fn builder() -> ClientOptionsBuilder {
        ClientOptionsBuilder::new()
    }
}

pub struct ClientOptionsBuilder {
    options: ClientOptions,
}

impl ClientOptionsBuilder {
    pub fn new() -> Self {
        Self {
            options: ClientOptions::default(),
        }
    }

    pub fn data_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.options.data_dir = path.as_ref().to_path_buf();
        self
    }

    pub fn max_pool_size(mut self, size: usize) -> Self {
        self.options.max_pool_size = size;
        self
    }

    pub fn min_pool_size(mut self, size: usize) -> Self {
        self.options.min_pool_size = size;
        self
    }

    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.options.connect_timeout = timeout;
        self
    }

    pub fn idle_timeout(mut self, timeout: Duration) -> Self {
        self.options.idle_timeout = timeout;
        self
    }

    pub fn retry_writes(mut self, retry: bool) -> Self {
        self.options.retry_writes = retry;
        self
    }

    pub fn retry_reads(mut self, retry: bool) -> Self {
        self.options.retry_reads = retry;
        self
    }

    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.options.app_name = Some(name.into());
        self
    }

    pub fn build(self) -> ClientOptions {
        self.options
    }
}

impl Default for ClientOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Client {
    options: ClientOptions,
    storage: Arc<StorageEngine>,
    databases: RwLock<HashMap<String, Arc<Database>>>,
    session_manager: Arc<SessionManager>,
    pool_semaphore: Arc<Semaphore>,
}

impl Client {
    pub async fn connect(uri: &str) -> MikuResult<Self> {
        let options = ClientOptions::parse(uri)?;
        Self::connect_with_options(options).await
    }

    pub async fn connect_with_options(options: ClientOptions) -> MikuResult<Self> {
        info!("Connecting to MikuDB at {:?}", options.data_dir);

        let storage_options = StorageOptions {
            data_dir: options.data_dir.clone(),
            ..StorageOptions::default()
        };

        let storage = tokio::task::spawn_blocking(move || StorageEngine::open(storage_options))
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
            .map_err(|e| MikuError::Storage(e.to_string()))?;

        let storage = Arc::new(storage);
        let session_manager = Arc::new(SessionManager::new(storage.clone()));
        let pool_semaphore = Arc::new(Semaphore::new(options.max_pool_size));

        Ok(Self {
            options,
            storage,
            databases: RwLock::new(HashMap::new()),
            session_manager,
            pool_semaphore,
        })
    }

    pub fn database(&self, name: &str) -> Arc<Database> {
        let mut databases = self.databases.write();

        if let Some(db) = databases.get(name) {
            return db.clone();
        }

        let db = Arc::new(
            Database::open_with_storage(name.to_string(), self.storage.clone())
        );

        databases.insert(name.to_string(), db.clone());
        db
    }

    pub fn list_database_names(&self) -> MikuResult<Vec<String>> {
        Ok(self.databases.read().keys().cloned().collect())
    }

    pub async fn start_session(&self) -> MikuResult<Arc<Session>> {
        let _permit = self
            .pool_semaphore
            .acquire()
            .await
            .map_err(|_| MikuError::Internal("Pool exhausted".to_string()))?;

        Ok(self.session_manager.create_session())
    }

    pub fn session_manager(&self) -> &Arc<SessionManager> {
        &self.session_manager
    }

    pub async fn execute(&self, db_name: &str, query: &str) -> MikuResult<QueryResponse> {
        let db = self.database(db_name);

        let query = query.to_string();
        tokio::task::spawn_blocking(move || db.execute(&query))
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }

    pub async fn close(&self) -> MikuResult<()> {
        info!("Closing MikuDB client");

        self.session_manager.cleanup_expired_sessions();

        self.storage
            .flush()
            .map_err(|e| MikuError::Storage(e.to_string()))?;

        Ok(())
    }

    pub fn options(&self) -> &ClientOptions {
        &self.options
    }

    pub fn pool_available(&self) -> usize {
        self.pool_semaphore.available_permits()
    }
}

pub struct AsyncDatabase {
    inner: Arc<Database>,
}

impl AsyncDatabase {
    pub fn new(db: Arc<Database>) -> Self {
        Self { inner: db }
    }

    pub async fn execute(&self, query: &str) -> MikuResult<QueryResponse> {
        let db = self.inner.clone();
        let query = query.to_string();

        tokio::task::spawn_blocking(move || db.execute(&query))
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }

    pub async fn create_collection(&self, name: &str) -> MikuResult<()> {
        let db = self.inner.clone();
        let name = name.to_string();

        tokio::task::spawn_blocking(move || db.create_collection(&name))
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }

    pub async fn drop_collection(&self, name: &str) -> MikuResult<()> {
        let db = self.inner.clone();
        let name = name.to_string();

        tokio::task::spawn_blocking(move || db.drop_collection(&name))
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }

    pub async fn list_collections(&self) -> MikuResult<Vec<String>> {
        let db = self.inner.clone();

        tokio::task::spawn_blocking(move || db.list_collections())
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }

    pub fn collection(&self, name: &str) -> MikuResult<AsyncCollection> {
        let collection = self.inner.collection(name)?;
        Ok(AsyncCollection::new(collection))
    }

    pub async fn compact(&self) -> MikuResult<()> {
        let db = self.inner.clone();

        tokio::task::spawn_blocking(move || db.compact())
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }

    pub async fn flush(&self) -> MikuResult<()> {
        let db = self.inner.clone();

        tokio::task::spawn_blocking(move || db.flush())
            .await
            .map_err(|e| MikuError::Internal(e.to_string()))?
    }
}

pub struct AsyncCollection {
    inner: crate::database::Collection,
}

impl AsyncCollection {
    pub fn new(collection: crate::database::Collection) -> Self {
        Self { inner: collection }
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub async fn insert(
        &self,
        doc: crate::boml::Document,
    ) -> MikuResult<crate::common::ObjectId> {
        let mut doc = doc;
        self.inner.insert(&mut doc)
    }

    pub async fn insert_many(
        &self,
        docs: Vec<crate::boml::Document>,
    ) -> MikuResult<Vec<crate::common::ObjectId>> {
        let mut docs = docs;
        self.inner.insert_many(&mut docs)
    }

    pub async fn find_one(
        &self,
        id: &crate::common::ObjectId,
    ) -> MikuResult<Option<crate::boml::Document>> {
        self.inner.find_one(id)
    }

    pub async fn find_all(&self) -> MikuResult<Vec<crate::boml::Document>> {
        self.inner.find_all()
    }

    pub async fn update(
        &self,
        id: &crate::common::ObjectId,
        doc: &crate::boml::Document,
    ) -> MikuResult<()> {
        self.inner.update(id, doc)
    }

    pub async fn delete(&self, id: &crate::common::ObjectId) -> MikuResult<bool> {
        self.inner.delete(id)
    }

    pub async fn count(&self) -> MikuResult<u64> {
        self.inner.count()
    }

    pub async fn clear(&self) -> MikuResult<u64> {
        self.inner.clear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_client_connect() {
        let dir = tempdir().unwrap();
        let uri = dir.path().to_str().unwrap();

        let client = Client::connect(uri).await.unwrap();
        assert!(client.pool_available() > 0);
    }

    #[tokio::test]
    async fn test_client_database() {
        let dir = tempdir().unwrap();
        let uri = dir.path().to_str().unwrap();

        let client = Client::connect(uri).await.unwrap();
        let db = client.database("test");

        assert_eq!(db.name(), "test");
    }

    #[tokio::test]
    async fn test_client_execute() {
        let dir = tempdir().unwrap();
        let uri = dir.path().to_str().unwrap();

        let client = Client::connect(uri).await.unwrap();

        let result = client.execute("default", "CREATE COLLECTION users").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_options_parse() {
        let options = ClientOptions::parse(
            "mikudb://localhost:3939/mydb?maxPoolSize=50&retryWrites=true",
        )
        .unwrap();

        assert_eq!(options.max_pool_size, 50);
        assert!(options.retry_writes);
    }

    #[test]
    fn test_client_options_builder() {
        let options = ClientOptions::builder()
            .data_dir("/tmp/test")
            .max_pool_size(200)
            .retry_writes(false)
            .app_name("test-app")
            .build();

        assert_eq!(options.data_dir, PathBuf::from("/tmp/test"));
        assert_eq!(options.max_pool_size, 200);
        assert!(!options.retry_writes);
        assert_eq!(options.app_name, Some("test-app".to_string()));
    }
}

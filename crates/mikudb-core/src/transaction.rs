//! 事务管理模块
//!
//! 提供 ACID 事务支持，包括多文档事务、会话管理和隔离级别控制。
//!
//! # 示例
//!
//! ```rust,ignore
//! use mikudb_core::{Database, Transaction};
//!
//! let db = Database::open("mydb", "/var/lib/mikudb/data")?;
//! let session = db.start_session()?;
//!
//! session.start_transaction()?;
//! session.execute("INSERT INTO users {name: 'Miku'}")?;
//! session.commit()?;
//! ```

use crate::boml::Document;
use crate::common::{MikuError, MikuResult, ObjectId};
use crate::query::{Parser, QueryResponse, Statement};
use crate::storage::StorageEngine;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

static TRANSACTION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Snapshot,
    Serializable,
}

impl Default for IsolationLevel {
    fn default() -> Self {
        Self::ReadCommitted
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    None,
    Starting,
    InProgress,
    Committing,
    Committed,
    Aborting,
    Aborted,
}

#[derive(Debug, Clone)]
pub struct TransactionOptions {
    pub isolation_level: IsolationLevel,
    pub read_only: bool,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl Default for TransactionOptions {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::ReadCommitted,
            read_only: false,
            timeout: Duration::from_secs(60),
            max_retries: 3,
        }
    }
}

#[derive(Debug)]
struct WriteOperation {
    collection: String,
    document_id: ObjectId,
    operation: WriteOpType,
    old_value: Option<Document>,
    new_value: Option<Document>,
}

#[derive(Debug, Clone, Copy)]
enum WriteOpType {
    Insert,
    Update,
    Delete,
}

pub struct Transaction {
    id: u64,
    session_id: u64,
    state: RwLock<TransactionState>,
    options: TransactionOptions,
    start_time: Instant,
    storage: Arc<StorageEngine>,
    write_set: Mutex<Vec<WriteOperation>>,
    read_set: Mutex<HashMap<String, Vec<ObjectId>>>,
    snapshot_version: u64,
}

impl Transaction {
    pub(crate) fn new(
        session_id: u64,
        storage: Arc<StorageEngine>,
        options: TransactionOptions,
    ) -> Self {
        let id = TRANSACTION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        debug!("Creating transaction {} for session {}", id, session_id);

        Self {
            id,
            session_id,
            state: RwLock::new(TransactionState::None),
            options,
            start_time: Instant::now(),
            storage,
            write_set: Mutex::new(Vec::new()),
            read_set: Mutex::new(HashMap::new()),
            snapshot_version: id,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn state(&self) -> TransactionState {
        *self.state.read()
    }

    pub fn is_active(&self) -> bool {
        matches!(
            *self.state.read(),
            TransactionState::Starting | TransactionState::InProgress
        )
    }

    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() > self.options.timeout
    }

    pub fn start(&self) -> MikuResult<()> {
        let mut state = self.state.write();
        if *state != TransactionState::None {
            return Err(MikuError::Transaction(
                "Transaction already started".to_string(),
            ));
        }

        *state = TransactionState::Starting;
        debug!("Starting transaction {}", self.id);
        *state = TransactionState::InProgress;

        Ok(())
    }

    pub fn commit(&self) -> MikuResult<()> {
        let mut state = self.state.write();
        if *state != TransactionState::InProgress {
            return Err(MikuError::Transaction(
                "Transaction not in progress".to_string(),
            ));
        }

        if self.is_timed_out() {
            *state = TransactionState::Aborted;
            return Err(MikuError::Transaction("Transaction timed out".to_string()));
        }

        *state = TransactionState::Committing;
        debug!("Committing transaction {}", self.id);

        let write_set = self.write_set.lock();
        for op in write_set.iter() {
            match op.operation {
                WriteOpType::Insert => {
                    if let Some(ref doc) = op.new_value {
                        let collection = self
                            .storage
                            .get_or_create_collection(&op.collection)
                            .map_err(|e| MikuError::Storage(e.to_string()))?;

                        let mut doc_clone = doc.clone();
                        collection
                            .insert(&mut doc_clone)
                            .map_err(|e| MikuError::Storage(e.to_string()))?;
                    }
                }
                WriteOpType::Update => {
                    if let Some(ref doc) = op.new_value {
                        let collection = self
                            .storage
                            .get_collection(&op.collection)
                            .map_err(|e| MikuError::Storage(e.to_string()))?;

                        collection
                            .update(&op.document_id, doc)
                            .map_err(|e| MikuError::Storage(e.to_string()))?;
                    }
                }
                WriteOpType::Delete => {
                    let collection = self
                        .storage
                        .get_collection(&op.collection)
                        .map_err(|e| MikuError::Storage(e.to_string()))?;

                    collection
                        .delete(&op.document_id)
                        .map_err(|e| MikuError::Storage(e.to_string()))?;
                }
            }
        }

        *state = TransactionState::Committed;
        info!("Transaction {} committed successfully", self.id);

        Ok(())
    }

    pub fn abort(&self) -> MikuResult<()> {
        let mut state = self.state.write();
        if !matches!(
            *state,
            TransactionState::InProgress | TransactionState::Starting
        ) {
            return Err(MikuError::Transaction(
                "Transaction not in progress".to_string(),
            ));
        }

        *state = TransactionState::Aborting;
        debug!("Aborting transaction {}", self.id);

        self.write_set.lock().clear();
        self.read_set.lock().clear();

        *state = TransactionState::Aborted;
        info!("Transaction {} aborted", self.id);

        Ok(())
    }

    pub fn rollback(&self) -> MikuResult<()> {
        self.abort()
    }

    pub(crate) fn add_insert(
        &self,
        collection: &str,
        document_id: ObjectId,
        document: Document,
    ) -> MikuResult<()> {
        if self.options.read_only {
            return Err(MikuError::Transaction(
                "Cannot write in read-only transaction".to_string(),
            ));
        }

        self.write_set.lock().push(WriteOperation {
            collection: collection.to_string(),
            document_id,
            operation: WriteOpType::Insert,
            old_value: None,
            new_value: Some(document),
        });

        Ok(())
    }

    pub(crate) fn add_update(
        &self,
        collection: &str,
        document_id: ObjectId,
        old_value: Option<Document>,
        new_value: Document,
    ) -> MikuResult<()> {
        if self.options.read_only {
            return Err(MikuError::Transaction(
                "Cannot write in read-only transaction".to_string(),
            ));
        }

        self.write_set.lock().push(WriteOperation {
            collection: collection.to_string(),
            document_id,
            operation: WriteOpType::Update,
            old_value,
            new_value: Some(new_value),
        });

        Ok(())
    }

    pub(crate) fn add_delete(
        &self,
        collection: &str,
        document_id: ObjectId,
        old_value: Option<Document>,
    ) -> MikuResult<()> {
        if self.options.read_only {
            return Err(MikuError::Transaction(
                "Cannot write in read-only transaction".to_string(),
            ));
        }

        self.write_set.lock().push(WriteOperation {
            collection: collection.to_string(),
            document_id,
            operation: WriteOpType::Delete,
            old_value,
            new_value: None,
        });

        Ok(())
    }

    pub(crate) fn track_read(&self, collection: &str, document_id: ObjectId) {
        self.read_set
            .lock()
            .entry(collection.to_string())
            .or_default()
            .push(document_id);
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        let state = *self.state.read();
        if state == TransactionState::InProgress {
            warn!("Transaction {} dropped while in progress, aborting", self.id);
            let _ = self.abort();
        }
    }
}

pub struct Session {
    id: u64,
    storage: Arc<StorageEngine>,
    current_transaction: Mutex<Option<Arc<Transaction>>>,
    default_transaction_options: TransactionOptions,
    created_at: Instant,
    last_active: Mutex<Instant>,
    timeout: Duration,
}

impl Session {
    pub(crate) fn new(storage: Arc<StorageEngine>) -> Self {
        let id = SESSION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        debug!("Creating session {}", id);

        Self {
            id,
            storage,
            current_transaction: Mutex::new(None),
            default_transaction_options: TransactionOptions::default(),
            created_at: Instant::now(),
            last_active: Mutex::new(Instant::now()),
            timeout: Duration::from_secs(30 * 60),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn is_expired(&self) -> bool {
        self.last_active.lock().elapsed() > self.timeout
    }

    fn touch(&self) {
        *self.last_active.lock() = Instant::now();
    }

    pub fn start_transaction(&self) -> MikuResult<Arc<Transaction>> {
        self.start_transaction_with_options(self.default_transaction_options.clone())
    }

    pub fn start_transaction_with_options(
        &self,
        options: TransactionOptions,
    ) -> MikuResult<Arc<Transaction>> {
        self.touch();

        let mut current = self.current_transaction.lock();
        if let Some(ref txn) = *current {
            if txn.is_active() {
                return Err(MikuError::Transaction(
                    "Session already has an active transaction".to_string(),
                ));
            }
        }

        let txn = Arc::new(Transaction::new(self.id, self.storage.clone(), options));
        txn.start()?;

        *current = Some(txn.clone());
        Ok(txn)
    }

    pub fn current_transaction(&self) -> Option<Arc<Transaction>> {
        self.touch();
        self.current_transaction.lock().clone()
    }

    pub fn has_active_transaction(&self) -> bool {
        self.current_transaction
            .lock()
            .as_ref()
            .map(|t| t.is_active())
            .unwrap_or(false)
    }

    pub fn commit_transaction(&self) -> MikuResult<()> {
        self.touch();

        let mut current = self.current_transaction.lock();
        if let Some(ref txn) = *current {
            txn.commit()?;
            *current = None;
            Ok(())
        } else {
            Err(MikuError::Transaction("No active transaction".to_string()))
        }
    }

    pub fn abort_transaction(&self) -> MikuResult<()> {
        self.touch();

        let mut current = self.current_transaction.lock();
        if let Some(ref txn) = *current {
            txn.abort()?;
            *current = None;
            Ok(())
        } else {
            Err(MikuError::Transaction("No active transaction".to_string()))
        }
    }

    pub fn execute(&self, query: &str) -> MikuResult<QueryResponse> {
        self.touch();

        let stmt = Parser::parse(query).map_err(|e| MikuError::Query(e.to_string()))?;

        self.execute_statement(&stmt)
    }

    pub fn execute_statement(&self, stmt: &Statement) -> MikuResult<QueryResponse> {
        self.touch();

        match stmt {
            Statement::BeginTransaction => {
                self.start_transaction()?;
                Ok(QueryResponse::Ok {
                    message: "Transaction started".to_string(),
                })
            }
            Statement::Commit => {
                self.commit_transaction()?;
                Ok(QueryResponse::Ok {
                    message: "Transaction committed".to_string(),
                })
            }
            Statement::Rollback => {
                self.abort_transaction()?;
                Ok(QueryResponse::Ok {
                    message: "Transaction rolled back".to_string(),
                })
            }
            _ => {
                let executor = crate::query::QueryExecutor::new(self.storage.clone());
                executor
                    .execute(stmt)
                    .map_err(|e| MikuError::Query(e.to_string()))
            }
        }
    }

    pub fn with_transaction<F, T>(&self, f: F) -> MikuResult<T>
    where
        F: FnOnce(&Transaction) -> MikuResult<T>,
    {
        let txn = self.start_transaction()?;
        match f(&txn) {
            Ok(result) => {
                self.commit_transaction()?;
                Ok(result)
            }
            Err(e) => {
                let _ = self.abort_transaction();
                Err(e)
            }
        }
    }

    pub fn with_transaction_retry<F, T>(&self, max_retries: u32, mut f: F) -> MikuResult<T>
    where
        F: FnMut(&Transaction) -> MikuResult<T>,
    {
        let mut attempts = 0;

        loop {
            let txn = self.start_transaction()?;
            match f(&txn) {
                Ok(result) => {
                    match self.commit_transaction() {
                        Ok(()) => return Ok(result),
                        Err(MikuError::Storage(ref msg)) if msg.contains("conflict") => {
                            attempts += 1;
                            if attempts >= max_retries {
                                return Err(MikuError::Transaction(format!(
                                    "Transaction failed after {} retries",
                                    max_retries
                                )));
                            }
                            warn!("Transaction conflict, retrying (attempt {}/{})", attempts, max_retries);
                            continue;
                        }
                        Err(e) => {
                            let _ = self.abort_transaction();
                            return Err(e);
                        }
                    }
                }
                Err(e) => {
                    let _ = self.abort_transaction();
                    return Err(e);
                }
            }
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.has_active_transaction() {
            warn!("Session {} dropped with active transaction, aborting", self.id);
            let _ = self.abort_transaction();
        }
        debug!("Session {} closed", self.id);
    }
}

pub struct SessionManager {
    storage: Arc<StorageEngine>,
    sessions: RwLock<HashMap<u64, Arc<Session>>>,
    session_timeout: Duration,
}

impl SessionManager {
    pub fn new(storage: Arc<StorageEngine>) -> Self {
        Self {
            storage,
            sessions: RwLock::new(HashMap::new()),
            session_timeout: Duration::from_secs(30 * 60),
        }
    }

    pub fn create_session(&self) -> Arc<Session> {
        let session = Arc::new(Session::new(self.storage.clone()));
        self.sessions.write().insert(session.id(), session.clone());
        session
    }

    pub fn get_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.read().get(&id).cloned()
    }

    pub fn end_session(&self, id: u64) -> MikuResult<()> {
        if let Some(session) = self.sessions.write().remove(&id) {
            if session.has_active_transaction() {
                session.abort_transaction()?;
            }
        }
        Ok(())
    }

    pub fn cleanup_expired_sessions(&self) {
        let mut sessions = self.sessions.write();
        let expired: Vec<u64> = sessions
            .iter()
            .filter(|(_, s)| s.is_expired())
            .map(|(id, _)| *id)
            .collect();

        for id in expired {
            if let Some(session) = sessions.remove(&id) {
                if session.has_active_transaction() {
                    let _ = session.abort_transaction();
                }
                info!("Cleaned up expired session {}", id);
            }
        }
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.read().len()
    }

    pub fn active_transaction_count(&self) -> usize {
        self.sessions
            .read()
            .values()
            .filter(|s| s.has_active_transaction())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageOptions;
    use tempfile::tempdir;

    fn create_test_storage() -> Arc<StorageEngine> {
        let dir = tempdir().unwrap();
        let options = StorageOptions {
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        Arc::new(StorageEngine::open(options).unwrap())
    }

    #[test]
    fn test_session_creation() {
        let storage = create_test_storage();
        let manager = SessionManager::new(storage);

        let session = manager.create_session();
        assert!(session.id() > 0);
        assert!(!session.has_active_transaction());
    }

    #[test]
    fn test_transaction_lifecycle() {
        let storage = create_test_storage();
        let session = Session::new(storage);

        let txn = session.start_transaction().unwrap();
        assert!(txn.is_active());
        assert_eq!(txn.state(), TransactionState::InProgress);

        session.commit_transaction().unwrap();
        assert!(!session.has_active_transaction());
    }

    #[test]
    fn test_transaction_abort() {
        let storage = create_test_storage();
        let session = Session::new(storage);

        session.start_transaction().unwrap();
        assert!(session.has_active_transaction());

        session.abort_transaction().unwrap();
        assert!(!session.has_active_transaction());
    }

    #[test]
    fn test_with_transaction() {
        let storage = create_test_storage();
        let session = Session::new(storage);

        let result = session.with_transaction(|_txn| Ok(42));
        assert_eq!(result.unwrap(), 42);
        assert!(!session.has_active_transaction());
    }

    #[test]
    fn test_nested_transaction_error() {
        let storage = create_test_storage();
        let session = Session::new(storage);

        session.start_transaction().unwrap();

        let result = session.start_transaction();
        assert!(result.is_err());
    }

    #[test]
    fn test_session_manager() {
        let storage = create_test_storage();
        let manager = SessionManager::new(storage);

        let session1 = manager.create_session();
        let session2 = manager.create_session();

        assert_eq!(manager.active_session_count(), 2);

        manager.end_session(session1.id()).unwrap();
        assert_eq!(manager.active_session_count(), 1);

        manager.end_session(session2.id()).unwrap();
        assert_eq!(manager.active_session_count(), 0);
    }
}

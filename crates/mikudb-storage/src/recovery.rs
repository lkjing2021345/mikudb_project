//! 崩溃恢复模块
//!
//! 实现基于 WAL 的崩溃恢复机制:
//! - **事务状态恢复**: 识别未完成的事务并回滚
//! - **数据一致性**: 重放已提交事务的操作
//! - **幂等性保证**: 确保重放操作可以安全执行多次
//!
//! # 恢复流程
//!
//! 1. 扫描 WAL 文件,收集所有事务的状态
//! 2. 识别已提交(Committed)、已中止(Aborted)和未完成(Pending)的事务
//! 3. 重放已提交事务的所有操作(Insert/Update/Delete)
//! 4. 忽略已中止和未完成的事务
//! 5. 清空 WAL 文件或创建 checkpoint

use crate::wal::{RecordType, WalRecord, WriteAheadLog};
use crate::{StorageError, StorageResult};
use mikudb_boml::codec;
use rocksdb::{BoundColumnFamily, WriteBatch, WriteOptions, DB};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// 事务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransactionState {
    /// 事务进行中(已开始,未提交/中止)
    Pending,
    /// 事务已提交
    Committed,
    /// 事务已中止
    Aborted,
}

/// 恢复管理器
///
/// 负责从 WAL 中恢复数据库状态
pub struct RecoveryManager {
    db: Arc<DB>,
    wal: Arc<WriteAheadLog>,
}

impl RecoveryManager {
    /// 创建恢复管理器
    ///
    /// # Arguments
    /// * `db` - RocksDB 实例
    /// * `wal` - WAL 实例
    pub fn new(db: Arc<DB>, wal: Arc<WriteAheadLog>) -> Self {
        Self { db, wal }
    }

    /// 执行崩溃恢复
    ///
    /// # Brief
    /// 从 WAL 中恢复未持久化的数据,确保数据库状态一致
    ///
    /// # Returns
    /// 成功返回恢复的操作数量,失败返回错误
    pub fn recover(&self) -> StorageResult<RecoveryStats> {
        info!("Starting crash recovery from WAL...");

        // 第一遍扫描: 收集所有事务的状态
        let tx_states = self.scan_transaction_states()?;

        debug!(
            "Found {} committed, {} aborted, {} pending transactions",
            tx_states.iter().filter(|(_, s)| **s == TransactionState::Committed).count(),
            tx_states.iter().filter(|(_, s)| **s == TransactionState::Aborted).count(),
            tx_states.iter().filter(|(_, s)| **s == TransactionState::Pending).count()
        );

        // 第二遍扫描: 重放已提交事务的操作
        let stats = self.replay_committed_transactions(&tx_states)?;

        // 恢复完成后,截断 WAL
        if stats.total_replayed > 0 {
            info!(
                "Recovery completed: {} operations replayed, truncating WAL",
                stats.total_replayed
            );
            self.wal.truncate()?;
        } else {
            info!("No operations to replay, WAL is clean");
        }

        Ok(stats)
    }

    /// 扫描 WAL 并收集所有事务的状态
    ///
    /// # Returns
    /// 事务 ID 到状态的映射
    fn scan_transaction_states(&self) -> StorageResult<HashMap<u64, TransactionState>> {
        let mut tx_states = HashMap::new();

        self.wal.replay(|record| {
            match record.record_type {
                RecordType::BeginTx => {
                    tx_states.insert(record.tx_id, TransactionState::Pending);
                }
                RecordType::CommitTx => {
                    tx_states.insert(record.tx_id, TransactionState::Committed);
                }
                RecordType::AbortTx => {
                    tx_states.insert(record.tx_id, TransactionState::Aborted);
                }
                _ => {
                    // Insert/Update/Delete 操作,确保事务存在
                    tx_states.entry(record.tx_id).or_insert(TransactionState::Pending);
                }
            }
            Ok(())
        })?;

        Ok(tx_states)
    }

    /// 重放已提交事务的操作
    ///
    /// # Arguments
    /// * `tx_states` - 事务状态映射
    ///
    /// # Returns
    /// 恢复统计信息
    fn replay_committed_transactions(
        &self,
        tx_states: &HashMap<u64, TransactionState>,
    ) -> StorageResult<RecoveryStats> {
        let mut stats = RecoveryStats::default();

        // 收集已提交的事务 ID
        let committed_txs: HashSet<u64> = tx_states
            .iter()
            .filter(|(_, state)| **state == TransactionState::Committed)
            .map(|(tx_id, _)| *tx_id)
            .collect();

        if committed_txs.is_empty() {
            return Ok(stats);
        }

        // 按事务分组操作
        let mut tx_operations: HashMap<u64, Vec<WalRecord>> = HashMap::new();

        self.wal.replay(|record| {
            // 只处理已提交事务的数据操作
            if committed_txs.contains(&record.tx_id) {
                match record.record_type {
                    RecordType::Insert | RecordType::Update | RecordType::Delete => {
                        tx_operations
                            .entry(record.tx_id)
                            .or_insert_with(Vec::new)
                            .push(record);
                    }
                    _ => {}
                }
            }
            Ok(())
        })?;

        // 重放每个已提交事务的操作
        for (tx_id, operations) in tx_operations {
            match self.replay_transaction_operations(tx_id, &operations) {
                Ok(tx_stats) => {
                    stats.transactions_recovered += 1;
                    stats.inserts_replayed += tx_stats.inserts;
                    stats.updates_replayed += tx_stats.updates;
                    stats.deletes_replayed += tx_stats.deletes;
                    stats.total_replayed += tx_stats.total();
                }
                Err(e) => {
                    error!("Failed to replay transaction {}: {}", tx_id, e);
                    stats.errors_encountered += 1;
                }
            }
        }

        Ok(stats)
    }

    /// 重放单个事务的所有操作
    ///
    /// # Arguments
    /// * `tx_id` - 事务 ID
    /// * `operations` - 事务的操作列表
    ///
    /// # Returns
    /// 该事务的统计信息
    fn replay_transaction_operations(
        &self,
        tx_id: u64,
        operations: &[WalRecord],
    ) -> StorageResult<TransactionRecoveryStats> {
        let mut batch = WriteBatch::default();
        let mut stats = TransactionRecoveryStats::default();

        for record in operations {
            // 获取集合的 ColumnFamily
            let cf = self.db.cf_handle(&record.collection).ok_or_else(|| {
                StorageError::CollectionNotFound(record.collection.clone())
            })?;

            match record.record_type {
                RecordType::Insert | RecordType::Update => {
                    // Insert 和 Update 都使用 put 操作(幂等)
                    batch.put_cf(&cf, &record.key, &record.value);

                    if record.record_type == RecordType::Insert {
                        stats.inserts += 1;
                    } else {
                        stats.updates += 1;
                    }
                }
                RecordType::Delete => {
                    batch.delete_cf(&cf, &record.key);
                    stats.deletes += 1;
                }
                _ => unreachable!("Only data operations should be replayed"),
            }
        }

        // 批量写入
        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(true); // 确保恢复操作持久化

        self.db.write_opt(batch, &write_opts)?;

        debug!(
            "Replayed transaction {}: {} inserts, {} updates, {} deletes",
            tx_id, stats.inserts, stats.updates, stats.deletes
        );

        Ok(stats)
    }
}

/// 恢复统计信息
#[derive(Debug, Default, Clone)]
pub struct RecoveryStats {
    /// 恢复的事务数量
    pub transactions_recovered: u64,
    /// 重放的插入操作数
    pub inserts_replayed: u64,
    /// 重放的更新操作数
    pub updates_replayed: u64,
    /// 重放的删除操作数
    pub deletes_replayed: u64,
    /// 遇到的错误数
    pub errors_encountered: u64,
    /// 总重放操作数
    pub total_replayed: u64,
}

/// 单个事务的恢复统计
#[derive(Debug, Default)]
struct TransactionRecoveryStats {
    inserts: u64,
    updates: u64,
    deletes: u64,
}

impl TransactionRecoveryStats {
    fn total(&self) -> u64 {
        self.inserts + self.updates + self.deletes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wal::WalRecord;
    use tempfile::tempdir;

    #[test]
    fn test_recovery_committed_transaction() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("db");
        let wal_path = dir.path().join("test.wal");

        // 创建数据库
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descriptor =
            rocksdb::ColumnFamilyDescriptor::new("test_collection", rocksdb::Options::default());
        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(&opts, &db_path, vec![cf_descriptor]).unwrap(),
        );

        // 创建 WAL 并写入事务记录
        let wal = Arc::new(WriteAheadLog::open(&wal_path, true).unwrap());

        // 事务 1: 已提交
        wal.append(&WalRecord::new_begin_tx(1)).unwrap();
        wal.append(&WalRecord::new_insert(
            1,
            "test_collection",
            b"key1".to_vec(),
            b"value1".to_vec(),
        ))
        .unwrap();
        wal.append(&WalRecord::new_commit_tx(1)).unwrap();

        // 事务 2: 未提交(应该被忽略)
        wal.append(&WalRecord::new_begin_tx(2)).unwrap();
        wal.append(&WalRecord::new_insert(
            2,
            "test_collection",
            b"key2".to_vec(),
            b"value2".to_vec(),
        ))
        .unwrap();

        wal.sync().unwrap();

        // 执行恢复
        let recovery = RecoveryManager::new(db.clone(), wal.clone());
        let stats = recovery.recover().unwrap();

        assert_eq!(stats.transactions_recovered, 1);
        assert_eq!(stats.inserts_replayed, 1);

        // 验证数据
        let cf = db.cf_handle("test_collection").unwrap();
        assert!(db.get_cf(&cf, b"key1").unwrap().is_some());
        assert!(db.get_cf(&cf, b"key2").unwrap().is_none());
    }

    #[test]
    fn test_recovery_aborted_transaction() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("db");
        let wal_path = dir.path().join("test.wal");

        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cf_descriptor =
            rocksdb::ColumnFamilyDescriptor::new("test_collection", rocksdb::Options::default());
        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(&opts, &db_path, vec![cf_descriptor]).unwrap(),
        );

        let wal = Arc::new(WriteAheadLog::open(&wal_path, true).unwrap());

        // 事务: 已中止(不应该重放)
        wal.append(&WalRecord::new_begin_tx(1)).unwrap();
        wal.append(&WalRecord::new_insert(
            1,
            "test_collection",
            b"key1".to_vec(),
            b"value1".to_vec(),
        ))
        .unwrap();
        wal.append(&WalRecord::new_abort_tx(1)).unwrap();
        wal.sync().unwrap();

        let recovery = RecoveryManager::new(db.clone(), wal.clone());
        let stats = recovery.recover().unwrap();

        assert_eq!(stats.transactions_recovered, 0);
        assert_eq!(stats.total_replayed, 0);

        // 验证数据未写入
        let cf = db.cf_handle("test_collection").unwrap();
        assert!(db.get_cf(&cf, b"key1").unwrap().is_none());
    }
}

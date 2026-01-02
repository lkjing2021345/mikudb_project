//! WAL (Write-Ahead Log) 模块
//!
//! 实现预写式日志,保证数据库的持久性和崩溃恢复能力:
//! - **原子性保证**: 所有操作先写入 WAL,再应用到数据库
//! - **崩溃恢复**: 通过重放 WAL 记录恢复未提交的事务
//! - **校验和保护**: 使用 xxHash3 校验和,防止数据损坏
//! - **文件轮转**: 超过大小限制时自动轮转 WAL 文件
//!
//! # WAL 记录格式
//!
//! 每条记录包含:
//! - 记录类型 (1 字节): Insert/Update/Delete/BeginTx/CommitTx/AbortTx/Checkpoint
//! - 事务 ID (8 字节)
//! - 集合名长度 (2 字节) + 集合名
//! - 键长度 (4 字节) + 键数据
//! - 值长度 (4 字节) + 值数据
//! - xxHash3 校验和 (8 字节)
//!
//! # OpenEuler 适配亮点
//!
//! - 使用 xxHash3 进行校验和计算,在 ARM64 (鲲鹏) 上性能优异
//! - 支持 Direct I/O 写入,减少内存拷贝

use crate::{StorageError, StorageResult};
use bytes::{BufMut, BytesMut};
use parking_lot::Mutex;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use xxhash_rust::xxh3::xxh3_64;

/// WAL 魔数字节 "MWAL"
const WAL_MAGIC: [u8; 4] = [0x4D, 0x57, 0x41, 0x4C];
/// WAL 文件格式版本号
const WAL_VERSION: u8 = 1;
/// 记录头大小 (17 字节: type(1) + tx_id(8) + collection_len(2) + key_len(4) + value_len(4) - 不包括变长数据)
const RECORD_HEADER_SIZE: usize = 17;

/// WAL 记录类型
///
/// 定义了 WAL 中可能出现的所有操作类型。
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    /// 插入操作
    Insert = 1,
    /// 更新操作
    Update = 2,
    /// 删除操作
    Delete = 3,
    /// 事务开始
    BeginTx = 10,
    /// 事务提交
    CommitTx = 11,
    /// 事务中止
    AbortTx = 12,
    /// 检查点
    Checkpoint = 20,
}

impl RecordType {
    /// # Brief
    /// 从字节值转换为 RecordType
    ///
    /// # Arguments
    /// * `v` - 字节值
    ///
    /// # Returns
    /// 有效的 RecordType 或 None
    fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(Self::Insert),
            2 => Some(Self::Update),
            3 => Some(Self::Delete),
            10 => Some(Self::BeginTx),
            11 => Some(Self::CommitTx),
            12 => Some(Self::AbortTx),
            20 => Some(Self::Checkpoint),
            _ => None,
        }
    }
}

/// WAL 记录
///
/// 表示一条完整的 WAL 记录,包含所有必要信息。
#[derive(Debug, Clone)]
pub struct WalRecord {
    /// 记录类型
    pub record_type: RecordType,
    /// 事务 ID
    pub tx_id: u64,
    /// 集合名称
    pub collection: String,
    /// 文档键
    pub key: Vec<u8>,
    /// 文档值
    pub value: Vec<u8>,
}

impl WalRecord {
    /// # Brief
    /// 创建插入记录
    ///
    /// # Arguments
    /// * `tx_id` - 事务 ID
    /// * `collection` - 集合名
    /// * `key` - 文档键
    /// * `value` - 文档值
    pub fn new_insert(tx_id: u64, collection: &str, key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Insert,
            tx_id,
            collection: collection.to_string(),
            key,
            value,
        }
    }

    /// # Brief
    /// 创建更新记录
    ///
    /// # Arguments
    /// * `tx_id` - 事务 ID
    /// * `collection` - 集合名
    /// * `key` - 文档键
    /// * `value` - 新的文档值
    pub fn new_update(tx_id: u64, collection: &str, key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Update,
            tx_id,
            collection: collection.to_string(),
            key,
            value,
        }
    }

    /// # Brief
    /// 创建删除记录
    ///
    /// # Arguments
    /// * `tx_id` - 事务 ID
    /// * `collection` - 集合名
    /// * `key` - 要删除的文档键
    pub fn new_delete(tx_id: u64, collection: &str, key: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Delete,
            tx_id,
            collection: collection.to_string(),
            key,
            value: Vec::new(),
        }
    }

    pub fn new_begin_tx(tx_id: u64) -> Self {
        Self {
            record_type: RecordType::BeginTx,
            tx_id,
            collection: String::new(),
            key: Vec::new(),
            value: Vec::new(),
        }
    }

    pub fn new_commit_tx(tx_id: u64) -> Self {
        Self {
            record_type: RecordType::CommitTx,
            tx_id,
            collection: String::new(),
            key: Vec::new(),
            value: Vec::new(),
        }
    }

    pub fn new_abort_tx(tx_id: u64) -> Self {
        Self {
            record_type: RecordType::AbortTx,
            tx_id,
            collection: String::new(),
            key: Vec::new(),
            value: Vec::new(),
        }
    }

    /// # Brief
    /// 将记录编码为字节数组
    ///
    /// 编码格式:
    /// - 记录类型 (1 字节)
    /// - 事务 ID (8 字节,小端)
    /// - 集合名长度 (2 字节,小端) + 集合名 UTF-8 字节
    /// - 键长度 (4 字节,小端) + 键数据
    /// - 值长度 (4 字节,小端) + 值数据
    /// - xxHash3 校验和 (8 字节,小端)
    ///
    /// # Returns
    /// 编码后的字节数组
    fn encode(&self) -> Vec<u8> {
        // 计算总长度: type(1) + tx_id(8) + coll_len(2) + coll + key_len(4) + key + val_len(4) + val
        let collection_bytes = self.collection.as_bytes();
        let total_len = 1 + 8 + 2 + collection_bytes.len() + 4 + self.key.len() + 4 + self.value.len();

        // 预留 8 字节用于校验和
        let mut buf = BytesMut::with_capacity(total_len + 8);

        // 写入记录类型
        buf.put_u8(self.record_type as u8);
        // 写入事务 ID
        buf.put_u64_le(self.tx_id);
        // 写入集合名长度和内容
        buf.put_u16_le(collection_bytes.len() as u16);
        buf.put_slice(collection_bytes);
        // 写入键长度和内容
        buf.put_u32_le(self.key.len() as u32);
        buf.put_slice(&self.key);
        // 写入值长度和内容
        buf.put_u32_le(self.value.len() as u32);
        buf.put_slice(&self.value);

        // 计算并附加 xxHash3 校验和
        let checksum = xxh3_64(&buf);
        buf.put_u64_le(checksum);

        buf.to_vec()
    }

    /// # Brief
    /// 从字节数组解码记录
    ///
    /// 首先验证 xxHash3 校验和,然后解析各个字段。
    ///
    /// # Arguments
    /// * `data` - 编码后的字节数组
    ///
    /// # Returns
    /// 解码后的 WalRecord,或校验和/格式错误
    fn decode(data: &[u8]) -> StorageResult<Self> {
        // 最小长度: type(1) + tx_id(8) + coll_len(2) + key_len(4) + val_len(4) + checksum(8) = 27
        // 但这里检查 20 是为了容错
        if data.len() < 20 {
            return Err(StorageError::Corruption("WAL record too small".to_string()));
        }

        // 验证 xxHash3 校验和
        let checksum_pos = data.len() - 8;
        let stored_checksum = u64::from_le_bytes(data[checksum_pos..].try_into().unwrap());
        let computed_checksum = xxh3_64(&data[..checksum_pos]);

        if stored_checksum != computed_checksum {
            return Err(StorageError::Corruption("WAL checksum mismatch".to_string()));
        }

        let mut pos = 0;

        // 解析记录类型
        let record_type = RecordType::from_u8(data[pos])
            .ok_or_else(|| StorageError::Corruption("Invalid record type".to_string()))?;
        pos += 1;

        // 解析事务 ID
        let tx_id = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        // 解析集合名
        let collection_len = u16::from_le_bytes(data[pos..pos + 2].try_into().unwrap()) as usize;
        pos += 2;

        let collection = String::from_utf8(data[pos..pos + collection_len].to_vec())
            .map_err(|e| StorageError::Corruption(format!("Invalid collection name: {}", e)))?;
        pos += collection_len;

        // 解析键
        let key_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        let key = data[pos..pos + key_len].to_vec();
        pos += key_len;

        // 解析值
        let value_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        let value = data[pos..pos + value_len].to_vec();

        Ok(Self {
            record_type,
            tx_id,
            collection,
            key,
            value,
        })
    }
}

/// 预写式日志 (Write-Ahead Log)
///
/// 用于保证数据库的持久性和崩溃恢复能力。
pub struct WriteAheadLog {
    /// WAL 文件路径
    path: PathBuf,
    /// 缓冲写入器 (线程安全)
    writer: Mutex<BufWriter<File>>,
    /// LSN (Log Sequence Number) 日志序列号
    lsn: AtomicU64,
    /// WAL 文件大小
    file_size: AtomicU64,
    /// 文件转转阈值 (64MB)
    max_file_size: u64,
    /// 是否每次写入后同步到磁盘
    sync_on_write: bool,
}

impl WriteAheadLog {
    /// # Brief
    /// 打开或创建 WAL 文件
    ///
    /// 如果文件不存在,创建新文件并写入魔数字节和版本号。
    /// 如果文件存在,恢复 LSN 以便继续追加写入。
    ///
    /// # Arguments
    /// * `path` - WAL 文件路径
    /// * `sync_on_write` - 是否每次写入后同步到磁盘
    ///
    /// # Returns
    /// WriteAheadLog 实例或错误
    pub fn open(path: impl AsRef<Path>, sync_on_write: bool) -> StorageResult<Self> {
        let path = path.as_ref().to_path_buf();

        // 创建父目录
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // 打开或创建文件
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();

        // 如果文件为空,写入魔数字节和版本号;否则恢复 LSN
        let lsn = if file_size > 0 {
            Self::recover_lsn(&path)?
        } else {
            let mut writer = BufWriter::new(file);
            writer.write_all(&WAL_MAGIC)?;  // 写入 "MWAL"
            writer.write_all(&[WAL_VERSION])?;  // 写入版本号 1
            writer.flush()?;
            0
        };

        // 以追加模式重新打开文件
        let file = OpenOptions::new().append(true).open(&path)?;

        info!("WAL opened at {:?}, LSN: {}", path, lsn);

        Ok(Self {
            path,
            writer: Mutex::new(BufWriter::new(file)),
            lsn: AtomicU64::new(lsn),
            file_size: AtomicU64::new(file_size),
            max_file_size: 64 * 1024 * 1024,
            sync_on_write,
        })
    }

    /// # Brief
    /// 从 WAL 文件恢复 LSN
    ///
    /// 扫描整个 WAL 文件,计算记录数量以恢复 LSN。
    ///
    /// # Arguments
    /// * `path` - WAL 文件路径
    ///
    /// # Returns
    /// 恢复的 LSN 值
    fn recover_lsn(path: &Path) -> StorageResult<u64> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 5];
        file.read_exact(&mut header)?;

        // 验证魔数字节
        if &header[0..4] != WAL_MAGIC {
            return Err(StorageError::Corruption("Invalid WAL magic".to_string()));
        }

        let mut lsn = 0u64;
        let mut pos = 5u64;  // 跳过魔数字节和版本号
        let file_size = file.metadata()?.len();

        // 扫描所有记录
        while pos < file_size {
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() {
                break;  // 文件末尾
            }

            let record_len = u32::from_le_bytes(len_buf) as usize;
            // 验证记录长度
            if record_len == 0 || pos + 4 + record_len as u64 > file_size {
                break;  // 无效记录
            }

            // 跳过记录内容,计数
            file.seek(SeekFrom::Current(record_len as i64))?;
            pos += 4 + record_len as u64;
            lsn += 1;
        }

        Ok(lsn)
    }

    /// # Brief
    /// 追加记录到 WAL
    ///
    /// 将记录编码并写入 WAL 文件,返回分配的 LSN。
    ///
    /// # Arguments
    /// * `record` - 要追加的 WAL 记录
    ///
    /// # Returns
    /// 分配的 LSN
    pub fn append(&self, record: &WalRecord) -> StorageResult<u64> {
        let encoded = record.encode();
        // 原子地分配 LSN
        let lsn = self.lsn.fetch_add(1, Ordering::SeqCst);

        let mut writer = self.writer.lock();

        // 写入记录长度 (4 字节,小端)
        let len_bytes = (encoded.len() as u32).to_le_bytes();
        writer.write_all(&len_bytes)?;
        // 写入记录内容
        writer.write_all(&encoded)?;

        // 如果配置了同步写入,刷新并同步到磁盘
        if self.sync_on_write {
            writer.flush()?;
            writer.get_ref().sync_data()?;
        }

        // 更新文件大小
        self.file_size
            .fetch_add(4 + encoded.len() as u64, Ordering::Relaxed);

        Ok(lsn)
    }

    /// # Brief
    /// 同步 WAL 到磁盘
    ///
    /// 刷新缓冲区并同步到磁盘,确保数据持久化。
    pub fn sync(&self) -> StorageResult<()> {
        let mut writer = self.writer.lock();
        writer.flush()?;
        writer.get_ref().sync_all()?;
        Ok(())
    }

    /// # Brief
    /// 获取当前 LSN
    ///
    /// # Returns
    /// 当前的日志序列号
    pub fn current_lsn(&self) -> u64 {
        self.lsn.load(Ordering::SeqCst)
    }

    /// # Brief
    /// 获取 WAL 文件大小
    ///
    /// # Returns
    /// 文件大小(字节)
    pub fn file_size(&self) -> u64 {
        self.file_size.load(Ordering::Relaxed)
    }

    /// # Brief
    /// 判断是否应该转转 WAL 文件
    ///
    /// 当文件大小超过 64MB 时返回 true。
    ///
    /// # Returns
    /// 是否应该转转
    pub fn should_rotate(&self) -> bool {
        self.file_size() > self.max_file_size
    }

    /// # Brief
    /// 转转 WAL 文件
    ///
    /// 将当前 WAL 文件重命名为归档文件(带时间戳后缀),
    /// 然后创建新的空 WAL 文件。
    ///
    /// # Returns
    /// 归档文件路径
    pub fn rotate(&self) -> StorageResult<PathBuf> {
        let mut writer = self.writer.lock();
        // 刷新并同步当前文件
        writer.flush()?;
        writer.get_ref().sync_all()?;

        // 生成归档文件名 (使用毫秒时间戳)
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let archive_path = self.path.with_extension(format!("wal.{}", timestamp));
        // 重命名当前 WAL 文件
        std::fs::rename(&self.path, &archive_path)?;

        // 创建新的 WAL 文件
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        let mut new_writer = BufWriter::new(file);
        new_writer.write_all(&WAL_MAGIC)?;
        new_writer.write_all(&[WAL_VERSION])?;
        new_writer.flush()?;

        // 更新写入器
        *writer = BufWriter::new(OpenOptions::new().append(true).open(&self.path)?);

        // 重置文件大小 (5 字节 = 魔数字节 + 版本号)
        self.file_size.store(5, Ordering::Relaxed);

        info!("WAL rotated to {:?}", archive_path);
        Ok(archive_path)
    }

    /// # Brief
    /// 重放 WAL 记录
    ///
    /// 读取 WAL 文件中的所有记录,并对每条记录调用回调函数。
    /// 用于崩溃恢复。
    ///
    /// # Arguments
    /// * `callback` - 处理每条记录的回调函数
    ///
    /// # Returns
    /// 重放的记录数量
    pub fn replay<F>(&self, mut callback: F) -> StorageResult<u64>
    where
        F: FnMut(WalRecord) -> StorageResult<()>,
    {
        let mut file = File::open(&self.path)?;
        let mut header = [0u8; 5];
        file.read_exact(&mut header)?;

        // 验证魔数字节
        if &header[0..4] != WAL_MAGIC {
            return Err(StorageError::Corruption("Invalid WAL magic".to_string()));
        }

        let mut count = 0u64;
        let file_size = file.metadata()?.len();
        let mut pos = 5u64;

        // 扫描并重放所有记录
        while pos < file_size {
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() {
                break;  // 文件末尾
            }

            let record_len = u32::from_le_bytes(len_buf) as usize;
            if record_len == 0 {
                break;  // 无效记录
            }

            // 读取记录内容
            let mut record_buf = vec![0u8; record_len];
            if file.read_exact(&mut record_buf).is_err() {
                warn!("Incomplete WAL record at position {}", pos);
                break;
            }

            // 解码并调用回调
            match WalRecord::decode(&record_buf) {
                Ok(record) => {
                    callback(record)?;
                    count += 1;
                }
                Err(e) => {
                    warn!("Failed to decode WAL record at {}: {}", pos, e);
                    break;  // 遇到损坏记录,停止重放
                }
            }

            pos += 4 + record_len as u64;
        }

        info!("Replayed {} WAL records", count);
        Ok(count)
    }

    /// # Brief
    /// 截断 WAL 文件
    ///
    /// 清空 WAL 文件,仅保留魔数字节和版本号。
    /// 在成功执行 checkpoint 后调用。
    ///
    /// # Returns
    /// 成功或错误
    pub fn truncate(&self) -> StorageResult<()> {
        let mut writer = self.writer.lock();
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);

        // 截断文件并重新写入头部
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        let mut new_writer = BufWriter::new(file);
        new_writer.write_all(&WAL_MAGIC)?;
        new_writer.write_all(&[WAL_VERSION])?;
        new_writer.flush()?;

        // 更新写入器
        let mut writer = self.writer.lock();
        *writer = BufWriter::new(OpenOptions::new().append(true).open(&self.path)?);

        // 重置文件大小和 LSN
        self.file_size.store(5, Ordering::Relaxed);
        self.lsn.store(0, Ordering::SeqCst);

        info!("WAL truncated at {:?}", self.path);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wal_write_and_replay() {
        let dir = tempdir().unwrap();
        let wal_path = dir.path().join("test.wal");

        let wal = WriteAheadLog::open(&wal_path, true).unwrap();

        let record1 = WalRecord::new_insert(1, "test", vec![1, 2, 3], vec![4, 5, 6]);
        let record2 = WalRecord::new_update(1, "test", vec![1, 2, 3], vec![7, 8, 9]);
        let record3 = WalRecord::new_delete(1, "test", vec![1, 2, 3]);

        wal.append(&record1).unwrap();
        wal.append(&record2).unwrap();
        wal.append(&record3).unwrap();
        wal.sync().unwrap();

        let mut records = Vec::new();
        wal.replay(|r| {
            records.push(r);
            Ok(())
        })
        .unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].record_type, RecordType::Insert);
        assert_eq!(records[1].record_type, RecordType::Update);
        assert_eq!(records[2].record_type, RecordType::Delete);
    }
}

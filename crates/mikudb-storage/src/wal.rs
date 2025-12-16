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

const WAL_MAGIC: [u8; 4] = [0x4D, 0x57, 0x41, 0x4C];
const WAL_VERSION: u8 = 1;
const RECORD_HEADER_SIZE: usize = 17;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    Insert = 1,
    Update = 2,
    Delete = 3,
    BeginTx = 10,
    CommitTx = 11,
    AbortTx = 12,
    Checkpoint = 20,
}

impl RecordType {
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

#[derive(Debug, Clone)]
pub struct WalRecord {
    pub record_type: RecordType,
    pub tx_id: u64,
    pub collection: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl WalRecord {
    pub fn new_insert(tx_id: u64, collection: &str, key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Insert,
            tx_id,
            collection: collection.to_string(),
            key,
            value,
        }
    }

    pub fn new_update(tx_id: u64, collection: &str, key: Vec<u8>, value: Vec<u8>) -> Self {
        Self {
            record_type: RecordType::Update,
            tx_id,
            collection: collection.to_string(),
            key,
            value,
        }
    }

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

    fn encode(&self) -> Vec<u8> {
        let collection_bytes = self.collection.as_bytes();
        let total_len = 1 + 8 + 2 + collection_bytes.len() + 4 + self.key.len() + 4 + self.value.len();

        let mut buf = BytesMut::with_capacity(total_len + 8);

        buf.put_u8(self.record_type as u8);
        buf.put_u64_le(self.tx_id);
        buf.put_u16_le(collection_bytes.len() as u16);
        buf.put_slice(collection_bytes);
        buf.put_u32_le(self.key.len() as u32);
        buf.put_slice(&self.key);
        buf.put_u32_le(self.value.len() as u32);
        buf.put_slice(&self.value);

        let checksum = xxh3_64(&buf);
        buf.put_u64_le(checksum);

        buf.to_vec()
    }

    fn decode(data: &[u8]) -> StorageResult<Self> {
        if data.len() < 20 {
            return Err(StorageError::Corruption("WAL record too small".to_string()));
        }

        let checksum_pos = data.len() - 8;
        let stored_checksum = u64::from_le_bytes(data[checksum_pos..].try_into().unwrap());
        let computed_checksum = xxh3_64(&data[..checksum_pos]);

        if stored_checksum != computed_checksum {
            return Err(StorageError::Corruption("WAL checksum mismatch".to_string()));
        }

        let mut pos = 0;

        let record_type = RecordType::from_u8(data[pos])
            .ok_or_else(|| StorageError::Corruption("Invalid record type".to_string()))?;
        pos += 1;

        let tx_id = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        pos += 8;

        let collection_len = u16::from_le_bytes(data[pos..pos + 2].try_into().unwrap()) as usize;
        pos += 2;

        let collection = String::from_utf8(data[pos..pos + collection_len].to_vec())
            .map_err(|e| StorageError::Corruption(format!("Invalid collection name: {}", e)))?;
        pos += collection_len;

        let key_len = u32::from_le_bytes(data[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;

        let key = data[pos..pos + key_len].to_vec();
        pos += key_len;

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

pub struct WriteAheadLog {
    path: PathBuf,
    writer: Mutex<BufWriter<File>>,
    lsn: AtomicU64,
    file_size: AtomicU64,
    max_file_size: u64,
    sync_on_write: bool,
}

impl WriteAheadLog {
    pub fn open(path: impl AsRef<Path>, sync_on_write: bool) -> StorageResult<Self> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)?;

        let metadata = file.metadata()?;
        let file_size = metadata.len();

        let lsn = if file_size > 0 {
            Self::recover_lsn(&path)?
        } else {
            let mut writer = BufWriter::new(file);
            writer.write_all(&WAL_MAGIC)?;
            writer.write_all(&[WAL_VERSION])?;
            writer.flush()?;
            0
        };

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

    fn recover_lsn(path: &Path) -> StorageResult<u64> {
        let mut file = File::open(path)?;
        let mut header = [0u8; 5];
        file.read_exact(&mut header)?;

        if &header[0..4] != WAL_MAGIC {
            return Err(StorageError::Corruption("Invalid WAL magic".to_string()));
        }

        let mut lsn = 0u64;
        let mut pos = 5u64;
        let file_size = file.metadata()?.len();

        while pos < file_size {
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() {
                break;
            }

            let record_len = u32::from_le_bytes(len_buf) as usize;
            if record_len == 0 || pos + 4 + record_len as u64 > file_size {
                break;
            }

            file.seek(SeekFrom::Current(record_len as i64))?;
            pos += 4 + record_len as u64;
            lsn += 1;
        }

        Ok(lsn)
    }

    pub fn append(&self, record: &WalRecord) -> StorageResult<u64> {
        let encoded = record.encode();
        let lsn = self.lsn.fetch_add(1, Ordering::SeqCst);

        let mut writer = self.writer.lock();

        let len_bytes = (encoded.len() as u32).to_le_bytes();
        writer.write_all(&len_bytes)?;
        writer.write_all(&encoded)?;

        if self.sync_on_write {
            writer.flush()?;
            writer.get_ref().sync_data()?;
        }

        self.file_size
            .fetch_add(4 + encoded.len() as u64, Ordering::Relaxed);

        Ok(lsn)
    }

    pub fn sync(&self) -> StorageResult<()> {
        let mut writer = self.writer.lock();
        writer.flush()?;
        writer.get_ref().sync_all()?;
        Ok(())
    }

    pub fn current_lsn(&self) -> u64 {
        self.lsn.load(Ordering::SeqCst)
    }

    pub fn file_size(&self) -> u64 {
        self.file_size.load(Ordering::Relaxed)
    }

    pub fn should_rotate(&self) -> bool {
        self.file_size() > self.max_file_size
    }

    pub fn rotate(&self) -> StorageResult<PathBuf> {
        let mut writer = self.writer.lock();
        writer.flush()?;
        writer.get_ref().sync_all()?;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let archive_path = self.path.with_extension(format!("wal.{}", timestamp));
        std::fs::rename(&self.path, &archive_path)?;

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&self.path)?;

        let mut new_writer = BufWriter::new(file);
        new_writer.write_all(&WAL_MAGIC)?;
        new_writer.write_all(&[WAL_VERSION])?;
        new_writer.flush()?;

        *writer = BufWriter::new(OpenOptions::new().append(true).open(&self.path)?);

        self.file_size.store(5, Ordering::Relaxed);

        info!("WAL rotated to {:?}", archive_path);
        Ok(archive_path)
    }

    pub fn replay<F>(&self, mut callback: F) -> StorageResult<u64>
    where
        F: FnMut(WalRecord) -> StorageResult<()>,
    {
        let mut file = File::open(&self.path)?;
        let mut header = [0u8; 5];
        file.read_exact(&mut header)?;

        if &header[0..4] != WAL_MAGIC {
            return Err(StorageError::Corruption("Invalid WAL magic".to_string()));
        }

        let mut count = 0u64;
        let file_size = file.metadata()?.len();
        let mut pos = 5u64;

        while pos < file_size {
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() {
                break;
            }

            let record_len = u32::from_le_bytes(len_buf) as usize;
            if record_len == 0 {
                break;
            }

            let mut record_buf = vec![0u8; record_len];
            if file.read_exact(&mut record_buf).is_err() {
                warn!("Incomplete WAL record at position {}", pos);
                break;
            }

            match WalRecord::decode(&record_buf) {
                Ok(record) => {
                    callback(record)?;
                    count += 1;
                }
                Err(e) => {
                    warn!("Failed to decode WAL record at {}: {}", pos, e);
                    break;
                }
            }

            pos += 4 + record_len as u64;
        }

        info!("Replayed {} WAL records", count);
        Ok(count)
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

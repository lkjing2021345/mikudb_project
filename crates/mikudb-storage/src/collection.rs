//! 集合模块
//!
//! 提供文档集合的 CRUD 操作，包括批量操作和迭代器支持。

use crate::{StorageError, StorageResult};
use mikudb_boml::{codec, Document, BomlValue};
use mikudb_common::ObjectId;
use parking_lot::RwLock;
use rocksdb::{BoundColumnFamily, IteratorMode, ReadOptions, WriteBatch, WriteOptions, DB};
use std::sync::Arc;
use tracing::{debug, trace};

/// 文档集合
///
/// 表示一个文档集合，对应 RocksDB 的一个 Column Family
pub struct Collection {
    name: String,
    db: Arc<DB>,
    stats: RwLock<CollectionStats>,
}

#[derive(Debug, Default)]
struct CollectionStats {
    doc_count: u64,
    total_size: u64,
    insert_count: u64,
    update_count: u64,
    delete_count: u64,
}

impl Collection {
    /// 创建新集合
    ///
    /// # Brief
    /// 创建一个新的集合实例
    ///
    /// # Arguments
    /// * `name` - 集合名称
    /// * `db` - RocksDB 实例的 Arc 引用
    ///
    /// # Returns
    /// 新的 Collection 实例
    pub fn new(name: String, db: Arc<DB>) -> Self {
        Self {
            name,
            db,
            stats: RwLock::new(CollectionStats::default()),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    fn cf(&self) -> StorageResult<Arc<BoundColumnFamily>> {
        self.db
            .cf_handle(&self.name)
            .ok_or_else(|| StorageError::CollectionNotFound(self.name.clone()))
    }

    fn doc_key(id: &ObjectId) -> Vec<u8> {
        let mut key = Vec::with_capacity(13);
        key.push(b'd');
        key.extend_from_slice(id.as_bytes());
        key
    }

    fn id_from_key(key: &[u8]) -> Option<ObjectId> {
        if key.len() == 13 && key[0] == b'd' {
            let mut bytes = [0u8; 12];
            bytes.copy_from_slice(&key[1..13]);
            Some(ObjectId::from_bytes(bytes))
        } else {
            None
        }
    }

    /// 插入文档
    ///
    /// # Brief
    /// 插入一个新文档到集合中
    ///
    /// # Arguments
    /// * `doc` - 要插入的文档（会自动生成 ID）
    ///
    /// # Returns
    /// 成功返回文档的 ObjectId，如果文档已存在则返回错误
    pub fn insert(&self, doc: &mut Document) -> StorageResult<ObjectId> {
        let id = *doc.ensure_id();
        let key = Self::doc_key(&id);

        let cf = self.cf()?;

        let existing = self.db.get_cf(&cf, &key)?;
        if existing.is_some() {
            return Err(StorageError::DocumentExists(id.to_string()));
        }

        let value = codec::encode_document(&doc.to_boml_value())?;

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);

        self.db.put_cf_opt(&cf, &key, &value, &write_opts)?;

        let mut stats = self.stats.write();
        stats.doc_count += 1;
        stats.total_size += value.len() as u64;
        stats.insert_count += 1;

        trace!("Inserted document {} into {}", id, self.name);
        Ok(id)
    }

    /// 批量插入文档
    ///
    /// # Brief
    /// 使用 WriteBatch 批量插入多个文档，性能更高
    ///
    /// # Arguments
    /// * `docs` - 要插入的文档切片
    ///
    /// # Returns
    /// 成功返回所有文档的 ObjectId 向量
    pub fn insert_many(&self, docs: &mut [Document]) -> StorageResult<Vec<ObjectId>> {
        let cf = self.cf()?;
        let mut batch = WriteBatch::default();
        let mut ids = Vec::with_capacity(docs.len());
        let mut total_size = 0u64;

        for doc in docs.iter_mut() {
            let id = *doc.ensure_id();
            let key = Self::doc_key(&id);
            let value = codec::encode_document(&doc.to_boml_value())?;

            batch.put_cf(&cf, &key, &value);
            total_size += value.len() as u64;
            ids.push(id);
        }

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);

        self.db.write_opt(batch, &write_opts)?;

        let mut stats = self.stats.write();
        stats.doc_count += ids.len() as u64;
        stats.total_size += total_size;
        stats.insert_count += ids.len() as u64;

        debug!("Inserted {} documents into {}", ids.len(), self.name);
        Ok(ids)
    }

    /// 获取文档
    ///
    /// # Brief
    /// 根据 ID 获取单个文档
    ///
    /// # Arguments
    /// * `id` - 文档的 ObjectId
    ///
    /// # Returns
    /// `Some(Document)` 如果文档存在，否则 `None`
    pub fn get(&self, id: &ObjectId) -> StorageResult<Option<Document>> {
        let cf = self.cf()?;
        let key = Self::doc_key(id);

        let mut read_opts = ReadOptions::default();
        read_opts.set_verify_checksums(true);

        match self.db.get_cf_opt(&cf, &key, &read_opts)? {
            Some(data) => {
                let value = codec::decode_document(&data)?;
                let doc = Document::from_boml_value(value)?;
                Ok(Some(doc))
            }
            None => Ok(None),
        }
    }

    /// 更新文档
    ///
    /// # Brief
    /// 更新指定 ID 的文档
    ///
    /// # Arguments
    /// * `id` - 文档的 ObjectId
    /// * `doc` - 新的文档内容
    ///
    /// # Returns
    /// 成功返回 Ok(()), 如果文档不存在则返回错误
    pub fn update(&self, id: &ObjectId, doc: &Document) -> StorageResult<()> {
        let cf = self.cf()?;
        let key = Self::doc_key(id);

        let existing = self.db.get_cf(&cf, &key)?;
        if existing.is_none() {
            return Err(StorageError::DocumentNotFound(id.to_string()));
        }

        let value = codec::encode_document(&doc.to_boml_value())?;

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);

        self.db.put_cf_opt(&cf, &key, &value, &write_opts)?;

        let mut stats = self.stats.write();
        stats.update_count += 1;

        trace!("Updated document {} in {}", id, self.name);
        Ok(())
    }

    /// 插入或更新文档
    ///
    /// # Brief
    /// 如果文档存在则更新，不存在则插入
    ///
    /// # Arguments
    /// * `doc` - 要插入或更新的文档
    ///
    /// # Returns
    /// 返回文档的 ObjectId
    pub fn upsert(&self, doc: &mut Document) -> StorageResult<ObjectId> {
        let id = *doc.ensure_id();
        let cf = self.cf()?;
        let key = Self::doc_key(&id);

        let value = codec::encode_document(&doc.to_boml_value())?;

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);

        let existing = self.db.get_cf(&cf, &key)?;
        self.db.put_cf_opt(&cf, &key, &value, &write_opts)?;

        let mut stats = self.stats.write();
        if existing.is_some() {
            stats.update_count += 1;
        } else {
            stats.doc_count += 1;
            stats.insert_count += 1;
        }
        stats.total_size += value.len() as u64;

        Ok(id)
    }

    /// 删除文档
    ///
    /// # Brief
    /// 根据 ID 删除文档
    ///
    /// # Arguments
    /// * `id` - 文档的 ObjectId
    ///
    /// # Returns
    /// 删除成功返回 `true`，文档不存在返回 `false`
    pub fn delete(&self, id: &ObjectId) -> StorageResult<bool> {
        let cf = self.cf()?;
        let key = Self::doc_key(id);

        let existing = self.db.get_cf(&cf, &key)?;
        if existing.is_none() {
            return Ok(false);
        }

        let mut write_opts = WriteOptions::default();
        write_opts.set_sync(false);

        self.db.delete_cf_opt(&cf, &key, &write_opts)?;

        let mut stats = self.stats.write();
        stats.doc_count = stats.doc_count.saturating_sub(1);
        stats.delete_count += 1;

        trace!("Deleted document {} from {}", id, self.name);
        Ok(true)
    }

    /// 批量删除文档
    ///
    /// # Brief
    /// 使用 WriteBatch 批量删除多个文档
    ///
    /// # Arguments
    /// * `ids` - 要删除的 ObjectId 切片
    ///
    /// # Returns
    /// 实际删除的文档数量
    pub fn delete_many(&self, ids: &[ObjectId]) -> StorageResult<u64> {
        let cf = self.cf()?;
        let mut batch = WriteBatch::default();
        let mut count = 0u64;

        for id in ids {
            let key = Self::doc_key(id);
            if self.db.get_cf(&cf, &key)?.is_some() {
                batch.delete_cf(&cf, &key);
                count += 1;
            }
        }

        if count > 0 {
            let mut write_opts = WriteOptions::default();
            write_opts.set_sync(false);
            self.db.write_opt(batch, &write_opts)?;

            let mut stats = self.stats.write();
            stats.doc_count = stats.doc_count.saturating_sub(count);
            stats.delete_count += count;
        }

        debug!("Deleted {} documents from {}", count, self.name);
        Ok(count)
    }

    /// 查找所有文档
    ///
    /// # Brief
    /// 返回集合中的所有文档
    ///
    /// # Returns
    /// 文档向量
    pub fn find_all(&self) -> StorageResult<Vec<Document>> {
        let cf = self.cf()?;
        let mut docs = Vec::new();

        let prefix = [b'd'];
        let iter = self.db.prefix_iterator_cf(&cf, &prefix);

        for item in iter {
            let (key, value) = item?;
            if key.len() == 13 && key[0] == b'd' {
                let boml_value = codec::decode_document(&value)?;
                let doc = Document::from_boml_value(boml_value)?;
                docs.push(doc);
            }
        }

        Ok(docs)
    }

    /// 根据 ID 列表查找文档
    ///
    /// # Brief
    /// 使用 multi_get 批量获取多个文档，性能更高
    ///
    /// # Arguments
    /// * `ids` - ObjectId 列表
    ///
    /// # Returns
    /// 找到的文档向量
    pub fn find_by_ids(&self, ids: &[ObjectId]) -> StorageResult<Vec<Document>> {
        let cf = self.cf()?;
        let mut docs = Vec::with_capacity(ids.len());

        for id in ids {
            let key = Self::doc_key(id);
            if let Some(data) = self.db.get_cf(&cf, &key)? {
                let boml_value = codec::decode_document(&data)?;
                let doc = Document::from_boml_value(boml_value)?;
                docs.push(doc);
            }
        }

        Ok(docs)
    }

    /// 获取文档数量（从缓存）
    ///
    /// # Brief
    /// 返回缓存的文档数量，性能高但可能不精确
    ///
    /// # Returns
    /// 文档数量
    pub fn count(&self) -> StorageResult<u64> {
        Ok(self.stats.read().doc_count)
    }

    /// 获取文档数量（扫描统计）
    ///
    /// # Brief
    /// 通过扫描计算精确的文档数量，性能较低
    ///
    /// # Returns
    /// 精确的文档数量
    pub fn count_scan(&self) -> StorageResult<u64> {
        let cf = self.cf()?;
        let prefix = [b'd'];
        let iter = self.db.prefix_iterator_cf(&cf, &prefix);
        let count = iter.count() as u64;
        Ok(count)
    }

    /// 检查文档是否存在
    ///
    /// # Brief
    /// 根据 ID 检查文档是否存在
    ///
    /// # Arguments
    /// * `id` - 文档的 ObjectId
    ///
    /// # Returns
    /// 存在返回 `true`
    pub fn exists(&self, id: &ObjectId) -> StorageResult<bool> {
        let cf = self.cf()?;
        let key = Self::doc_key(id);
        Ok(self.db.get_cf(&cf, &key)?.is_some())
    }

    /// 清空集合
    ///
    /// # Brief
    /// 删除集合中的所有文档
    ///
    /// # Returns
    /// 删除的文档数量
    pub fn clear(&self) -> StorageResult<u64> {
        let cf = self.cf()?;
        let prefix = [b'd'];
        let iter = self.db.prefix_iterator_cf(&cf, &prefix);

        let mut batch = WriteBatch::default();
        let mut count = 0u64;

        for item in iter {
            let (key, _) = item?;
            batch.delete_cf(&cf, &key);
            count += 1;
        }

        if count > 0 {
            self.db.write(batch)?;

            let mut stats = self.stats.write();
            stats.doc_count = 0;
            stats.total_size = 0;
        }

        debug!("Cleared {} documents from {}", count, self.name);
        Ok(count)
    }

    /// 获取集合迭代器
    ///
    /// # Brief
    /// 返回集合的文档迭代器，用于逐个遍历文档
    ///
    /// # Returns
    /// CollectionIterator 实例
    pub fn iter(&self) -> StorageResult<CollectionIterator> {
        let cf = self.cf()?;
        Ok(CollectionIterator {
            inner: self.db.prefix_iterator_cf(&cf, [b'd']),
        })
    }

    /// 获取集合统计信息
    ///
    /// # Brief
    /// 返回集合的统计快照
    ///
    /// # Returns
    /// CollectionStatsSnapshot 实例
    pub fn stats(&self) -> CollectionStatsSnapshot {
        let stats = self.stats.read();
        CollectionStatsSnapshot {
            name: self.name.clone(),
            doc_count: stats.doc_count,
            total_size: stats.total_size,
            insert_count: stats.insert_count,
            update_count: stats.update_count,
            delete_count: stats.delete_count,
        }
    }
}

/// 集合文档迭代器
///
/// 用于逐个遍历集合中的文档
pub struct CollectionIterator<'a> {
    inner: rocksdb::DBIteratorWithThreadMode<'a, DB>,
}

impl<'a> Iterator for CollectionIterator<'a> {
    type Item = StorageResult<Document>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(Ok((key, value))) => {
                    if key.len() == 13 && key[0] == b'd' {
                        match codec::decode_document(&value) {
                            Ok(boml_value) => match Document::from_boml_value(boml_value) {
                                Ok(doc) => return Some(Ok(doc)),
                                Err(e) => return Some(Err(StorageError::Boml(e))),
                            },
                            Err(e) => return Some(Err(StorageError::Boml(e))),
                        }
                    }
                }
                Some(Err(e)) => return Some(Err(StorageError::RocksDb(e))),
                None => return None,
            }
        }
    }
}

/// 集合统计信息快照
///
/// 包含集合的各种统计数据
#[derive(Debug, Clone)]
pub struct CollectionStatsSnapshot {
    pub name: String,
    pub doc_count: u64,
    pub total_size: u64,
    pub insert_count: u64,
    pub update_count: u64,
    pub delete_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{StorageEngine, StorageOptions};
    use tempfile::tempdir;

    fn setup() -> (StorageEngine, Arc<Collection>) {
        let dir = tempdir().unwrap();
        let options = StorageOptions {
            data_dir: dir.into_path(),
            ..Default::default()
        };
        let engine = StorageEngine::open(options).unwrap();
        let collection = engine.create_collection("test").unwrap();
        (engine, collection)
    }

    #[test]
    fn test_insert_and_get() {
        let (_engine, collection) = setup();

        let mut doc = Document::new();
        doc.insert("name", "test");
        doc.insert("value", 42);

        let id = collection.insert(&mut doc).unwrap();
        let retrieved = collection.get(&id).unwrap().unwrap();

        assert_eq!(retrieved.get_str("name"), Some("test"));
        assert_eq!(retrieved.get_i32("value"), Some(42));
    }

    #[test]
    fn test_update() {
        let (_engine, collection) = setup();

        let mut doc = Document::new();
        doc.insert("name", "original");
        let id = collection.insert(&mut doc).unwrap();

        let mut updated = Document::with_id(id);
        updated.insert("name", "updated");
        collection.update(&id, &updated).unwrap();

        let retrieved = collection.get(&id).unwrap().unwrap();
        assert_eq!(retrieved.get_str("name"), Some("updated"));
    }

    #[test]
    fn test_delete() {
        let (_engine, collection) = setup();

        let mut doc = Document::new();
        doc.insert("name", "to_delete");
        let id = collection.insert(&mut doc).unwrap();

        assert!(collection.delete(&id).unwrap());
        assert!(collection.get(&id).unwrap().is_none());
    }

    #[test]
    fn test_insert_many() {
        let (_engine, collection) = setup();

        let mut docs: Vec<Document> = (0..100)
            .map(|i| {
                let mut doc = Document::new();
                doc.insert("index", i);
                doc
            })
            .collect();

        let ids = collection.insert_many(&mut docs).unwrap();
        assert_eq!(ids.len(), 100);

        let all = collection.find_all().unwrap();
        assert_eq!(all.len(), 100);
    }
}

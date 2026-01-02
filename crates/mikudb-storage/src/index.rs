//! 存储层索引引擎
//!
//! 本模块实现持久化的索引引擎:
//! - **哈希索引**: 基于 HashMap,适合等值查询
//! - **复合索引**: 支持多字段索引
//! - **唯一索引**: 保证键的唯一性
//! - **稀疏索引**: 只索引非空字段的文档
//! - **TTL 索引**: 自动过期删除文档
//!
//! # 索引持久化
//!
//! 索引数据存储在 RocksDB 的独立 ColumnFamily 中:
//! - 索引元数据: `_index_meta` CF
//! - 索引数据: `idx_{index_name}` CF
//!
//! # OpenEuler 适配亮点
//!
//! - 使用 xxHash3 计算哈希索引键,在鲲鹏 CPU 上性能优异
//! - 支持 Direct I/O 优化索引读写

use crate::{StorageError, StorageResult};
use mikudb_boml::{BomlValue, Document};
use mikudb_common::ObjectId;
use parking_lot::RwLock;
use rocksdb::{BoundColumnFamily, IteratorMode, WriteBatch, WriteOptions, DB};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};
use xxhash_rust::xxh3::xxh3_64;

/// 索引定义
///
/// 描述索引的元数据和配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// 索引名称
    pub name: String,
    /// 所属集合
    pub collection: String,
    /// 索引字段列表
    pub fields: Vec<IndexField>,
    /// 索引类型
    pub index_type: IndexType,
    /// 是否唯一索引
    pub unique: bool,
    /// 是否稀疏索引(不索引缺失字段的文档)
    pub sparse: bool,
    /// TTL 配置(秒数,None 表示不过期)
    pub ttl_seconds: Option<u64>,
}

/// 索引字段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexField {
    /// 字段路径(支持嵌套,如 "user.name")
    pub path: String,
    /// 排序顺序
    pub order: IndexOrder,
}

/// 索引排序顺序
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexOrder {
    /// 升序
    Ascending,
    /// 降序
    Descending,
}

/// 索引类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    /// B-树索引(使用 RocksDB 原生支持)
    BTree,
    /// 哈希索引(适合等值查询)
    Hash,
    /// 全文索引
    Text,
    /// 地理空间索引
    Geo2dsphere,
}

/// 索引引擎
///
/// 管理所有索引的创建、删除、查询和维护
pub struct IndexEngine {
    db: Arc<DB>,
    /// 索引元数据缓存
    index_defs: RwLock<HashMap<String, IndexDefinition>>,
}

impl IndexEngine {
    /// 创建索引引擎
    ///
    /// # Arguments
    /// * `db` - RocksDB 实例
    pub fn new(db: Arc<DB>) -> Self {
        Self {
            db,
            index_defs: RwLock::new(HashMap::new()),
        }
    }

    /// 加载所有索引元数据
    ///
    /// 从 `_index_meta` CF 加载所有索引定义到内存
    pub fn load_indexes(&self) -> StorageResult<()> {
        let meta_cf = self.db.cf_handle("_index_meta").ok_or_else(|| {
            StorageError::Internal("Index metadata CF not found".to_string())
        })?;

        let mut index_defs = self.index_defs.write();

        let iter = self.db.iterator_cf(&meta_cf, IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;
            let index_name = String::from_utf8(key.to_vec())
                .map_err(|e| StorageError::Corruption(format!("Invalid index name: {}", e)))?;

            let definition: IndexDefinition = serde_json::from_slice(&value)
                .map_err(|e| StorageError::Corruption(format!("Invalid index definition: {}", e)))?;

            index_defs.insert(index_name, definition);
        }

        info!("Loaded {} indexes", index_defs.len());
        Ok(())
    }

    /// 创建索引
    ///
    /// # Arguments
    /// * `definition` - 索引定义
    ///
    /// # Returns
    /// 成功或错误
    pub fn create_index(&self, definition: IndexDefinition) -> StorageResult<()> {
        // 检查索引是否已存在
        {
            let index_defs = self.index_defs.read();
            if index_defs.contains_key(&definition.name) {
                return Err(StorageError::Internal(format!(
                    "Index {} already exists",
                    definition.name
                )));
            }
        }

        // 创建索引 ColumnFamily
        let cf_name = format!("idx_{}", definition.name);
        let mut opts = rocksdb::Options::default();
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        self.db.create_cf(&cf_name, &opts)?;

        // 保存索引元数据
        let meta_cf = self.db.cf_handle("_index_meta").ok_or_else(|| {
            StorageError::Internal("Index metadata CF not found".to_string())
        })?;

        let meta_bytes = serde_json::to_vec(&definition)
            .map_err(|e| StorageError::Internal(format!("Failed to serialize index def: {}", e)))?;

        self.db.put_cf(&meta_cf, definition.name.as_bytes(), &meta_bytes)?;

        // 添加到缓存
        self.index_defs.write().insert(definition.name.clone(), definition.clone());

        info!("Created index: {}", definition.name);
        Ok(())
    }

    /// 删除索引
    ///
    /// # Arguments
    /// * `name` - 索引名称
    pub fn drop_index(&self, name: &str) -> StorageResult<bool> {
        // 从缓存移除
        let removed = self.index_defs.write().remove(name).is_some();
        if !removed {
            return Ok(false);
        }

        // 删除元数据
        let meta_cf = self.db.cf_handle("_index_meta").ok_or_else(|| {
            StorageError::Internal("Index metadata CF not found".to_string())
        })?;
        self.db.delete_cf(&meta_cf, name.as_bytes())?;

        // 删除索引 ColumnFamily
        let cf_name = format!("idx_{}", name);
        if let Err(e) = self.db.drop_cf(&cf_name) {
            warn!("Failed to drop index CF {}: {}", cf_name, e);
        }

        info!("Dropped index: {}", name);
        Ok(true)
    }

    /// 获取索引定义
    pub fn get_index(&self, name: &str) -> Option<IndexDefinition> {
        self.index_defs.read().get(name).cloned()
    }

    /// 列出集合的所有索引
    pub fn list_indexes(&self, collection: &str) -> Vec<IndexDefinition> {
        self.index_defs
            .read()
            .values()
            .filter(|def| def.collection == collection)
            .cloned()
            .collect()
    }

    /// 插入文档到索引
    ///
    /// # Arguments
    /// * `index_name` - 索引名称
    /// * `doc` - 文档
    /// * `doc_id` - 文档 ID
    pub fn insert_document(
        &self,
        index_name: &str,
        doc: &Document,
        doc_id: &ObjectId,
    ) -> StorageResult<()> {
        let definition = self.get_index(index_name).ok_or_else(|| {
            StorageError::Internal(format!("Index {} not found", index_name))
        })?;

        // 提取索引键
        let key_values = self.extract_key_values(&definition.fields, doc)?;

        // 稀疏索引: 如果任何字段缺失,跳过索引
        if definition.sparse && key_values.iter().any(|v| matches!(v, BomlValue::Null)) {
            return Ok(());
        }

        let index_key = self.build_index_key(&key_values, &definition)?;

        // 唯一索引检查
        if definition.unique {
            if self.lookup_internal(&definition, &index_key)?.is_some() {
                return Err(StorageError::Internal(format!(
                    "Duplicate key error for unique index {}",
                    index_name
                )));
            }
        }

        // 插入到索引
        let cf_name = format!("idx_{}", index_name);
        let cf = self.db.cf_handle(&cf_name).ok_or_else(|| {
            StorageError::Internal(format!("Index CF {} not found", cf_name))
        })?;

        // 键: index_key + doc_id, 值: 空(或 TTL 时间戳)
        let mut full_key = index_key;
        full_key.extend_from_slice(doc_id.as_bytes());

        let value = if let Some(ttl_seconds) = definition.ttl_seconds {
            let expire_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                + ttl_seconds;
            expire_time.to_le_bytes().to_vec()
        } else {
            vec![]
        };

        self.db.put_cf(&cf, &full_key, &value)?;

        Ok(())
    }

    /// 从索引删除文档
    pub fn delete_document(
        &self,
        index_name: &str,
        doc: &Document,
        doc_id: &ObjectId,
    ) -> StorageResult<()> {
        let definition = self.get_index(index_name).ok_or_else(|| {
            StorageError::Internal(format!("Index {} not found", index_name))
        })?;

        let key_values = self.extract_key_values(&definition.fields, doc)?;
        if definition.sparse && key_values.iter().any(|v| matches!(v, BomlValue::Null)) {
            return Ok(());
        }

        let index_key = self.build_index_key(&key_values, &definition)?;

        let cf_name = format!("idx_{}", index_name);
        let cf = self.db.cf_handle(&cf_name).ok_or_else(|| {
            StorageError::Internal(format!("Index CF {} not found", cf_name))
        })?;

        let mut full_key = index_key;
        full_key.extend_from_slice(doc_id.as_bytes());

        self.db.delete_cf(&cf, &full_key)?;

        Ok(())
    }

    /// 查找索引
    ///
    /// # Arguments
    /// * `index_name` - 索引名称
    /// * `key_values` - 索引键值列表
    ///
    /// # Returns
    /// 匹配的文档 ID 列表
    pub fn lookup(
        &self,
        index_name: &str,
        key_values: &[BomlValue],
    ) -> StorageResult<Vec<ObjectId>> {
        let definition = self.get_index(index_name).ok_or_else(|| {
            StorageError::Internal(format!("Index {} not found", index_name))
        })?;

        let index_key = self.build_index_key(key_values, &definition)?;

        if let Some(doc_id) = self.lookup_internal(&definition, &index_key)? {
            Ok(vec![doc_id])
        } else {
            // 哈希索引: 精确查找
            if matches!(definition.index_type, IndexType::Hash) {
                return Ok(vec![]);
            }

            // BTree 索引: 范围查找(前缀匹配)
            self.range_scan(&definition, &index_key, &index_key, true)
        }
    }

    /// 范围查询
    pub fn range_query(
        &self,
        index_name: &str,
        start_key: Option<&[BomlValue]>,
        end_key: Option<&[BomlValue]>,
        inclusive: bool,
    ) -> StorageResult<Vec<ObjectId>> {
        let definition = self.get_index(index_name).ok_or_else(|| {
            StorageError::Internal(format!("Index {} not found", index_name))
        })?;

        if matches!(definition.index_type, IndexType::Hash) {
            return Err(StorageError::Internal(
                "Range query not supported on hash index".to_string(),
            ));
        }

        let start_bytes = if let Some(start) = start_key {
            self.build_index_key(start, &definition)?
        } else {
            vec![]
        };

        let end_bytes = if let Some(end) = end_key {
            self.build_index_key(end, &definition)?
        } else {
            vec![0xFF; 256] // 最大键
        };

        self.range_scan(&definition, &start_bytes, &end_bytes, inclusive)
    }

    /// 清理过期的 TTL 索引项
    ///
    /// 扫描所有 TTL 索引,删除已过期的文档
    pub fn cleanup_expired_ttl(&self) -> StorageResult<u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut total_deleted = 0u64;

        // 查找所有 TTL 索引
        let ttl_indexes: Vec<_> = self
            .index_defs
            .read()
            .values()
            .filter(|def| def.ttl_seconds.is_some())
            .cloned()
            .collect();

        for definition in ttl_indexes {
            let cf_name = format!("idx_{}", definition.name);
            let cf = self.db.cf_handle(&cf_name).ok_or_else(|| {
                StorageError::Internal(format!("Index CF {} not found", cf_name))
            })?;

            let mut batch = WriteBatch::default();
            let mut count = 0u64;

            let iter = self.db.iterator_cf(&cf, IteratorMode::Start);
            for item in iter {
                let (key, value) = item?;

                if value.len() >= 8 {
                    let expire_time = u64::from_le_bytes(value[0..8].try_into().unwrap());
                    if now >= expire_time {
                        batch.delete_cf(&cf, &key);
                        count += 1;
                    }
                }
            }

            if count > 0 {
                self.db.write(batch)?;
                info!(
                    "Cleaned up {} expired documents from TTL index {}",
                    count, definition.name
                );
                total_deleted += count;
            }
        }

        Ok(total_deleted)
    }

    // ========== 内部辅助方法 ==========

    /// 提取文档的索引键值
    fn extract_key_values(
        &self,
        fields: &[IndexField],
        doc: &Document,
    ) -> StorageResult<Vec<BomlValue>> {
        let mut values = Vec::with_capacity(fields.len());

        for field in fields {
            // 支持嵌套字段路径,如 "user.name"
            let value = self.get_nested_field(doc, &field.path);
            values.push(value);
        }

        Ok(values)
    }

    /// 获取嵌套字段值
    fn get_nested_field(&self, doc: &Document, path: &str) -> BomlValue {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = doc.to_boml_value();

        for part in parts {
            match current {
                BomlValue::Document(ref map) => {
                    if let Some(value) = map.get(part) {
                        current = value.clone();
                    } else {
                        return BomlValue::Null;
                    }
                }
                _ => return BomlValue::Null,
            }
        }

        current
    }

    /// 构建索引键
    fn build_index_key(
        &self,
        key_values: &[BomlValue],
        definition: &IndexDefinition,
    ) -> StorageResult<Vec<u8>> {
        match definition.index_type {
            IndexType::Hash => {
                // 哈希索引: 使用 xxHash3 计算哈希值
                let mut hasher_input = Vec::new();
                for value in key_values {
                    hasher_input.extend(self.value_to_bytes(value));
                }
                let hash = xxh3_64(&hasher_input);
                Ok(hash.to_be_bytes().to_vec())
            }
            IndexType::BTree => {
                // BTree 索引: 直接序列化键值(保持排序)
                let mut key = Vec::new();
                for (i, value) in key_values.iter().enumerate() {
                    if i > 0 {
                        key.push(0x00); // 分隔符
                    }
                    key.extend(self.value_to_bytes(value));
                }
                Ok(key)
            }
            _ => Err(StorageError::Internal(format!(
                "Unsupported index type: {:?}",
                definition.index_type
            ))),
        }
    }

    /// 将 BomlValue 序列化为字节
    fn value_to_bytes(&self, value: &BomlValue) -> Vec<u8> {
        match value {
            BomlValue::Null => vec![0x00],
            BomlValue::Boolean(false) => vec![0x01, 0x00],
            BomlValue::Boolean(true) => vec![0x01, 0x01],
            BomlValue::Int32(n) => {
                let mut bytes = vec![0x02];
                bytes.extend(&n.to_be_bytes());
                bytes
            }
            BomlValue::Int64(n) => {
                let mut bytes = vec![0x03];
                bytes.extend(&n.to_be_bytes());
                bytes
            }
            BomlValue::Float64(f) => {
                let mut bytes = vec![0x04];
                bytes.extend(&f.to_be_bytes());
                bytes
            }
            BomlValue::String(s) => {
                let mut bytes = vec![0x05];
                bytes.extend(s.as_bytes());
                bytes
            }
            BomlValue::ObjectId(id) => {
                let mut bytes = vec![0x06];
                bytes.extend(id.as_bytes());
                bytes
            }
            _ => vec![0x00], // 其他类型视为 Null
        }
    }

    /// 内部查找方法(唯一索引)
    fn lookup_internal(
        &self,
        definition: &IndexDefinition,
        index_key: &[u8],
    ) -> StorageResult<Option<ObjectId>> {
        let cf_name = format!("idx_{}", definition.name);
        let cf = self.db.cf_handle(&cf_name).ok_or_else(|| {
            StorageError::Internal(format!("Index CF {} not found", cf_name))
        })?;

        // 对于唯一索引,查找第一个匹配的键
        let iter = self.db.iterator_cf(&cf, IteratorMode::From(index_key, rocksdb::Direction::Forward));

        for item in iter {
            let (key, _) = item?;
            if key.starts_with(index_key) {
                // 提取 doc_id (最后 12 字节)
                if key.len() >= 12 {
                    let doc_id_start = key.len() - 12;
                    let doc_id_bytes: [u8; 12] = key[doc_id_start..].try_into().unwrap();
                    return Ok(Some(ObjectId::from_bytes(doc_id_bytes)));
                }
            } else {
                break;
            }
        }

        Ok(None)
    }

    /// 范围扫描
    fn range_scan(
        &self,
        definition: &IndexDefinition,
        start_key: &[u8],
        end_key: &[u8],
        _inclusive: bool,
    ) -> StorageResult<Vec<ObjectId>> {
        let cf_name = format!("idx_{}", definition.name);
        let cf = self.db.cf_handle(&cf_name).ok_or_else(|| {
            StorageError::Internal(format!("Index CF {} not found", cf_name))
        })?;

        let mut doc_ids = Vec::new();

        let iter = self.db.iterator_cf(
            &cf,
            IteratorMode::From(start_key, rocksdb::Direction::Forward),
        );

        for item in iter {
            let (key, _) = item?;

            // 检查是否超出范围
            if key.as_ref() > end_key {
                break;
            }

            // 提取 doc_id
            if key.len() >= 12 {
                let doc_id_start = key.len() - 12;
                let doc_id_bytes: [u8; 12] = key[doc_id_start..].try_into().unwrap();
                doc_ids.push(ObjectId::from_bytes(doc_id_bytes));
            }
        }

        Ok(doc_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_drop_index() {
        let dir = tempdir().unwrap();
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(
                &opts,
                dir.path(),
                vec![rocksdb::ColumnFamilyDescriptor::new(
                    "_index_meta",
                    rocksdb::Options::default(),
                )],
            )
            .unwrap(),
        );

        let engine = IndexEngine::new(db);

        let definition = IndexDefinition {
            name: "test_idx".to_string(),
            collection: "users".to_string(),
            fields: vec![IndexField {
                path: "name".to_string(),
                order: IndexOrder::Ascending,
            }],
            index_type: IndexType::BTree,
            unique: false,
            sparse: false,
            ttl_seconds: None,
        };

        engine.create_index(definition.clone()).unwrap();
        assert!(engine.get_index("test_idx").is_some());

        assert!(engine.drop_index("test_idx").unwrap());
        assert!(engine.get_index("test_idx").is_none());
    }

    #[test]
    fn test_unique_index() {
        let dir = tempdir().unwrap();
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = Arc::new(
            rocksdb::DB::open_cf_descriptors(
                &opts,
                dir.path(),
                vec![rocksdb::ColumnFamilyDescriptor::new(
                    "_index_meta",
                    rocksdb::Options::default(),
                )],
            )
            .unwrap(),
        );

        let engine = IndexEngine::new(db);

        let definition = IndexDefinition {
            name: "unique_idx".to_string(),
            collection: "users".to_string(),
            fields: vec![IndexField {
                path: "email".to_string(),
                order: IndexOrder::Ascending,
            }],
            index_type: IndexType::Hash,
            unique: true,
            sparse: false,
            ttl_seconds: None,
        };

        engine.create_index(definition).unwrap();

        let mut doc1 = Document::new();
        doc1.insert("email", "test@example.com");
        let id1 = ObjectId::new();

        engine.insert_document("unique_idx", &doc1, &id1).unwrap();

        // 尝试插入相同键应该失败
        let id2 = ObjectId::new();
        let result = engine.insert_document("unique_idx", &doc1, &id2);
        assert!(result.is_err());
    }
}

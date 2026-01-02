//! 索引模块
//!
//! 本模块实现数据库索引功能:
//! - 索引定义和元数据
//! - BTree 索引实现
//! - 索引键的序列化和排序
//! - 索引管理器(创建、删除、查询索引)
//!
//! 支持的索引类型:
//! - BTree: B-树索引,支持范围查询和排序
//! - Hash: 哈希索引(使用 BTree 实现)
//! - Text: 全文索引(待实现)
//! - Geo2d/Geo2dsphere: 地理空间索引(待实现)
//!
//! 索引键序列化格式:
//! - Null: 0x00
//! - Boolean: 0x01 + 值
//! - Integer: 0x02 + 大端字节序
//! - String: 0x03 + UTF-8 字节
//! - Binary: 0x04 + 原始字节

use crate::{QueryError, QueryResult};
use mikudb_boml::BomlValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

/// 索引定义
///
/// 描述索引的元数据和配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    /// 索引名称
    pub name: String,
    /// 所属集合
    pub collection: String,
    /// 索引字段列表
    pub fields: Vec<IndexField>,
    /// 是否唯一索引
    pub unique: bool,
    /// 是否稀疏索引(不索引缺失字段的文档)
    pub sparse: bool,
    /// 索引类型
    pub index_type: IndexType,
}

/// 索引字段
///
/// 定义索引中的单个字段及其排序顺序。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexField {
    /// 字段名称
    pub name: String,
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
    /// B-树索引(适合范围查询)
    BTree,
    /// 哈希索引(适合等值查询)
    Hash,
    /// 全文索引
    Text,
    /// 2D 地理空间索引
    Geo2d,
    /// 球面地理空间索引
    Geo2dsphere,
}

/// 索引 trait
///
/// 定义索引的通用接口。
pub trait Index: Send + Sync {
    /// # Brief
    /// 获取索引名称
    fn name(&self) -> &str;

    /// # Brief
    /// 获取索引定义
    fn definition(&self) -> &IndexDefinition;

    /// # Brief
    /// 插入索引项
    ///
    /// # Arguments
    /// * `key` - 索引键
    /// * `doc_id` - 文档 ID
    fn insert(&self, key: IndexKey, doc_id: &[u8]) -> QueryResult<()>;

    /// # Brief
    /// 删除索引项
    ///
    /// # Arguments
    /// * `key` - 索引键
    /// * `doc_id` - 文档 ID
    ///
    /// # Returns
    /// 是否删除成功
    fn delete(&self, key: &IndexKey, doc_id: &[u8]) -> QueryResult<bool>;

    /// # Brief
    /// 精确查找
    ///
    /// # Arguments
    /// * `key` - 索引键
    ///
    /// # Returns
    /// 匹配的文档 ID 列表
    fn lookup(&self, key: &IndexKey) -> QueryResult<Vec<Vec<u8>>>;

    /// # Brief
    /// 范围查询
    ///
    /// # Arguments
    /// * `start` - 起始键(None 表示无下界)
    /// * `end` - 结束键(None 表示无上界)
    /// * `inclusive` - 是否包含边界
    ///
    /// # Returns
    /// 范围内的文档 ID 列表
    fn range(
        &self,
        start: Option<&IndexKey>,
        end: Option<&IndexKey>,
        inclusive: bool,
    ) -> QueryResult<Vec<Vec<u8>>>;

    /// # Brief
    /// 获取索引项数量
    fn count(&self) -> u64;
}

/// 索引键
///
/// 可以包含多个字段(复合索引)的索引键。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexKey(Vec<KeyPart>);

impl IndexKey {
    /// # Brief
    /// 创建复合索引键
    ///
    /// # Arguments
    /// * `parts` - 键部分列表
    pub fn new(parts: Vec<KeyPart>) -> Self {
        Self(parts)
    }

    /// # Brief
    /// 创建单字段索引键
    ///
    /// # Arguments
    /// * `part` - 键部分
    pub fn single(part: KeyPart) -> Self {
        Self(vec![part])
    }

    /// # Brief
    /// 从 BOML 值创建索引键
    ///
    /// # Arguments
    /// * `value` - BOML 值
    pub fn from_value(value: &BomlValue) -> Self {
        Self::single(KeyPart::from_value(value))
    }

    /// # Brief
    /// 获取键部分列表
    pub fn parts(&self) -> &[KeyPart] {
        &self.0
    }

    /// # Brief
    /// 序列化为字节数组
    ///
    /// 使用 0x00 分隔多个键部分。
    ///
    /// # Returns
    /// 序列化后的字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (i, part) in self.0.iter().enumerate() {
            // 使用 0x00 分隔多个键部分
            if i > 0 {
                bytes.push(0x00);
            }
            bytes.extend(part.to_bytes());
        }
        bytes
    }
}

/// 索引键的单个部分
///
/// 支持的类型:Null, Boolean, Integer, String, Binary。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyPart {
    /// Null 值
    Null,
    /// 布尔值
    Boolean(bool),
    /// 整数(统一为 i64)
    Integer(i64),
    /// 字符串
    String(String),
    /// 二进制数据
    Binary(Vec<u8>),
}

impl KeyPart {
    /// # Brief
    /// 从 BOML 值创建键部分
    ///
    /// 类型转换:
    /// - Int32/Int64 -> Integer
    /// - ObjectId -> Binary(12 字节)
    /// - 不支持的类型 -> Null
    ///
    /// # Arguments
    /// * `value` - BOML 值
    pub fn from_value(value: &BomlValue) -> Self {
        match value {
            BomlValue::Null => KeyPart::Null,
            BomlValue::Boolean(b) => KeyPart::Boolean(*b),
            // Int32/Int64 统一为 i64
            BomlValue::Int32(n) => KeyPart::Integer(*n as i64),
            BomlValue::Int64(n) => KeyPart::Integer(*n),
            BomlValue::String(s) => KeyPart::String(s.to_string()),
            BomlValue::Binary(b) => KeyPart::Binary(b.clone()),
            // ObjectId 转换为 12 字节 Binary
            BomlValue::ObjectId(id) => KeyPart::Binary(id.as_bytes().to_vec()),
            // 不支持的类型视为 Null
            _ => KeyPart::Null,
        }
    }

    /// # Brief
    /// 序列化为字节数组
    ///
    /// 格式:
    /// - Null: 0x00
    /// - Boolean(false): 0x01 0x00
    /// - Boolean(true): 0x01 0x01
    /// - Integer: 0x02 + 大端 8 字节
    /// - String: 0x03 + UTF-8 字节
    /// - Binary: 0x04 + 原始字节
    ///
    /// # Returns
    /// 序列化后的字节数组
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            KeyPart::Null => vec![0x00],
            KeyPart::Boolean(false) => vec![0x01, 0x00],
            KeyPart::Boolean(true) => vec![0x01, 0x01],
            KeyPart::Integer(n) => {
                let mut bytes = vec![0x02];
                // 使用大端字节序保证排序正确
                bytes.extend(&n.to_be_bytes());
                bytes
            }
            KeyPart::String(s) => {
                let mut bytes = vec![0x03];
                bytes.extend(s.as_bytes());
                bytes
            }
            KeyPart::Binary(b) => {
                let mut bytes = vec![0x04];
                bytes.extend(b);
                bytes
            }
        }
    }
}

/// BTree 索引实现
///
/// 使用 Rust 标准库的 BTreeMap 实现 B-树索引。
/// 支持范围查询和排序,适合大多数查询场景。
pub struct BTreeIndex {
    /// 索引定义
    definition: IndexDefinition,
    /// 索引数据:键 -> 文档 ID 列表
    tree: parking_lot::RwLock<BTreeMap<IndexKey, Vec<Vec<u8>>>>,
}

impl BTreeIndex {
    /// # Brief
    /// 创建 BTree 索引
    ///
    /// # Arguments
    /// * `definition` - 索引定义
    pub fn new(definition: IndexDefinition) -> Self {
        Self {
            definition,
            tree: parking_lot::RwLock::new(BTreeMap::new()),
        }
    }
}

impl Index for BTreeIndex {
    fn name(&self) -> &str {
        &self.definition.name
    }

    fn definition(&self) -> &IndexDefinition {
        &self.definition
    }

    /// # Brief
    /// 插入索引项
    ///
    /// 唯一索引检查:如果索引是唯一的,检查键是否已存在。
    fn insert(&self, key: IndexKey, doc_id: &[u8]) -> QueryResult<()> {
        let mut tree = self.tree.write();

        // 唯一索引检查
        if self.definition.unique {
            if let Some(existing) = tree.get(&key) {
                if !existing.is_empty() && existing[0] != doc_id {
                    return Err(QueryError::Execution(format!(
                        "Duplicate key error for index {}",
                        self.definition.name
                    )));
                }
            }
        }

        // 插入到索引
        tree.entry(key).or_default().push(doc_id.to_vec());
        Ok(())
    }

    /// # Brief
    /// 删除索引项
    ///
    /// 如果键的文档列表变空,删除整个键。
    fn delete(&self, key: &IndexKey, doc_id: &[u8]) -> QueryResult<bool> {
        let mut tree = self.tree.write();

        if let Some(ids) = tree.get_mut(key) {
            if let Some(pos) = ids.iter().position(|id| id == doc_id) {
                ids.remove(pos);
                // 如果列表为空,删除键
                if ids.is_empty() {
                    tree.remove(key);
                }
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn lookup(&self, key: &IndexKey) -> QueryResult<Vec<Vec<u8>>> {
        let tree = self.tree.read();
        Ok(tree.get(key).cloned().unwrap_or_default())
    }

    /// # Brief
    /// 范围查询
    ///
    /// 支持四种范围:
    /// - [start, end]: 有界范围
    /// - [start, ∞): 左有界
    /// - (-∞, end]: 右有界
    /// - (-∞, ∞): 全部
    fn range(
        &self,
        start: Option<&IndexKey>,
        end: Option<&IndexKey>,
        _inclusive: bool,
    ) -> QueryResult<Vec<Vec<u8>>> {
        let tree = self.tree.read();
        let mut results = Vec::new();

        // 根据边界构造范围迭代器
        let range = match (start, end) {
            (Some(s), Some(e)) => tree.range(s.clone()..=e.clone()),
            (Some(s), None) => tree.range(s.clone()..),
            (None, Some(e)) => tree.range(..=e.clone()),
            (None, None) => tree.range(..),
        };

        // 收集所有匹配的文档 ID
        for (_, ids) in range {
            results.extend(ids.clone());
        }

        Ok(results)
    }

    /// # Brief
    /// 获取索引项数量
    ///
    /// 统计所有键的文档数量总和。
    fn count(&self) -> u64 {
        let tree = self.tree.read();
        tree.values().map(|v| v.len() as u64).sum()
    }
}

/// 索引管理器
///
/// 管理集合的所有索引,提供创建、删除、查询索引的功能。
pub struct IndexManager {
    /// 索引列表:索引名 -> 索引实例
    indexes: parking_lot::RwLock<std::collections::HashMap<String, Arc<dyn Index>>>,
}

impl IndexManager {
    /// # Brief
    /// 创建索引管理器
    pub fn new() -> Self {
        Self {
            indexes: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// # Brief
    /// 创建索引
    ///
    /// 如果索引已存在则返回错误。
    /// 目前只支持 BTree 和 Hash 类型(Hash 使用 BTree 实现)。
    ///
    /// # Arguments
    /// * `definition` - 索引定义
    pub fn create_index(&self, definition: IndexDefinition) -> QueryResult<()> {
        let mut indexes = self.indexes.write();

        // 检查索引是否已存在
        if indexes.contains_key(&definition.name) {
            return Err(QueryError::Execution(format!(
                "Index {} already exists",
                definition.name
            )));
        }

        // 根据索引类型创建索引实例
        let index: Arc<dyn Index> = match definition.index_type {
            // BTree 和 Hash 都使用 BTreeIndex 实现
            IndexType::BTree | IndexType::Hash => Arc::new(BTreeIndex::new(definition.clone())),
            _ => {
                return Err(QueryError::Execution(format!(
                    "Index type {:?} not supported",
                    definition.index_type
                )));
            }
        };

        indexes.insert(definition.name.clone(), index);
        Ok(())
    }

    /// # Brief
    /// 删除索引
    ///
    /// # Arguments
    /// * `name` - 索引名称
    ///
    /// # Returns
    /// 是否删除成功
    pub fn drop_index(&self, name: &str) -> QueryResult<bool> {
        let mut indexes = self.indexes.write();
        Ok(indexes.remove(name).is_some())
    }

    /// # Brief
    /// 获取索引实例
    ///
    /// # Arguments
    /// * `name` - 索引名称
    ///
    /// # Returns
    /// 索引实例(如果存在)
    pub fn get_index(&self, name: &str) -> Option<Arc<dyn Index>> {
        let indexes = self.indexes.read();
        indexes.get(name).cloned()
    }

    /// # Brief
    /// 查找集合的所有索引
    ///
    /// # Arguments
    /// * `collection` - 集合名称
    ///
    /// # Returns
    /// 索引列表
    pub fn find_indexes_for_collection(&self, collection: &str) -> Vec<Arc<dyn Index>> {
        let indexes = self.indexes.read();
        indexes
            .values()
            .filter(|idx| idx.definition().collection == collection)
            .cloned()
            .collect()
    }

    /// # Brief
    /// 列出所有索引定义
    ///
    /// # Returns
    /// 索引定义列表
    pub fn list_indexes(&self) -> Vec<IndexDefinition> {
        let indexes = self.indexes.read();
        indexes.values().map(|idx| idx.definition().clone()).collect()
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

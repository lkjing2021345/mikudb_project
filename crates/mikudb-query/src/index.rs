use crate::{QueryError, QueryResult};
use mikudb_boml::BomlValue;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDefinition {
    pub name: String,
    pub collection: String,
    pub fields: Vec<IndexField>,
    pub unique: bool,
    pub sparse: bool,
    pub index_type: IndexType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexField {
    pub name: String,
    pub order: IndexOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexOrder {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Text,
    Geo2d,
    Geo2dsphere,
}

pub trait Index: Send + Sync {
    fn name(&self) -> &str;
    fn definition(&self) -> &IndexDefinition;
    fn insert(&self, key: IndexKey, doc_id: &[u8]) -> QueryResult<()>;
    fn delete(&self, key: &IndexKey, doc_id: &[u8]) -> QueryResult<bool>;
    fn lookup(&self, key: &IndexKey) -> QueryResult<Vec<Vec<u8>>>;
    fn range(
        &self,
        start: Option<&IndexKey>,
        end: Option<&IndexKey>,
        inclusive: bool,
    ) -> QueryResult<Vec<Vec<u8>>>;
    fn count(&self) -> u64;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexKey(Vec<KeyPart>);

impl IndexKey {
    pub fn new(parts: Vec<KeyPart>) -> Self {
        Self(parts)
    }

    pub fn single(part: KeyPart) -> Self {
        Self(vec![part])
    }

    pub fn from_value(value: &BomlValue) -> Self {
        Self::single(KeyPart::from_value(value))
    }

    pub fn parts(&self) -> &[KeyPart] {
        &self.0
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (i, part) in self.0.iter().enumerate() {
            if i > 0 {
                bytes.push(0x00);
            }
            bytes.extend(part.to_bytes());
        }
        bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyPart {
    Null,
    Boolean(bool),
    Integer(i64),
    String(String),
    Binary(Vec<u8>),
}

impl KeyPart {
    pub fn from_value(value: &BomlValue) -> Self {
        match value {
            BomlValue::Null => KeyPart::Null,
            BomlValue::Boolean(b) => KeyPart::Boolean(*b),
            BomlValue::Int32(n) => KeyPart::Integer(*n as i64),
            BomlValue::Int64(n) => KeyPart::Integer(*n),
            BomlValue::String(s) => KeyPart::String(s.to_string()),
            BomlValue::Binary(b) => KeyPart::Binary(b.clone()),
            BomlValue::ObjectId(id) => KeyPart::Binary(id.as_bytes().to_vec()),
            _ => KeyPart::Null,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            KeyPart::Null => vec![0x00],
            KeyPart::Boolean(false) => vec![0x01, 0x00],
            KeyPart::Boolean(true) => vec![0x01, 0x01],
            KeyPart::Integer(n) => {
                let mut bytes = vec![0x02];
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

pub struct BTreeIndex {
    definition: IndexDefinition,
    tree: parking_lot::RwLock<BTreeMap<IndexKey, Vec<Vec<u8>>>>,
}

impl BTreeIndex {
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

    fn insert(&self, key: IndexKey, doc_id: &[u8]) -> QueryResult<()> {
        let mut tree = self.tree.write();

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

        tree.entry(key).or_default().push(doc_id.to_vec());
        Ok(())
    }

    fn delete(&self, key: &IndexKey, doc_id: &[u8]) -> QueryResult<bool> {
        let mut tree = self.tree.write();

        if let Some(ids) = tree.get_mut(key) {
            if let Some(pos) = ids.iter().position(|id| id == doc_id) {
                ids.remove(pos);
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

    fn range(
        &self,
        start: Option<&IndexKey>,
        end: Option<&IndexKey>,
        _inclusive: bool,
    ) -> QueryResult<Vec<Vec<u8>>> {
        let tree = self.tree.read();
        let mut results = Vec::new();

        let range = match (start, end) {
            (Some(s), Some(e)) => tree.range(s.clone()..=e.clone()),
            (Some(s), None) => tree.range(s.clone()..),
            (None, Some(e)) => tree.range(..=e.clone()),
            (None, None) => tree.range(..),
        };

        for (_, ids) in range {
            results.extend(ids.clone());
        }

        Ok(results)
    }

    fn count(&self) -> u64 {
        let tree = self.tree.read();
        tree.values().map(|v| v.len() as u64).sum()
    }
}

pub struct IndexManager {
    indexes: parking_lot::RwLock<std::collections::HashMap<String, Arc<dyn Index>>>,
}

impl IndexManager {
    pub fn new() -> Self {
        Self {
            indexes: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn create_index(&self, definition: IndexDefinition) -> QueryResult<()> {
        let mut indexes = self.indexes.write();

        if indexes.contains_key(&definition.name) {
            return Err(QueryError::Execution(format!(
                "Index {} already exists",
                definition.name
            )));
        }

        let index: Arc<dyn Index> = match definition.index_type {
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

    pub fn drop_index(&self, name: &str) -> QueryResult<bool> {
        let mut indexes = self.indexes.write();
        Ok(indexes.remove(name).is_some())
    }

    pub fn get_index(&self, name: &str) -> Option<Arc<dyn Index>> {
        let indexes = self.indexes.read();
        indexes.get(name).cloned()
    }

    pub fn find_indexes_for_collection(&self, collection: &str) -> Vec<Arc<dyn Index>> {
        let indexes = self.indexes.read();
        indexes
            .values()
            .filter(|idx| idx.definition().collection == collection)
            .cloned()
            .collect()
    }

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

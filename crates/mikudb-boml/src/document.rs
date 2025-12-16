//! BOML 文档结构模块
//!
//! 提供高级 Document API，包装 BomlValue 并提供便捷的文档操作方法。

use crate::value::BomlValue;
use crate::BomlResult;
use compact_str::CompactString;
use indexmap::IndexMap;
use mikudb_common::ObjectId;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// BOML 文档结构
///
/// 表示一个完整的 BOML 文档，包含可选的 `_id` 字段和其他字段。
/// 使用 `IndexMap` 保持字段插入顺序。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Document {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    #[serde(flatten)]
    fields: IndexMap<CompactString, BomlValue>,
}

impl Document {
    /// 创建新文档
    ///
    /// # Brief
    /// 创建一个带有自动生成 ObjectId 的空文档
    ///
    /// # Returns
    /// 新的 Document 实例
    pub fn new() -> Self {
        Self {
            id: Some(ObjectId::new()),
            fields: IndexMap::new(),
        }
    }

    /// 使用指定 ID 创建文档
    ///
    /// # Brief
    /// 创建一个使用指定 ObjectId 的空文档
    ///
    /// # Arguments
    /// * `id` - 文档的 ObjectId
    ///
    /// # Returns
    /// 新的 Document 实例
    pub fn with_id(id: ObjectId) -> Self {
        Self {
            id: Some(id),
            fields: IndexMap::new(),
        }
    }

    /// 创建无 ID 文档
    ///
    /// # Brief
    /// 创建一个不带 `_id` 字段的空文档
    ///
    /// # Returns
    /// 新的 Document 实例
    pub fn without_id() -> Self {
        Self {
            id: None,
            fields: IndexMap::new(),
        }
    }

    /// 获取文档 ID
    ///
    /// # Brief
    /// 返回文档的 ObjectId 引用
    ///
    /// # Returns
    /// `Some(&ObjectId)` 如果存在，否则 `None`
    pub fn id(&self) -> Option<&ObjectId> {
        self.id.as_ref()
    }

    /// 设置文档 ID
    ///
    /// # Brief
    /// 设置或替换文档的 ObjectId
    ///
    /// # Arguments
    /// * `id` - 新的 ObjectId
    pub fn set_id(&mut self, id: ObjectId) {
        self.id = Some(id);
    }

    /// 确保文档有 ID
    ///
    /// # Brief
    /// 如果文档没有 ID，则自动生成一个；返回 ID 的引用
    ///
    /// # Returns
    /// 文档 ObjectId 的引用
    pub fn ensure_id(&mut self) -> &ObjectId {
        if self.id.is_none() {
            self.id = Some(ObjectId::new());
        }
        self.id.as_ref().unwrap()
    }

    /// 插入字段
    ///
    /// # Brief
    /// 向文档中插入或更新一个字段
    ///
    /// # Arguments
    /// * `key` - 字段名
    /// * `value` - 字段值
    pub fn insert(&mut self, key: impl Into<CompactString>, value: impl Into<BomlValue>) {
        self.fields.insert(key.into(), value.into());
    }

    /// 获取字段值
    ///
    /// # Brief
    /// 根据字段名获取值的引用
    ///
    /// # Arguments
    /// * `key` - 字段名
    ///
    /// # Returns
    /// `Some(&BomlValue)` 如果字段存在，否则 `None`
    pub fn get(&self, key: &str) -> Option<&BomlValue> {
        if key == "_id" {
            self.id.as_ref().map(|id| {
                static NULL: BomlValue = BomlValue::Null;
                &NULL
            })
        } else {
            self.fields.get(key)
        }
    }

    /// 获取字段的可变引用
    ///
    /// # Brief
    /// 根据字段名获取值的可变引用
    ///
    /// # Arguments
    /// * `key` - 字段名
    ///
    /// # Returns
    /// `Some(&mut BomlValue)` 如果字段存在，否则 `None`
    pub fn get_mut(&mut self, key: &str) -> Option<&mut BomlValue> {
        self.fields.get_mut(key)
    }

    /// 移除字段
    ///
    /// # Brief
    /// 从文档中移除指定字段并返回其值
    ///
    /// # Arguments
    /// * `key` - 字段名
    ///
    /// # Returns
    /// `Some(BomlValue)` 如果字段存在，否则 `None`
    pub fn remove(&mut self, key: &str) -> Option<BomlValue> {
        self.fields.shift_remove(key)
    }

    /// 检查字段是否存在
    ///
    /// # Brief
    /// 判断文档中是否包含指定字段
    ///
    /// # Arguments
    /// * `key` - 字段名
    ///
    /// # Returns
    /// 如果字段存在返回 `true`
    pub fn contains_key(&self, key: &str) -> bool {
        if key == "_id" {
            self.id.is_some()
        } else {
            self.fields.contains_key(key)
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.fields.keys().map(|k| k.as_str())
    }

    pub fn values(&self) -> impl Iterator<Item = &BomlValue> {
        self.fields.values()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &BomlValue)> {
        self.fields.iter().map(|(k, v)| (k.as_str(), v))
    }

    pub fn len(&self) -> usize {
        self.fields.len() + if self.id.is_some() { 1 } else { 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.id.is_none()
    }

    pub fn clear(&mut self) {
        self.fields.clear();
    }

    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.fields.get(key).and_then(|v| v.as_str())
    }

    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.fields.get(key).and_then(|v| v.as_i32())
    }

    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.fields.get(key).and_then(|v| v.as_i64())
    }

    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.fields.get(key).and_then(|v| v.as_f64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.fields.get(key).and_then(|v| v.as_bool())
    }

    pub fn get_array(&self, key: &str) -> Option<&Vec<BomlValue>> {
        self.fields.get(key).and_then(|v| v.as_array())
    }

    pub fn get_document(&self, key: &str) -> Option<&IndexMap<CompactString, BomlValue>> {
        self.fields.get(key).and_then(|v| v.as_document())
    }

    /// 按路径获取嵌套值
    ///
    /// # Brief
    /// 使用点分隔的路径访问嵌套文档中的值
    ///
    /// # Arguments
    /// * `path` - 点分隔的路径，如 "user.address.city"
    ///
    /// # Returns
    /// `Some(&BomlValue)` 如果路径存在，否则 `None`
    pub fn get_path(&self, path: &str) -> Option<&BomlValue> {
        let mut parts = path.split('.');
        let first = parts.next()?;

        let mut current = if first == "_id" {
            return None;
        } else {
            self.fields.get(first)?
        };

        for part in parts {
            current = current.get(part)?;
        }

        Some(current)
    }

    /// 转换为 BomlValue
    ///
    /// # Brief
    /// 将文档转换为 BomlValue::Document 类型
    ///
    /// # Returns
    /// 包含所有字段的 BomlValue::Document
    pub fn to_boml_value(&self) -> BomlValue {
        let mut doc = self.fields.clone();
        if let Some(id) = &self.id {
            doc.insert(CompactString::from("_id"), BomlValue::ObjectId(*id));
        }
        BomlValue::Document(doc)
    }

    /// 从 BomlValue 创建文档
    ///
    /// # Brief
    /// 将 BomlValue::Document 转换为 Document 结构
    ///
    /// # Arguments
    /// * `value` - 必须是 BomlValue::Document 类型
    ///
    /// # Returns
    /// 成功返回 Document，失败返回错误
    pub fn from_boml_value(value: BomlValue) -> BomlResult<Self> {
        match value {
            BomlValue::Document(mut fields) => {
                let id = fields.shift_remove("_id").and_then(|v| {
                    match v {
                        BomlValue::ObjectId(id) => Some(id),
                        _ => None,
                    }
                });
                Ok(Self { id, fields })
            }
            _ => Err(crate::BomlError::InvalidDocument(
                "Expected document type".to_string(),
            )),
        }
    }

    /// 合并另一个文档
    ///
    /// # Brief
    /// 将另一个文档的所有字段合并到当前文档中
    ///
    /// # Arguments
    /// * `other` - 要合并的文档
    pub fn merge(&mut self, other: Document) {
        for (k, v) in other.fields {
            self.fields.insert(k, v);
        }
    }

    /// 从 JSON 字符串创建文档
    ///
    /// # Brief
    /// 解析 JSON 字符串并创建文档
    ///
    /// # Arguments
    /// * `json` - JSON 格式的字符串
    ///
    /// # Returns
    /// 成功返回 Document，失败返回解析错误
    pub fn from_json(json: &str) -> BomlResult<Self> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| crate::BomlError::Deserialization(e.to_string()))?;
        let boml_value: BomlValue = value.into();
        Self::from_boml_value(boml_value)
    }

    /// 转换为 JSON 字符串
    ///
    /// # Brief
    /// 将文档序列化为紧凑的 JSON 字符串
    ///
    /// # Returns
    /// JSON 格式的字符串
    pub fn to_json(&self) -> String {
        let json_value: serde_json::Value = self.to_boml_value().into();
        serde_json::to_string(&json_value).unwrap_or_default()
    }

    /// 转换为格式化的 JSON 字符串
    ///
    /// # Brief
    /// 将文档序列化为带缩进的 JSON 字符串
    ///
    /// # Returns
    /// 格式化的 JSON 字符串
    pub fn to_json_pretty(&self) -> String {
        let json_value: serde_json::Value = self.to_boml_value().into();
        serde_json::to_string_pretty(&json_value).unwrap_or_default()
    }
}

impl From<IndexMap<CompactString, BomlValue>> for Document {
    fn from(mut fields: IndexMap<CompactString, BomlValue>) -> Self {
        let id = fields.shift_remove("_id").and_then(|v| match v {
            BomlValue::ObjectId(id) => Some(id),
            _ => None,
        });
        Self { id, fields }
    }
}

impl From<Document> for BomlValue {
    fn from(doc: Document) -> Self {
        doc.to_boml_value()
    }
}

/// 构造 Document 的便捷宏
///
/// # 示例
///
/// ```rust,ignore
/// use mikudb_boml::doc;
///
/// let empty = doc!();
/// let doc = doc! {
///     "name": "test",
///     "value": 123
/// };
/// ```
#[macro_export]
macro_rules! doc {
    () => {
        $crate::Document::new()
    };
    ($($key:tt : $value:tt),* $(,)?) => {
        {
            let mut doc = $crate::Document::new();
            $(
                doc.insert($key, $crate::boml!($value));
            )*
            doc
        }
    };
}

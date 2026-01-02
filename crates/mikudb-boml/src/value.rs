//! BOML 值类型定义模块
//!
//! 定义了 BOML 格式支持的所有数据类型，包括基础类型和复合类型。
//! 使用 `CompactString` 优化短字符串的内存占用。

use chrono::{DateTime, Utc};
use compact_str::CompactString;
use indexmap::IndexMap;
use mikudb_common::ObjectId;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// BOML 值的枚举类型
///
/// 表示 BOML 格式支持的所有数据类型，类似于 JSON 但支持更多类型。
///
/// # 支持的类型
///
/// - **基础类型**: Null, Boolean, Int32/64/128, Float32/64, Decimal, String, Binary
/// - **标识类型**: ObjectId, Uuid
/// - **时间类型**: DateTime, Timestamp
/// - **复合类型**: Array, Document
/// - **特殊类型**: Regex
///
/// # 示例
///
/// ```rust,ignore
/// use mikudb_boml::BomlValue;
///
/// let value = BomlValue::String("hello".into());
/// assert_eq!(value.type_name(), "string");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BomlValue {
    /// 空值
    Null,
    /// 布尔值
    Boolean(bool),
    /// 32位有符号整数
    Int32(i32),
    /// 64位有符号整数
    Int64(i64),
    /// 128位有符号整数（用于高精度计算）
    Int128(i128),
    /// 32位浮点数
    Float32(f32),
    /// 64位浮点数
    Float64(f64),
    /// 高精度十进制数（用于金融计算）
    Decimal(Decimal),
    /// UTF-8 字符串（使用 CompactString 优化短字符串）
    String(CompactString),
    /// 二进制数据
    Binary(Vec<u8>),
    /// 12字节的唯一对象标识符
    ObjectId(ObjectId),
    /// UUID v4/v7
    Uuid(Uuid),
    /// UTC 日期时间
    DateTime(DateTime<Utc>),
    /// Unix 时间戳（毫秒）
    Timestamp(i64),
    /// 值数组
    Array(Vec<BomlValue>),
    /// 文档（有序键值对）
    Document(IndexMap<CompactString, BomlValue>),
    /// 正则表达式
    Regex(RegexValue),
    /// JavaScript 代码
    JavaScript(JavaScriptValue),
}

/// 正则表达式值
///
/// 包含正则表达式的模式和选项（如 i, m, s 等）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegexValue {
    /// 正则表达式模式
    pub pattern: CompactString,
    /// 正则表达式选项
    pub options: CompactString,
}

/// JavaScript 代码值
///
/// 包含 JavaScript 代码字符串和可选的作用域（变量绑定）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JavaScriptValue {
    /// JavaScript 代码
    pub code: CompactString,
    /// 作用域（可选的变量绑定）
    pub scope: Option<IndexMap<CompactString, BomlValue>>,
}

impl BomlValue {
    /// 获取值的类型名称
    ///
    /// # Brief
    /// 返回 BOML 值的类型名称字符串
    ///
    /// # Returns
    /// 类型名称的静态字符串引用
    pub fn type_name(&self) -> &'static str {
        match self {
            BomlValue::Null => "null",
            BomlValue::Boolean(_) => "boolean",
            BomlValue::Int32(_) => "int32",
            BomlValue::Int64(_) => "int64",
            BomlValue::Int128(_) => "int128",
            BomlValue::Float32(_) => "float32",
            BomlValue::Float64(_) => "float64",
            BomlValue::Decimal(_) => "decimal",
            BomlValue::String(_) => "string",
            BomlValue::Binary(_) => "binary",
            BomlValue::ObjectId(_) => "objectId",
            BomlValue::Uuid(_) => "uuid",
            BomlValue::DateTime(_) => "dateTime",
            BomlValue::Timestamp(_) => "timestamp",
            BomlValue::Array(_) => "array",
            BomlValue::Document(_) => "document",
            BomlValue::Regex(_) => "regex",
            BomlValue::JavaScript(_) => "javascript",
        }
    }

    /// 检查值是否为 Null
    ///
    /// # Brief
    /// 判断当前值是否为空值
    ///
    /// # Returns
    /// 如果是 Null 返回 true，否则返回 false
    pub fn is_null(&self) -> bool {
        matches!(self, BomlValue::Null)
    }

    /// 尝试获取布尔值
    ///
    /// # Brief
    /// 如果值是布尔类型，返回其值；否则返回 None
    ///
    /// # Returns
    /// `Some(bool)` 如果是布尔值，否则 `None`
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            BomlValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// 尝试获取 i32 值
    ///
    /// # Brief
    /// 如果值是 Int32 类型，返回其值；否则返回 None
    ///
    /// # Returns
    /// `Some(i32)` 如果是 Int32，否则 `None`
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            BomlValue::Int32(n) => Some(*n),
            _ => None,
        }
    }

    /// 尝试获取 i64 值
    ///
    /// # Brief
    /// 如果值是整数类型（Int32 或 Int64），返回 i64 值
    ///
    /// # Returns
    /// `Some(i64)` 如果是整数类型，否则 `None`
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            BomlValue::Int32(n) => Some(*n as i64),
            BomlValue::Int64(n) => Some(*n),
            _ => None,
        }
    }

    /// 尝试获取 f64 值
    ///
    /// # Brief
    /// 如果值是数值类型，返回 f64 值（支持自动类型转换）
    ///
    /// # Returns
    /// `Some(f64)` 如果是数值类型，否则 `None`
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            BomlValue::Float32(n) => Some(*n as f64),
            BomlValue::Float64(n) => Some(*n),
            BomlValue::Int32(n) => Some(*n as f64),
            BomlValue::Int64(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// 尝试获取字符串引用
    ///
    /// # Brief
    /// 如果值是字符串类型，返回字符串切片
    ///
    /// # Returns
    /// `Some(&str)` 如果是字符串，否则 `None`
    pub fn as_str(&self) -> Option<&str> {
        match self {
            BomlValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// 尝试获取数组引用
    ///
    /// # Brief
    /// 如果值是数组类型，返回数组的引用
    ///
    /// # Returns
    /// `Some(&Vec<BomlValue>)` 如果是数组，否则 `None`
    pub fn as_array(&self) -> Option<&Vec<BomlValue>> {
        match self {
            BomlValue::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// 尝试获取文档引用
    ///
    /// # Brief
    /// 如果值是文档类型，返回文档（IndexMap）的引用
    ///
    /// # Returns
    /// `Some(&IndexMap)` 如果是文档，否则 `None`
    pub fn as_document(&self) -> Option<&IndexMap<CompactString, BomlValue>> {
        match self {
            BomlValue::Document(doc) => Some(doc),
            _ => None,
        }
    }

    /// 获取指定键的值
    ///
    /// # Brief
    /// 从文档中获取指定键的值，或从数组中获取指定索引的值
    ///
    /// # Arguments
    /// * `key` - 键名（文档）或索引字符串（数组）
    ///
    /// # Returns
    /// `Some(&BomlValue)` 如果找到，否则 `None`
    pub fn get(&self, key: &str) -> Option<&BomlValue> {
        match self {
            BomlValue::Document(doc) => doc.get(key),
            BomlValue::Array(arr) => key.parse::<usize>().ok().and_then(|i| arr.get(i)),
            _ => None,
        }
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
    ///
    /// # Example
    /// ```rust,ignore
    /// let value = doc.get_path("user.profile.name");
    /// ```
    pub fn get_path(&self, path: &str) -> Option<&BomlValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = self;
        for part in parts {
            current = current.get(part)?;
        }
        Some(current)
    }
}

impl Default for BomlValue {
    fn default() -> Self {
        BomlValue::Null
    }
}

impl fmt::Display for BomlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BomlValue::Null => write!(f, "null"),
            BomlValue::Boolean(b) => write!(f, "{}", b),
            BomlValue::Int32(n) => write!(f, "{}", n),
            BomlValue::Int64(n) => write!(f, "{}", n),
            BomlValue::Int128(n) => write!(f, "{}", n),
            BomlValue::Float32(n) => write!(f, "{}", n),
            BomlValue::Float64(n) => write!(f, "{}", n),
            BomlValue::Decimal(n) => write!(f, "{}", n),
            BomlValue::String(s) => write!(f, "\"{}\"", s),
            BomlValue::Binary(b) => write!(f, "<binary:{} bytes>", b.len()),
            BomlValue::ObjectId(id) => write!(f, "ObjectId(\"{}\")", id),
            BomlValue::Uuid(u) => write!(f, "UUID(\"{}\")", u),
            BomlValue::DateTime(dt) => write!(f, "DateTime(\"{}\")", dt),
            BomlValue::Timestamp(ts) => write!(f, "Timestamp({})", ts),
            BomlValue::Array(arr) => {
                write!(f, "[")?;
                for (i, v) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            BomlValue::Document(doc) => {
                write!(f, "{{")?;
                for (i, (k, v)) in doc.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            BomlValue::Regex(r) => write!(f, "/{}/{}", r.pattern, r.options),
            BomlValue::JavaScript(js) => {
                if let Some(scope) = &js.scope {
                    write!(f, "JavaScript({}, scope: {:?})", js.code, scope)
                } else {
                    write!(f, "JavaScript({})", js.code)
                }
            }
        }
    }
}

// ============================================================================
// From 特征实现 - 支持从各种 Rust 类型转换为 BomlValue
// ============================================================================

impl From<bool> for BomlValue {
    fn from(v: bool) -> Self {
        BomlValue::Boolean(v)
    }
}

impl From<i32> for BomlValue {
    fn from(v: i32) -> Self {
        BomlValue::Int32(v)
    }
}

impl From<i64> for BomlValue {
    fn from(v: i64) -> Self {
        BomlValue::Int64(v)
    }
}

impl From<f64> for BomlValue {
    fn from(v: f64) -> Self {
        BomlValue::Float64(v)
    }
}

impl From<&str> for BomlValue {
    fn from(v: &str) -> Self {
        BomlValue::String(CompactString::from(v))
    }
}

impl From<String> for BomlValue {
    fn from(v: String) -> Self {
        BomlValue::String(CompactString::from(v))
    }
}

impl From<Vec<u8>> for BomlValue {
    fn from(v: Vec<u8>) -> Self {
        BomlValue::Binary(v)
    }
}

impl From<ObjectId> for BomlValue {
    fn from(v: ObjectId) -> Self {
        BomlValue::ObjectId(v)
    }
}

impl From<Uuid> for BomlValue {
    fn from(v: Uuid) -> Self {
        BomlValue::Uuid(v)
    }
}

impl<T: Into<BomlValue>> From<Vec<T>> for BomlValue {
    fn from(v: Vec<T>) -> Self {
        BomlValue::Array(v.into_iter().map(Into::into).collect())
    }
}

// ============================================================================
// JSON 互转支持
// ============================================================================

impl From<serde_json::Value> for BomlValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => BomlValue::Null,
            serde_json::Value::Bool(b) => BomlValue::Boolean(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        BomlValue::Int32(i as i32)
                    } else {
                        BomlValue::Int64(i)
                    }
                } else if let Some(f) = n.as_f64() {
                    BomlValue::Float64(f)
                } else {
                    BomlValue::Null
                }
            }
            serde_json::Value::String(s) => BomlValue::String(CompactString::from(s)),
            serde_json::Value::Array(arr) => {
                BomlValue::Array(arr.into_iter().map(Into::into).collect())
            }
            serde_json::Value::Object(obj) => {
                let mut doc = IndexMap::new();
                for (k, v) in obj {
                    doc.insert(CompactString::from(k), v.into());
                }
                BomlValue::Document(doc)
            }
        }
    }
}

impl From<BomlValue> for serde_json::Value {
    fn from(v: BomlValue) -> Self {
        match v {
            BomlValue::Null => serde_json::Value::Null,
            BomlValue::Boolean(b) => serde_json::Value::Bool(b),
            BomlValue::Int32(n) => serde_json::Value::Number(n.into()),
            BomlValue::Int64(n) => serde_json::Value::Number(n.into()),
            BomlValue::Int128(n) => {
                serde_json::Value::String(n.to_string())
            }
            BomlValue::Float32(n) => {
                serde_json::Number::from_f64(n as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            BomlValue::Float64(n) => {
                serde_json::Number::from_f64(n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            BomlValue::Decimal(n) => serde_json::Value::String(n.to_string()),
            BomlValue::String(s) => serde_json::Value::String(s.to_string()),
            BomlValue::Binary(b) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$binary".to_string(), serde_json::Value::String(base64_encode(&b)));
                    map
                })
            }
            BomlValue::ObjectId(id) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$oid".to_string(), serde_json::Value::String(id.to_hex()));
                    map
                })
            }
            BomlValue::Uuid(u) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$uuid".to_string(), serde_json::Value::String(u.to_string()));
                    map
                })
            }
            BomlValue::DateTime(dt) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$date".to_string(), serde_json::Value::String(dt.to_rfc3339()));
                    map
                })
            }
            BomlValue::Timestamp(ts) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$timestamp".to_string(), serde_json::Value::Number(ts.into()));
                    map
                })
            }
            BomlValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Into::into).collect())
            }
            BomlValue::Document(doc) => {
                let mut map = serde_json::Map::new();
                for (k, v) in doc {
                    map.insert(k.to_string(), v.into());
                }
                serde_json::Value::Object(map)
            }
            BomlValue::Regex(r) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$regex".to_string(), serde_json::Value::String(r.pattern.to_string()));
                    map.insert("$options".to_string(), serde_json::Value::String(r.options.to_string()));
                    map
                })
            }
            BomlValue::JavaScript(js) => {
                serde_json::Value::Object({
                    let mut map = serde_json::Map::new();
                    map.insert("$code".to_string(), serde_json::Value::String(js.code.to_string()));
                    if let Some(scope) = js.scope {
                        map.insert("$scope".to_string(), BomlValue::Document(scope).into());
                    }
                    map
                })
            }
        }
    }
}

/// Base64 编码辅助函数
///
/// # Brief
/// 将字节数组编码为 Base64 字符串
///
/// # Arguments
/// * `data` - 要编码的字节切片
///
/// # Returns
/// Base64 编码的字符串
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);
        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }
    result
}

/// 构造 BomlValue 的便捷宏
///
/// # 示例
///
/// ```rust,ignore
/// use mikudb_boml::boml;
///
/// let null = boml!(null);
/// let boolean = boml!(true);
/// let number = boml!(42);
/// let string = boml!("hello");
/// let array = boml!([1, 2, 3]);
/// let doc = boml!({ "name": "test", "value": 123 });
/// ```
#[macro_export]
macro_rules! boml {
    (null) => {
        $crate::BomlValue::Null
    };
    (true) => {
        $crate::BomlValue::Boolean(true)
    };
    (false) => {
        $crate::BomlValue::Boolean(false)
    };
    ($e:expr) => {
        $crate::BomlValue::from($e)
    };
    ([ $($elem:tt),* $(,)? ]) => {
        $crate::BomlValue::Array(vec![ $(boml!($elem)),* ])
    };
    ({ $($key:tt : $value:tt),* $(,)? }) => {
        {
            let mut doc = indexmap::IndexMap::new();
            $(
                doc.insert(compact_str::CompactString::from($key), boml!($value));
            )*
            $crate::BomlValue::Document(doc)
        }
    };
}

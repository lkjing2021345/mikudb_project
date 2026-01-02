//! BOML 与 BSON 互转模块
//!
//! 提供 BOML 格式与 BSON (Binary JSON) 格式之间的相互转换功能。
//! BSON 是 MongoDB 使用的二进制文档格式。

use crate::value::{BomlValue, JavaScriptValue, RegexValue};
use crate::{BomlError, BomlResult};
use bson::{Bson, Document as BsonDocument};
use chrono::TimeZone;
use compact_str::CompactString;
use indexmap::IndexMap;
use mikudb_common::ObjectId;

/// 将 BomlValue 转换为 BSON
///
/// # Brief
/// 将 BOML 值转换为 BSON 值，保持类型兼容性
///
/// # 类型映射
/// - Null → Null
/// - Boolean → Boolean
/// - Int32/Int64 → Int32/Int64
/// - Float32/Float64 → Double
/// - String → String
/// - Binary → Binary
/// - ObjectId → ObjectId
/// - DateTime → DateTime
/// - Array → Array
/// - Document → Document
/// - Regex → RegularExpression
/// - JavaScript → JavaScriptCode / JavaScriptCodeWithScope
///
/// # Arguments
/// * `value` - 要转换的 BOML 值
///
/// # Returns
/// 成功返回 BSON 值，失败返回错误
pub fn to_bson(value: &BomlValue) -> BomlResult<Bson> {
    match value {
        BomlValue::Null => Ok(Bson::Null),
        BomlValue::Boolean(b) => Ok(Bson::Boolean(*b)),
        BomlValue::Int32(n) => Ok(Bson::Int32(*n)),
        BomlValue::Int64(n) => Ok(Bson::Int64(*n)),
        BomlValue::Int128(n) => {
            // BSON 不支持 i128，转为字符串存储在文档中
            if *n >= i64::MIN as i128 && *n <= i64::MAX as i128 {
                Ok(Bson::Int64(*n as i64))
            } else {
                // 超出 i64 范围，使用字符串表示
                Ok(Bson::String(n.to_string()))
            }
        }
        BomlValue::Float32(f) => Ok(Bson::Double(*f as f64)),
        BomlValue::Float64(f) => Ok(Bson::Double(*f)),
        BomlValue::Decimal(d) => {
            // 转换为 BSON Decimal128，使用字符串构造
            let s = d.to_string();
            match s.parse::<bson::Decimal128>() {
                Ok(decimal) => Ok(Bson::Decimal128(decimal)),
                Err(_) => Ok(Bson::String(s)), // 如果解析失败，降级为字符串
            }
        }
        BomlValue::String(s) => Ok(Bson::String(s.to_string())),
        BomlValue::Binary(b) => Ok(Bson::Binary(bson::Binary {
            subtype: bson::spec::BinarySubtype::Generic,
            bytes: b.clone(),
        })),
        BomlValue::ObjectId(oid) => {
            let bytes = oid.as_bytes();
            let bson_oid = bson::oid::ObjectId::from_bytes(*bytes);
            Ok(Bson::ObjectId(bson_oid))
        }
        BomlValue::Uuid(uuid) => {
            // UUID 在 BSON 中存储为二进制
            Ok(Bson::Binary(bson::Binary {
                subtype: bson::spec::BinarySubtype::Uuid,
                bytes: uuid.as_bytes().to_vec(),
            }))
        }
        BomlValue::DateTime(dt) => {
            let timestamp_millis = dt.timestamp_millis();
            let bson_dt = bson::DateTime::from_millis(timestamp_millis);
            Ok(Bson::DateTime(bson_dt))
        }
        BomlValue::Timestamp(ts) => {
            // Timestamp 转为 DateTime
            let bson_dt = bson::DateTime::from_millis(*ts);
            Ok(Bson::DateTime(bson_dt))
        }
        BomlValue::Array(arr) => {
            let bson_arr: Result<Vec<_>, _> = arr.iter().map(to_bson).collect();
            Ok(Bson::Array(bson_arr?))
        }
        BomlValue::Document(doc) => {
            let mut bson_doc = BsonDocument::new();
            for (k, v) in doc {
                bson_doc.insert(k.to_string(), to_bson(v)?);
            }
            Ok(Bson::Document(bson_doc))
        }
        BomlValue::Regex(r) => Ok(Bson::RegularExpression(bson::Regex {
            pattern: r.pattern.to_string(),
            options: r.options.to_string(),
        })),
        BomlValue::JavaScript(js) => {
            if let Some(scope) = &js.scope {
                let mut bson_scope = BsonDocument::new();
                for (k, v) in scope {
                    bson_scope.insert(k.to_string(), to_bson(v)?);
                }
                Ok(Bson::JavaScriptCodeWithScope(bson::JavaScriptCodeWithScope {
                    code: js.code.to_string(),
                    scope: bson_scope,
                }))
            } else {
                Ok(Bson::JavaScriptCode(js.code.to_string()))
            }
        }
    }
}

/// 从 BSON 转换为 BomlValue
///
/// # Brief
/// 将 BSON 值转换为 BOML 值
///
/// # Arguments
/// * `bson` - BSON 值
///
/// # Returns
/// 成功返回 BOML 值，失败返回错误
pub fn from_bson(bson: &Bson) -> BomlResult<BomlValue> {
    match bson {
        Bson::Null => Ok(BomlValue::Null),
        Bson::Boolean(b) => Ok(BomlValue::Boolean(*b)),
        Bson::Int32(n) => Ok(BomlValue::Int32(*n)),
        Bson::Int64(n) => Ok(BomlValue::Int64(*n)),
        Bson::Double(f) => Ok(BomlValue::Float64(*f)),
        Bson::Decimal128(d) => {
            // 尝试转换为 Decimal
            let s = d.to_string();
            let decimal = s.parse().map_err(|_| {
                BomlError::Deserialization("Invalid Decimal128".to_string())
            })?;
            Ok(BomlValue::Decimal(decimal))
        }
        Bson::String(s) => Ok(BomlValue::String(CompactString::new(s))),
        Bson::Binary(b) => {
            match b.subtype {
                bson::spec::BinarySubtype::Uuid => {
                    // UUID 二进制数据
                    if b.bytes.len() == 16 {
                        let uuid = uuid::Uuid::from_bytes(
                            b.bytes[..16]
                                .try_into()
                                .map_err(|_| BomlError::Deserialization("Invalid UUID".to_string()))?,
                        );
                        Ok(BomlValue::Uuid(uuid))
                    } else {
                        Ok(BomlValue::Binary(b.bytes.clone()))
                    }
                }
                _ => Ok(BomlValue::Binary(b.bytes.clone())),
            }
        }
        Bson::ObjectId(oid) => {
            let bytes = oid.bytes();
            let boml_oid = ObjectId::from_bytes(bytes);
            Ok(BomlValue::ObjectId(boml_oid))
        }
        Bson::DateTime(dt) => {
            let timestamp_millis = dt.timestamp_millis();
            let chrono_dt = chrono::Utc
                .timestamp_millis_opt(timestamp_millis)
                .single()
                .ok_or_else(|| BomlError::Deserialization("Invalid datetime".to_string()))?;
            Ok(BomlValue::DateTime(chrono_dt))
        }
        Bson::Array(arr) => {
            let boml_arr: Result<Vec<_>, _> = arr.iter().map(from_bson).collect();
            Ok(BomlValue::Array(boml_arr?))
        }
        Bson::Document(doc) => {
            let mut boml_doc = IndexMap::new();
            for (k, v) in doc {
                boml_doc.insert(CompactString::new(k), from_bson(v)?);
            }
            Ok(BomlValue::Document(boml_doc))
        }
        Bson::RegularExpression(regex) => Ok(BomlValue::Regex(RegexValue {
            pattern: CompactString::new(&regex.pattern),
            options: CompactString::new(&regex.options),
        })),
        Bson::JavaScriptCode(code) => Ok(BomlValue::JavaScript(JavaScriptValue {
            code: CompactString::new(code),
            scope: None,
        })),
        Bson::JavaScriptCodeWithScope(js) => {
            let mut scope = IndexMap::new();
            for (k, v) in &js.scope {
                scope.insert(CompactString::new(k), from_bson(v)?);
            }
            Ok(BomlValue::JavaScript(JavaScriptValue {
                code: CompactString::new(&js.code),
                scope: Some(scope),
            }))
        }
        Bson::Timestamp(ts) => {
            // BSON Timestamp 是特殊的内部类型，转为 Int64
            Ok(BomlValue::Int64(ts.time as i64))
        }
        Bson::Symbol(s) => {
            // Symbol 转为 String
            Ok(BomlValue::String(CompactString::new(s)))
        }
        Bson::Undefined => Ok(BomlValue::Null),
        Bson::MaxKey | Bson::MinKey => {
            // MinKey/MaxKey 没有直接对应类型，转为 Null
            Ok(BomlValue::Null)
        }
        Bson::DbPointer(_) => {
            // DbPointer 是遗留类型，转为 Null
            Ok(BomlValue::Null)
        }
    }
}

/// 将 BOML 值序列化为 BSON 二进制格式
///
/// # Brief
/// 将 BOML 值转换为 BSON 文档并序列化为字节数组
///
/// # Arguments
/// * `value` - 要序列化的 BOML 值（必须是 Document）
///
/// # Returns
/// 成功返回 BSON 二进制数据，失败返回错误
pub fn to_bson_bytes(value: &BomlValue) -> BomlResult<Vec<u8>> {
    let bson_value = to_bson(value)?;
    if let Bson::Document(doc) = bson_value {
        let mut bytes = Vec::new();
        doc.to_writer(&mut bytes).map_err(|e| {
            BomlError::Serialization(format!("BSON serialization failed: {}", e))
        })?;
        Ok(bytes)
    } else {
        Err(BomlError::Serialization(
            "Only documents can be serialized to BSON bytes".to_string(),
        ))
    }
}

/// 从 BSON 二进制格式反序列化为 BOML 值
///
/// # Brief
/// 解析 BSON 二进制数据并转换为 BOML 值
///
/// # Arguments
/// * `bytes` - BSON 二进制数据
///
/// # Returns
/// 成功返回 BOML 值，失败返回错误
pub fn from_bson_bytes(bytes: &[u8]) -> BomlResult<BomlValue> {
    let doc = BsonDocument::from_reader(&mut &bytes[..]).map_err(|e| {
        BomlError::Deserialization(format!("BSON deserialization failed: {}", e))
    })?;
    from_bson(&Bson::Document(doc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn test_basic_types() {
        assert_eq!(
            from_bson(&to_bson(&BomlValue::Null).unwrap()).unwrap(),
            BomlValue::Null
        );
        assert_eq!(
            from_bson(&to_bson(&BomlValue::Boolean(true)).unwrap()).unwrap(),
            BomlValue::Boolean(true)
        );
        assert_eq!(
            from_bson(&to_bson(&BomlValue::Int32(42)).unwrap()).unwrap(),
            BomlValue::Int32(42)
        );
    }

    #[test]
    fn test_document() {
        let mut doc = Document::new();
        doc.insert("name", "Alice");
        doc.insert("age", 30);

        let bson_val = to_bson(&doc.to_boml_value()).unwrap();
        let restored = from_bson(&bson_val).unwrap();

        assert_eq!(doc.to_boml_value(), restored);
    }

    #[test]
    fn test_round_trip() {
        let original = BomlValue::Document(
            vec![
                (
                    CompactString::new("name"),
                    BomlValue::String(CompactString::new("Bob")),
                ),
                (CompactString::new("age"), BomlValue::Int32(25)),
                (
                    CompactString::new("active"),
                    BomlValue::Boolean(true),
                ),
            ]
            .into_iter()
            .collect(),
        );

        let bytes = to_bson_bytes(&original).unwrap();
        let restored = from_bson_bytes(&bytes).unwrap();

        assert_eq!(original, restored);
    }
}

//! BOML 与 JSON 互转模块
//!
//! 提供 BOML 格式与 JSON 格式之间的相互转换功能。
//! 由于 JSON 类型系统较简单，某些 BOML 类型会转换为扩展 JSON 格式。

use crate::value::{BomlValue, JavaScriptValue, RegexValue};
use crate::{BomlError, BomlResult};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use chrono::TimeZone;
use compact_str::CompactString;
use indexmap::IndexMap;
use mikudb_common::ObjectId;
use serde_json::{json, Map, Number, Value as JsonValue};

/// 将 BomlValue 转换为 JSON
///
/// # Brief
/// 将 BOML 值转换为 JSON 值，复杂类型使用扩展 JSON 格式
///
/// # 扩展 JSON 格式
/// - ObjectId: `{"$oid": "507f1f77bcf86cd799439011"}`
/// - DateTime: `{"$date": 1234567890000}`
/// - Regex: `{"$regex": "pattern", "$options": "i"}`
/// - Binary: `{"$binary": "base64_string"}`
/// - JavaScript: `{"$code": "function() {}"}`
/// - JavaScript with Scope: `{"$code": "...", "$scope": {...}}`
///
/// # Arguments
/// * `value` - 要转换的 BOML 值
///
/// # Returns
/// 成功返回 JSON 值，失败返回错误
pub fn to_json(value: &BomlValue) -> BomlResult<JsonValue> {
    match value {
        BomlValue::Null => Ok(JsonValue::Null),
        BomlValue::Boolean(b) => Ok(JsonValue::Bool(*b)),
        BomlValue::Int32(n) => Ok(json!(*n)),
        BomlValue::Int64(n) => Ok(json!(*n)),
        BomlValue::Int128(n) => {
            // JSON 不支持 i128，转为字符串
            Ok(json!({"$numberLong": n.to_string()}))
        }
        BomlValue::Float32(f) => {
            if let Some(n) = Number::from_f64(*f as f64) {
                Ok(JsonValue::Number(n))
            } else {
                Ok(json!(f.to_string()))
            }
        }
        BomlValue::Float64(f) => {
            if let Some(n) = Number::from_f64(*f) {
                Ok(JsonValue::Number(n))
            } else {
                Ok(json!(f.to_string()))
            }
        }
        BomlValue::Decimal(d) => {
            // Decimal 转为字符串保持精度
            Ok(json!({"$numberDecimal": d.to_string()}))
        }
        BomlValue::String(s) => Ok(JsonValue::String(s.to_string())),
        BomlValue::Binary(b) => {
            // 二进制数据编码为 Base64
            let base64 = STANDARD.encode(b);
            Ok(json!({"$binary": base64}))
        }
        BomlValue::ObjectId(oid) => {
            Ok(json!({"$oid": oid.to_string()}))
        }
        BomlValue::Uuid(uuid) => {
            Ok(json!({"$uuid": uuid.to_string()}))
        }
        BomlValue::DateTime(dt) => {
            Ok(json!({"$date": dt.timestamp_millis()}))
        }
        BomlValue::Timestamp(ts) => {
            Ok(json!({"$timestamp": ts}))
        }
        BomlValue::Array(arr) => {
            let json_arr: Result<Vec<_>, _> = arr.iter().map(to_json).collect();
            Ok(JsonValue::Array(json_arr?))
        }
        BomlValue::Document(doc) => {
            let mut json_obj = Map::new();
            for (k, v) in doc {
                json_obj.insert(k.to_string(), to_json(v)?);
            }
            Ok(JsonValue::Object(json_obj))
        }
        BomlValue::Regex(r) => {
            Ok(json!({
                "$regex": r.pattern.as_str(),
                "$options": r.options.as_str()
            }))
        }
        BomlValue::JavaScript(js) => {
            if let Some(scope) = &js.scope {
                Ok(json!({
                    "$code": js.code.as_str(),
                    "$scope": to_json(&BomlValue::Document(scope.clone()))?
                }))
            } else {
                Ok(json!({"$code": js.code.as_str()}))
            }
        }
    }
}

/// 从 JSON 转换为 BomlValue
///
/// # Brief
/// 将 JSON 值转换为 BOML 值，识别扩展 JSON 格式
///
/// # Arguments
/// * `value` - JSON 值
///
/// # Returns
/// 成功返回 BOML 值，失败返回错误
pub fn from_json(value: &JsonValue) -> BomlResult<BomlValue> {
    match value {
        JsonValue::Null => Ok(BomlValue::Null),
        JsonValue::Bool(b) => Ok(BomlValue::Boolean(*b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                    Ok(BomlValue::Int32(i as i32))
                } else {
                    Ok(BomlValue::Int64(i))
                }
            } else if let Some(f) = n.as_f64() {
                Ok(BomlValue::Float64(f))
            } else {
                Err(BomlError::Deserialization("Invalid number".to_string()))
            }
        }
        JsonValue::String(s) => Ok(BomlValue::String(CompactString::new(s))),
        JsonValue::Array(arr) => {
            let boml_arr: Result<Vec<_>, _> = arr.iter().map(from_json).collect();
            Ok(BomlValue::Array(boml_arr?))
        }
        JsonValue::Object(obj) => {
            // 检查是否为扩展 JSON 格式
            if let Some(oid) = obj.get("$oid") {
                if let JsonValue::String(s) = oid {
                    let oid = ObjectId::from_hex(s).map_err(|_| BomlError::InvalidObjectId)?;
                    return Ok(BomlValue::ObjectId(oid));
                }
            }

            if let Some(uuid) = obj.get("$uuid") {
                if let JsonValue::String(s) = uuid {
                    let uuid = s
                        .parse()
                        .map_err(|_| BomlError::Deserialization("Invalid UUID".to_string()))?;
                    return Ok(BomlValue::Uuid(uuid));
                }
            }

            if let Some(date) = obj.get("$date") {
                if let JsonValue::Number(n) = date {
                    if let Some(millis) = n.as_i64() {
                        let dt = chrono::Utc
                            .timestamp_millis_opt(millis)
                            .single()
                            .ok_or_else(|| {
                                BomlError::Deserialization("Invalid datetime".to_string())
                            })?;
                        return Ok(BomlValue::DateTime(dt));
                    }
                }
            }

            if let Some(ts) = obj.get("$timestamp") {
                if let JsonValue::Number(n) = ts {
                    if let Some(millis) = n.as_i64() {
                        return Ok(BomlValue::Timestamp(millis));
                    }
                }
            }

            if let Some(binary) = obj.get("$binary") {
                if let JsonValue::String(s) = binary {
                    let bytes = STANDARD.decode(s).map_err(|_| {
                        BomlError::Deserialization("Invalid base64".to_string())
                    })?;
                    return Ok(BomlValue::Binary(bytes));
                }
            }

            if let Some(regex) = obj.get("$regex") {
                if let JsonValue::String(pattern) = regex {
                    let options = obj
                        .get("$options")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    return Ok(BomlValue::Regex(RegexValue {
                        pattern: CompactString::new(pattern),
                        options: CompactString::new(options),
                    }));
                }
            }

            if let Some(code) = obj.get("$code") {
                if let JsonValue::String(code_str) = code {
                    let scope = if let Some(scope_val) = obj.get("$scope") {
                        let scope_boml = from_json(scope_val)?;
                        if let BomlValue::Document(doc) = scope_boml {
                            Some(doc)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    return Ok(BomlValue::JavaScript(JavaScriptValue {
                        code: CompactString::new(code_str),
                        scope,
                    }));
                }
            }

            if let Some(num_long) = obj.get("$numberLong") {
                if let JsonValue::String(s) = num_long {
                    let n: i128 = s.parse().map_err(|_| {
                        BomlError::Deserialization("Invalid $numberLong".to_string())
                    })?;
                    return Ok(BomlValue::Int128(n));
                }
            }

            if let Some(decimal) = obj.get("$numberDecimal") {
                if let JsonValue::String(s) = decimal {
                    let d = s.parse().map_err(|_| {
                        BomlError::Deserialization("Invalid $numberDecimal".to_string())
                    })?;
                    return Ok(BomlValue::Decimal(d));
                }
            }

            // 普通文档
            let mut boml_doc = IndexMap::new();
            for (k, v) in obj {
                boml_doc.insert(CompactString::new(k), from_json(v)?);
            }
            Ok(BomlValue::Document(boml_doc))
        }
    }
}

/// 将 BOML 值序列化为 JSON 字符串
///
/// # Brief
/// 将 BOML 值转换为美化的 JSON 字符串
///
/// # Arguments
/// * `value` - 要序列化的 BOML 值
///
/// # Returns
/// 成功返回 JSON 字符串，失败返回错误
pub fn to_json_string(value: &BomlValue) -> BomlResult<String> {
    let json_value = to_json(value)?;
    serde_json::to_string_pretty(&json_value).map_err(|e| {
        BomlError::Serialization(format!("JSON serialization failed: {}", e))
    })
}

/// 从 JSON 字符串反序列化为 BOML 值
///
/// # Brief
/// 解析 JSON 字符串并转换为 BOML 值
///
/// # Arguments
/// * `json_str` - JSON 字符串
///
/// # Returns
/// 成功返回 BOML 值，失败返回错误
pub fn from_json_string(json_str: &str) -> BomlResult<BomlValue> {
    let json_value: JsonValue = serde_json::from_str(json_str).map_err(|e| {
        BomlError::Deserialization(format!("JSON parsing failed: {}", e))
    })?;
    from_json(&json_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn test_basic_types() {
        assert_eq!(to_json(&BomlValue::Null).unwrap(), JsonValue::Null);
        assert_eq!(
            to_json(&BomlValue::Boolean(true)).unwrap(),
            JsonValue::Bool(true)
        );
        assert_eq!(to_json(&BomlValue::Int32(42)).unwrap(), json!(42));
        assert_eq!(
            to_json(&BomlValue::String(CompactString::new("hello"))).unwrap(),
            json!("hello")
        );
    }

    #[test]
    fn test_array() {
        let arr = BomlValue::Array(vec![
            BomlValue::Int32(1),
            BomlValue::Int32(2),
            BomlValue::Int32(3),
        ]);
        assert_eq!(to_json(&arr).unwrap(), json!([1, 2, 3]));
    }

    #[test]
    fn test_document() {
        let mut doc = Document::new();
        doc.insert("name", "Alice");
        doc.insert("age", 30);

        let json_val = to_json(&doc.to_boml_value()).unwrap();
        assert_eq!(json_val, json!({"name": "Alice", "age": 30}));
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
            ]
            .into_iter()
            .collect(),
        );

        let json_str = to_json_string(&original).unwrap();
        let restored = from_json_string(&json_str).unwrap();

        assert_eq!(original, restored);
    }
}

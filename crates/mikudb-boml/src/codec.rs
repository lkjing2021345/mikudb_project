//! BOML 编解码模块
//!
//! 提供 BOML 格式的二进制序列化和反序列化功能。
//! 使用 xxHash3 进行校验和计算，在 ARM64 (鲲鹏) 上有优秀性能。

use crate::spec::*;
use crate::value::{BomlValue, RegexValue};
use crate::{BomlError, BomlResult};
use bytes::{Buf, BufMut, BytesMut};
use chrono::{TimeZone, Utc};
use compact_str::CompactString;
use indexmap::IndexMap;
use mikudb_common::ObjectId;
use rust_decimal::Decimal;
use std::io::{Read, Write};
use uuid::Uuid;

/// 编码 BomlValue 到缓冲区
///
/// # Brief
/// 将 BomlValue 序列化为二进制格式写入缓冲区
///
/// # Arguments
/// * `value` - 要编码的值
/// * `buf` - 目标缓冲区
///
/// # Returns
/// 成功返回 Ok(()), 失败返回错误
pub fn encode(value: &BomlValue, buf: &mut BytesMut) -> BomlResult<()> {
    Encoder::new(buf).encode_value(value)
}

/// 编码 BomlValue 到 Vec<u8>
///
/// # Brief
/// 将 BomlValue 序列化为二进制字节向量
///
/// # Arguments
/// * `value` - 要编码的值
///
/// # Returns
/// 成功返回字节向量, 失败返回错误
pub fn encode_to_vec(value: &BomlValue) -> BomlResult<Vec<u8>> {
    let mut buf = BytesMut::with_capacity(256);
    encode(value, &mut buf)?;
    Ok(buf.to_vec())
}

/// 解码二进制数据为 BomlValue
///
/// # Brief
/// 将二进制数据反序列化为 BomlValue
///
/// # Arguments
/// * `data` - 要解码的字节切片
///
/// # Returns
/// 成功返回 BomlValue, 失败返回错误
pub fn decode(data: &[u8]) -> BomlResult<BomlValue> {
    Decoder::new(data).decode_value()
}

/// 编码文档（带魔数和校验和）
///
/// # Brief
/// 将 BomlValue 编码为完整的 BOML 文档格式，包含魔数、版本号和 xxHash3 校验和
///
/// # OpenEuler 适配亮点
/// 使用 xxHash3 进行校验和计算，在鲲鹏 ARM64 处理器上有优秀的性能表现
///
/// # Arguments
/// * `value` - 要编码的文档值
///
/// # Returns
/// 成功返回带校验和的字节向量, 失败返回错误
pub fn encode_document(value: &BomlValue) -> BomlResult<Vec<u8>> {
    let mut buf = BytesMut::with_capacity(256);
    buf.put_slice(&BOML_MAGIC);
    buf.put_u8(BOML_VERSION);
    encode(value, &mut buf)?;
    let checksum = xxhash_rust::xxh3::xxh3_64(&buf[5..]);
    buf.put_u64_le(checksum);
    Ok(buf.to_vec())
}

/// 解码文档（带魔数和校验和验证）
///
/// # Brief
/// 解码完整的 BOML 文档，验证魔数、版本号和 xxHash3 校验和
///
/// # Arguments
/// * `data` - 要解码的字节切片
///
/// # Returns
/// 成功返回 BomlValue, 校验失败或格式错误返回错误
pub fn decode_document(data: &[u8]) -> BomlResult<BomlValue> {
    if data.len() < 13 {
        return Err(BomlError::UnexpectedEof);
    }
    if &data[0..4] != BOML_MAGIC {
        return Err(BomlError::InvalidDocument("Invalid magic number".to_string()));
    }
    let version = data[4];
    if version != BOML_VERSION {
        return Err(BomlError::InvalidDocument(format!(
            "Unsupported version: {}",
            version
        )));
    }
    let checksum_offset = data.len() - 8;
    let stored_checksum = u64::from_le_bytes(data[checksum_offset..].try_into().unwrap());
    let computed_checksum = xxhash_rust::xxh3::xxh3_64(&data[5..checksum_offset]);
    if stored_checksum != computed_checksum {
        return Err(BomlError::InvalidDocument("Checksum mismatch".to_string()));
    }
    decode(&data[5..checksum_offset])
}

/// BOML 编码器
///
/// 内部结构，用于将 BomlValue 序列化为二进制格式
struct Encoder<'a> {
    buf: &'a mut BytesMut,
    depth: usize,
}

impl<'a> Encoder<'a> {
    fn new(buf: &'a mut BytesMut) -> Self {
        Self { buf, depth: 0 }
    }

    fn encode_value(&mut self, value: &BomlValue) -> BomlResult<()> {
        if self.depth > MAX_NESTING_DEPTH {
            return Err(BomlError::NestingTooDeep(MAX_NESTING_DEPTH));
        }

        match value {
            BomlValue::Null => {
                self.buf.put_u8(TypeMarker::Null as u8);
            }
            BomlValue::Boolean(true) => {
                self.buf.put_u8(TypeMarker::BooleanTrue as u8);
            }
            BomlValue::Boolean(false) => {
                self.buf.put_u8(TypeMarker::BooleanFalse as u8);
            }
            BomlValue::Int32(n) => {
                self.encode_int32(*n);
            }
            BomlValue::Int64(n) => {
                self.encode_int64(*n);
            }
            BomlValue::Int128(n) => {
                self.buf.put_u8(TypeMarker::Int128 as u8);
                self.buf.put_i128_le(*n);
            }
            BomlValue::Float32(n) => {
                self.buf.put_u8(TypeMarker::Float32 as u8);
                self.buf.put_f32_le(*n);
            }
            BomlValue::Float64(n) => {
                if *n == 0.0 {
                    self.buf.put_u8(TypeMarker::Float64Zero as u8);
                } else {
                    self.buf.put_u8(TypeMarker::Float64 as u8);
                    self.buf.put_f64_le(*n);
                }
            }
            BomlValue::Decimal(n) => {
                self.buf.put_u8(TypeMarker::Decimal as u8);
                let bytes = n.serialize();
                self.buf.put_slice(&bytes);
            }
            BomlValue::String(s) => {
                self.encode_string(s);
            }
            BomlValue::Binary(b) => {
                self.buf.put_u8(TypeMarker::Binary as u8);
                self.encode_varint(b.len() as u64);
                self.buf.put_slice(b);
            }
            BomlValue::ObjectId(id) => {
                self.buf.put_u8(TypeMarker::ObjectId as u8);
                self.buf.put_slice(id.as_bytes());
            }
            BomlValue::Uuid(u) => {
                self.buf.put_u8(TypeMarker::Uuid as u8);
                self.buf.put_slice(u.as_bytes());
            }
            BomlValue::DateTime(dt) => {
                self.buf.put_u8(TypeMarker::DateTime as u8);
                self.buf.put_i64_le(dt.timestamp_millis());
            }
            BomlValue::Timestamp(ts) => {
                self.buf.put_u8(TypeMarker::Timestamp as u8);
                self.buf.put_i64_le(*ts);
            }
            BomlValue::Array(arr) => {
                self.encode_array(arr)?;
            }
            BomlValue::Document(doc) => {
                self.encode_document(doc)?;
            }
            BomlValue::Regex(r) => {
                self.buf.put_u8(TypeMarker::Regex as u8);
                self.encode_string(&r.pattern);
                self.encode_string(&r.options);
            }
        }
        Ok(())
    }

    fn encode_int32(&mut self, n: i32) {
        match n {
            0 => self.buf.put_u8(TypeMarker::Int32Zero as u8),
            1 => self.buf.put_u8(TypeMarker::Int32One as u8),
            -1 => self.buf.put_u8(TypeMarker::Int32NegOne as u8),
            n if n >= 0 && n < 16 => {
                self.buf.put_u8(0x30 + n as u8);
            }
            n => {
                self.buf.put_u8(TypeMarker::Int32 as u8);
                self.buf.put_i32_le(n);
            }
        }
    }

    fn encode_int64(&mut self, n: i64) {
        if n == 0 {
            self.buf.put_u8(TypeMarker::Int64Zero as u8);
        } else if n >= i32::MIN as i64 && n <= i32::MAX as i64 {
            self.encode_int32(n as i32);
        } else {
            self.buf.put_u8(TypeMarker::Int64 as u8);
            self.buf.put_i64_le(n);
        }
    }

    fn encode_string(&mut self, s: &str) {
        let len = s.len();
        if len == 0 {
            self.buf.put_u8(TypeMarker::EmptyString as u8);
        } else if len < 16 {
            self.buf.put_u8(0x20 + len as u8);
            self.buf.put_slice(s.as_bytes());
        } else {
            self.buf.put_u8(TypeMarker::String as u8);
            self.encode_varint(len as u64);
            self.buf.put_slice(s.as_bytes());
        }
    }

    fn encode_array(&mut self, arr: &[BomlValue]) -> BomlResult<()> {
        let len = arr.len();
        if len == 0 {
            self.buf.put_u8(TypeMarker::EmptyArray as u8);
        } else if len < 16 {
            self.buf.put_u8(0x40 + len as u8);
        } else {
            self.buf.put_u8(TypeMarker::Array as u8);
            self.encode_varint(len as u64);
        }

        self.depth += 1;
        for item in arr {
            self.encode_value(item)?;
        }
        self.depth -= 1;
        Ok(())
    }

    fn encode_document(&mut self, doc: &IndexMap<CompactString, BomlValue>) -> BomlResult<()> {
        let len = doc.len();
        if len == 0 {
            self.buf.put_u8(TypeMarker::EmptyDocument as u8);
            return Ok(());
        }

        self.buf.put_u8(TypeMarker::Document as u8);
        self.encode_varint(len as u64);

        self.depth += 1;
        for (key, value) in doc {
            self.encode_string(key);
            self.encode_value(value)?;
        }
        self.depth -= 1;
        Ok(())
    }

    fn encode_varint(&mut self, mut n: u64) {
        while n >= 0x80 {
            self.buf.put_u8((n as u8) | 0x80);
            n >>= 7;
        }
        self.buf.put_u8(n as u8);
    }
}

/// BOML 解码器
///
/// 内部结构，用于从二进制数据反序列化 BomlValue
struct Decoder<'a> {
    data: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Decoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            pos: 0,
            depth: 0,
        }
    }

    fn decode_value(&mut self) -> BomlResult<BomlValue> {
        if self.depth > MAX_NESTING_DEPTH {
            return Err(BomlError::NestingTooDeep(MAX_NESTING_DEPTH));
        }

        let marker = self.read_u8()?;

        if TypeMarker::is_small_string(marker) {
            let len = TypeMarker::small_string_len(marker);
            return self.read_string(len);
        }

        if TypeMarker::is_small_int(marker) {
            let val = TypeMarker::small_int_value(marker);
            return Ok(BomlValue::Int32(val as i32));
        }

        if TypeMarker::is_small_array(marker) {
            let len = TypeMarker::small_array_len(marker);
            return self.decode_array_items(len);
        }

        match TypeMarker::from_u8(marker) {
            Some(TypeMarker::Null) => Ok(BomlValue::Null),
            Some(TypeMarker::BooleanTrue) => Ok(BomlValue::Boolean(true)),
            Some(TypeMarker::BooleanFalse) => Ok(BomlValue::Boolean(false)),
            Some(TypeMarker::Int32Zero) => Ok(BomlValue::Int32(0)),
            Some(TypeMarker::Int32One) => Ok(BomlValue::Int32(1)),
            Some(TypeMarker::Int32NegOne) => Ok(BomlValue::Int32(-1)),
            Some(TypeMarker::Int64Zero) => Ok(BomlValue::Int64(0)),
            Some(TypeMarker::Float64Zero) => Ok(BomlValue::Float64(0.0)),
            Some(TypeMarker::EmptyString) => Ok(BomlValue::String(CompactString::new(""))),
            Some(TypeMarker::EmptyArray) => Ok(BomlValue::Array(vec![])),
            Some(TypeMarker::EmptyDocument) => Ok(BomlValue::Document(IndexMap::new())),
            Some(TypeMarker::Int32) => {
                let n = self.read_i32()?;
                Ok(BomlValue::Int32(n))
            }
            Some(TypeMarker::Int64) => {
                let n = self.read_i64()?;
                Ok(BomlValue::Int64(n))
            }
            Some(TypeMarker::Int128) => {
                let n = self.read_i128()?;
                Ok(BomlValue::Int128(n))
            }
            Some(TypeMarker::Float32) => {
                let n = self.read_f32()?;
                Ok(BomlValue::Float32(n))
            }
            Some(TypeMarker::Float64) => {
                let n = self.read_f64()?;
                Ok(BomlValue::Float64(n))
            }
            Some(TypeMarker::Decimal) => {
                let mut bytes = [0u8; 16];
                self.read_exact(&mut bytes)?;
                let d = Decimal::deserialize(bytes);
                Ok(BomlValue::Decimal(d))
            }
            Some(TypeMarker::String) => {
                let len = self.read_varint()? as usize;
                self.read_string(len)
            }
            Some(TypeMarker::Binary) => {
                let len = self.read_varint()? as usize;
                let bytes = self.read_bytes(len)?;
                Ok(BomlValue::Binary(bytes))
            }
            Some(TypeMarker::ObjectId) => {
                let mut bytes = [0u8; 12];
                self.read_exact(&mut bytes)?;
                Ok(BomlValue::ObjectId(ObjectId::from_bytes(bytes)))
            }
            Some(TypeMarker::Uuid) => {
                let mut bytes = [0u8; 16];
                self.read_exact(&mut bytes)?;
                Ok(BomlValue::Uuid(Uuid::from_bytes(bytes)))
            }
            Some(TypeMarker::DateTime) => {
                let millis = self.read_i64()?;
                let dt = Utc.timestamp_millis_opt(millis).single().ok_or_else(|| {
                    BomlError::InvalidDocument("Invalid datetime".to_string())
                })?;
                Ok(BomlValue::DateTime(dt))
            }
            Some(TypeMarker::Timestamp) => {
                let ts = self.read_i64()?;
                Ok(BomlValue::Timestamp(ts))
            }
            Some(TypeMarker::Array) => {
                let len = self.read_varint()? as usize;
                self.decode_array_items(len)
            }
            Some(TypeMarker::Document) => {
                let len = self.read_varint()? as usize;
                self.decode_document_items(len)
            }
            Some(TypeMarker::Regex) => {
                let pattern_len = self.read_varint()? as usize;
                let pattern = self.read_compact_string(pattern_len)?;
                let options_len = self.read_varint()? as usize;
                let options = self.read_compact_string(options_len)?;
                Ok(BomlValue::Regex(RegexValue { pattern, options }))
            }
            _ => Err(BomlError::InvalidTypeMarker(marker)),
        }
    }

    fn decode_array_items(&mut self, len: usize) -> BomlResult<BomlValue> {
        if len > MAX_ARRAY_LENGTH {
            return Err(BomlError::InvalidDocument(format!(
                "Array too large: {} > {}",
                len, MAX_ARRAY_LENGTH
            )));
        }

        self.depth += 1;
        let mut arr = Vec::with_capacity(len);
        for _ in 0..len {
            arr.push(self.decode_value()?);
        }
        self.depth -= 1;
        Ok(BomlValue::Array(arr))
    }

    fn decode_document_items(&mut self, len: usize) -> BomlResult<BomlValue> {
        self.depth += 1;
        let mut doc = IndexMap::with_capacity(len);
        for _ in 0..len {
            let key_marker = self.read_u8()?;
            let key = if TypeMarker::is_small_string(key_marker) {
                let key_len = TypeMarker::small_string_len(key_marker);
                self.read_compact_string(key_len)?
            } else if key_marker == TypeMarker::EmptyString as u8 {
                CompactString::new("")
            } else if key_marker == TypeMarker::String as u8 {
                let key_len = self.read_varint()? as usize;
                self.read_compact_string(key_len)?
            } else {
                return Err(BomlError::InvalidDocument(
                    "Expected string key in document".to_string(),
                ));
            };
            let value = self.decode_value()?;
            doc.insert(key, value);
        }
        self.depth -= 1;
        Ok(BomlValue::Document(doc))
    }

    fn read_u8(&mut self) -> BomlResult<u8> {
        if self.pos >= self.data.len() {
            return Err(BomlError::UnexpectedEof);
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> BomlResult<()> {
        if self.pos + buf.len() > self.data.len() {
            return Err(BomlError::UnexpectedEof);
        }
        buf.copy_from_slice(&self.data[self.pos..self.pos + buf.len()]);
        self.pos += buf.len();
        Ok(())
    }

    fn read_bytes(&mut self, len: usize) -> BomlResult<Vec<u8>> {
        if self.pos + len > self.data.len() {
            return Err(BomlError::UnexpectedEof);
        }
        let bytes = self.data[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(bytes)
    }

    fn read_i32(&mut self) -> BomlResult<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_i64(&mut self) -> BomlResult<i64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    fn read_i128(&mut self) -> BomlResult<i128> {
        let mut buf = [0u8; 16];
        self.read_exact(&mut buf)?;
        Ok(i128::from_le_bytes(buf))
    }

    fn read_f32(&mut self) -> BomlResult<f32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_f64(&mut self) -> BomlResult<f64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }

    fn read_varint(&mut self) -> BomlResult<u64> {
        let mut result: u64 = 0;
        let mut shift = 0;
        loop {
            let byte = self.read_u8()?;
            result |= ((byte & 0x7f) as u64) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
            if shift > 63 {
                return Err(BomlError::InvalidDocument("Varint too large".to_string()));
            }
        }
        Ok(result)
    }

    fn read_string(&mut self, len: usize) -> BomlResult<BomlValue> {
        if len > MAX_STRING_LENGTH {
            return Err(BomlError::InvalidDocument(format!(
                "String too large: {} > {}",
                len, MAX_STRING_LENGTH
            )));
        }
        let bytes = self.read_bytes(len)?;
        let s = String::from_utf8(bytes)?;
        Ok(BomlValue::String(CompactString::from(s)))
    }

    fn read_compact_string(&mut self, len: usize) -> BomlResult<CompactString> {
        let bytes = self.read_bytes(len)?;
        let s = String::from_utf8(bytes)?;
        Ok(CompactString::from(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_null() {
        let value = BomlValue::Null;
        let encoded = encode_to_vec(&value).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_bool() {
        for v in [true, false] {
            let value = BomlValue::Boolean(v);
            let encoded = encode_to_vec(&value).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(value, decoded);
        }
    }

    #[test]
    fn test_encode_decode_int32() {
        for n in [-1, 0, 1, 15, 16, 100, i32::MIN, i32::MAX] {
            let value = BomlValue::Int32(n);
            let encoded = encode_to_vec(&value).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(value, decoded);
        }
    }

    #[test]
    fn test_encode_decode_string() {
        for s in ["", "hello", "a".repeat(100).as_str()] {
            let value = BomlValue::String(CompactString::from(s));
            let encoded = encode_to_vec(&value).unwrap();
            let decoded = decode(&encoded).unwrap();
            assert_eq!(value, decoded);
        }
    }

    #[test]
    fn test_encode_decode_array() {
        let value = BomlValue::Array(vec![
            BomlValue::Int32(1),
            BomlValue::String(CompactString::from("test")),
            BomlValue::Boolean(true),
        ]);
        let encoded = encode_to_vec(&value).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_encode_decode_document() {
        let mut doc = IndexMap::new();
        doc.insert(CompactString::from("name"), BomlValue::String(CompactString::from("test")));
        doc.insert(CompactString::from("value"), BomlValue::Int32(42));
        let value = BomlValue::Document(doc);
        let encoded = encode_to_vec(&value).unwrap();
        let decoded = decode(&encoded).unwrap();
        assert_eq!(value, decoded);
    }

    #[test]
    fn test_document_with_checksum() {
        let mut doc = IndexMap::new();
        doc.insert(CompactString::from("test"), BomlValue::Int32(123));
        let value = BomlValue::Document(doc);
        let encoded = encode_document(&value).unwrap();
        let decoded = decode_document(&encoded).unwrap();
        assert_eq!(value, decoded);
    }
}

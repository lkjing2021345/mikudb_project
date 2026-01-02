//! Serde 反序列化模块
//!
//! 实现 Serde Deserializer trait,将 BomlValue 反序列化为 Rust 数据结构。
//!
//! 支持所有标准 Rust 类型的反序列化:
//! - 基本类型: bool, 整数, 浮点数, 字符串
//! - 复合类型: 结构体, 枚举, 数组, 元组, HashMap
//! - 自动类型转换: Int32 -> i64, Int64 -> i32 (如果在范围内)

use crate::value::BomlValue;
use crate::BomlError;
use compact_str::CompactString;
use serde::de::{self, DeserializeSeed, IntoDeserializer, MapAccess, SeqAccess, Visitor};
use serde::Deserialize;
use std::fmt;

pub struct Deserializer<'de> {
    input: &'de BomlValue,
}

impl<'de> Deserializer<'de> {
    pub fn from_boml(input: &'de BomlValue) -> Self {
        Deserializer { input }
    }
}

pub fn from_boml<'a, T: Deserialize<'a>>(value: &'a BomlValue) -> Result<T, BomlError> {
    let deserializer = Deserializer::from_boml(value);
    T::deserialize(deserializer)
}

impl de::Error for BomlError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        BomlError::Deserialization(msg.to_string())
    }
}

impl<'de, 'a> de::Deserializer<'de> for Deserializer<'de> {
    type Error = BomlError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Null => visitor.visit_unit(),
            BomlValue::Boolean(b) => visitor.visit_bool(*b),
            BomlValue::Int32(n) => visitor.visit_i32(*n),
            BomlValue::Int64(n) => visitor.visit_i64(*n),
            BomlValue::Int128(n) => visitor.visit_i128(*n),
            BomlValue::Float32(n) => visitor.visit_f32(*n),
            BomlValue::Float64(n) => visitor.visit_f64(*n),
            BomlValue::String(s) => visitor.visit_str(s.as_str()),
            BomlValue::Binary(b) => visitor.visit_bytes(b),
            BomlValue::Array(arr) => {
                let seq = SeqDeserializer::new(arr.iter());
                visitor.visit_seq(seq)
            }
            BomlValue::Document(doc) => {
                let map = MapDeserializer::new(doc.iter());
                visitor.visit_map(map)
            }
            _ => Err(BomlError::Deserialization(format!(
                "Cannot deserialize {} as any",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Boolean(b) => visitor.visit_bool(*b),
            _ => Err(BomlError::Deserialization(format!(
                "Expected boolean, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i32(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i32(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Int32(n) => visitor.visit_i32(*n),
            BomlValue::Int64(n) => visitor.visit_i64(*n),
            _ => Err(BomlError::Deserialization(format!(
                "Expected integer, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Int32(n) => visitor.visit_i64(*n as i64),
            BomlValue::Int64(n) => visitor.visit_i64(*n),
            _ => Err(BomlError::Deserialization(format!(
                "Expected integer, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Int32(n) => visitor.visit_i128(*n as i128),
            BomlValue::Int64(n) => visitor.visit_i128(*n as i128),
            BomlValue::Int128(n) => visitor.visit_i128(*n),
            _ => Err(BomlError::Deserialization(format!(
                "Expected integer, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u32(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u32(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Int32(n) if *n >= 0 => visitor.visit_u32(*n as u32),
            BomlValue::Int64(n) if *n >= 0 && *n <= u32::MAX as i64 => visitor.visit_u32(*n as u32),
            _ => Err(BomlError::Deserialization(format!(
                "Expected unsigned integer, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Int32(n) if *n >= 0 => visitor.visit_u64(*n as u64),
            BomlValue::Int64(n) if *n >= 0 => visitor.visit_u64(*n as u64),
            _ => Err(BomlError::Deserialization(format!(
                "Expected unsigned integer, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Int32(n) if *n >= 0 => visitor.visit_u128(*n as u128),
            BomlValue::Int64(n) if *n >= 0 => visitor.visit_u128(*n as u128),
            BomlValue::Int128(n) if *n >= 0 => visitor.visit_u128(*n as u128),
            _ => Err(BomlError::Deserialization(format!(
                "Expected unsigned integer, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Float32(n) => visitor.visit_f32(*n),
            BomlValue::Float64(n) => visitor.visit_f32(*n as f32),
            BomlValue::Int32(n) => visitor.visit_f32(*n as f32),
            _ => Err(BomlError::Deserialization(format!(
                "Expected float, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Float32(n) => visitor.visit_f64(*n as f64),
            BomlValue::Float64(n) => visitor.visit_f64(*n),
            BomlValue::Int32(n) => visitor.visit_f64(*n as f64),
            BomlValue::Int64(n) => visitor.visit_f64(*n as f64),
            _ => Err(BomlError::Deserialization(format!(
                "Expected float, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::String(s) if s.len() == 1 => {
                visitor.visit_char(s.chars().next().unwrap())
            }
            _ => Err(BomlError::Deserialization(format!(
                "Expected char, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::String(s) => visitor.visit_str(s.as_str()),
            _ => Err(BomlError::Deserialization(format!(
                "Expected string, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Binary(b) => visitor.visit_bytes(b),
            _ => Err(BomlError::Deserialization(format!(
                "Expected binary, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Null => visitor.visit_unit(),
            _ => Err(BomlError::Deserialization(format!(
                "Expected null, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Array(arr) => {
                let seq = SeqDeserializer::new(arr.iter());
                visitor.visit_seq(seq)
            }
            _ => Err(BomlError::Deserialization(format!(
                "Expected array, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::Document(doc) => {
                let map = MapDeserializer::new(doc.iter());
                visitor.visit_map(map)
            }
            _ => Err(BomlError::Deserialization(format!(
                "Expected document, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.input {
            BomlValue::String(s) => visitor.visit_enum(s.as_str().into_deserializer()),
            BomlValue::Document(doc) if doc.len() == 1 => {
                let (key, value) = doc.iter().next().unwrap();
                visitor.visit_enum(EnumDeserializer {
                    variant: key.as_str(),
                    value,
                })
            }
            _ => Err(BomlError::Deserialization(format!(
                "Expected string or document for enum, got {}",
                self.input.type_name()
            ))),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}

struct SeqDeserializer<'de, I> {
    iter: I,
    _marker: std::marker::PhantomData<&'de ()>,
}

impl<'de, I: Iterator<Item = &'de BomlValue>> SeqDeserializer<'de, I> {
    fn new(iter: I) -> Self {
        Self {
            iter,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'de, I: Iterator<Item = &'de BomlValue>> SeqAccess<'de> for SeqDeserializer<'de, I> {
    type Error = BomlError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.iter.next() {
            Some(value) => seed.deserialize(Deserializer::from_boml(value)).map(Some),
            None => Ok(None),
        }
    }
}

struct MapDeserializer<'de, I> {
    iter: I,
    value: Option<&'de BomlValue>,
    _marker: std::marker::PhantomData<&'de ()>,
}

impl<'de, I: Iterator<Item = (&'de CompactString, &'de BomlValue)>> MapDeserializer<'de, I> {
    fn new(iter: I) -> Self {
        Self {
            iter,
            value: None,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'de, I: Iterator<Item = (&'de CompactString, &'de BomlValue)>> MapAccess<'de>
    for MapDeserializer<'de, I>
{
    type Error = BomlError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key.as_str().into_deserializer()).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let value = self
            .value
            .take()
            .ok_or_else(|| BomlError::Deserialization("No value".to_string()))?;
        seed.deserialize(Deserializer::from_boml(value))
    }
}

struct EnumDeserializer<'de> {
    variant: &'de str,
    value: &'de BomlValue,
}

impl<'de> de::EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = BomlError;
    type Variant = VariantDeserializer<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        use serde::de::value::StrDeserializer;
        let deserializer: StrDeserializer<'de, BomlError> = self.variant.into_deserializer();
        let variant: V::Value = seed.deserialize(deserializer)?;
        Ok((variant, VariantDeserializer { value: self.value }))
    }
}

struct VariantDeserializer<'de> {
    value: &'de BomlValue,
}

impl<'de> de::VariantAccess<'de> for VariantDeserializer<'de> {
    type Error = BomlError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        seed: T,
    ) -> Result<T::Value, Self::Error> {
        seed.deserialize(Deserializer::from_boml(self.value))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        de::Deserializer::deserialize_seq(Deserializer::from_boml(self.value), visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        de::Deserializer::deserialize_map(Deserializer::from_boml(self.value), visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        name: String,
        value: i32,
        active: bool,
    }

    #[test]
    fn test_roundtrip_struct() {
        let original = TestStruct {
            name: "test".to_string(),
            value: 42,
            active: true,
        };

        let boml = crate::ser::to_boml(&original).unwrap();
        let restored: TestStruct = from_boml(&boml).unwrap();

        assert_eq!(original, restored);
    }
}

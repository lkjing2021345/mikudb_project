//! 公共类型定义模块
//!
//! 定义 MikuDB 的核心类型:
//! - ObjectId: 12 字节唯一标识符(类似 MongoDB ObjectId)
//! - DocumentId: 文档 ID 封装
//! - CollectionName: 集合名称(带验证)
//! - DatabaseName: 数据库名称(带验证)
//! - Timestamp: 毫秒级时间戳

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// ObjectId - 12 字节唯一标识符
///
/// 格式:
/// - 前 4 字节: 时间戳(秒,大端)
/// - 后 8 字节: 随机数(/dev/urandom 或系统熵)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId([u8; 12]);

impl ObjectId {
    pub fn new() -> Self {
        let mut bytes = [0u8; 12];
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as u32;
        bytes[0..4].copy_from_slice(&timestamp.to_be_bytes());
        let random: [u8; 8] = rand_bytes();
        bytes[4..12].copy_from_slice(&random);
        Self(bytes)
    }

    pub fn from_bytes(bytes: [u8; 12]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 12] {
        &self.0
    }

    pub fn timestamp(&self) -> u32 {
        u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(s: &str) -> Result<Self, crate::error::MikuError> {
        let bytes = hex::decode(s).map_err(|e| {
            crate::error::MikuError::InvalidObjectId(format!("Invalid hex: {}", e))
        })?;
        if bytes.len() != 12 {
            return Err(crate::error::MikuError::InvalidObjectId(
                "ObjectId must be 12 bytes".to_string(),
            ));
        }
        let mut arr = [0u8; 12];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

fn rand_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    #[cfg(target_os = "linux")]
    {
        use std::fs::File;
        use std::io::Read;
        if let Ok(mut f) = File::open("/dev/urandom") {
            let _ = f.read_exact(&mut bytes);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let state = RandomState::new();
        for chunk in bytes.chunks_mut(8) {
            let hash = state.build_hasher().finish().to_le_bytes();
            let len = chunk.len().min(8);
            chunk.copy_from_slice(&hash[..len]);
        }
    }
    bytes
}

/// 文档 ID
///
/// 封装 ObjectId,用于标识文档。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentId(pub ObjectId);

impl DocumentId {
    pub fn new() -> Self {
        Self(ObjectId::new())
    }

    pub fn from_object_id(id: ObjectId) -> Self {
        Self(id)
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DocumentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 集合名称
///
/// 带验证的集合名称,禁止:
/// - 空名称
/// - system. 前缀
/// - 包含 null 字符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CollectionName(String);

impl CollectionName {
    pub fn new(name: impl Into<String>) -> Result<Self, crate::error::MikuError> {
        let name = name.into();
        if name.is_empty() {
            return Err(crate::error::MikuError::Validation(
                "Collection name cannot be empty".to_string(),
            ));
        }
        if name.starts_with("system.") {
            return Err(crate::error::MikuError::Validation(
                "Collection name cannot start with 'system.'".to_string(),
            ));
        }
        if name.contains('\0') {
            return Err(crate::error::MikuError::Validation(
                "Collection name cannot contain null character".to_string(),
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CollectionName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 数据库名称
///
/// 带验证的数据库名称,限制:
/// - 不能为空
/// - 最大 64 字符
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseName(String);

impl DatabaseName {
    pub fn new(name: impl Into<String>) -> Result<Self, crate::error::MikuError> {
        let name = name.into();
        if name.is_empty() {
            return Err(crate::error::MikuError::Validation(
                "Database name cannot be empty".to_string(),
            ));
        }
        if name.len() > 64 {
            return Err(crate::error::MikuError::Validation(
                "Database name cannot exceed 64 characters".to_string(),
            ));
        }
        Ok(Self(name))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for DatabaseName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 时间戳(毫秒)
///
/// Unix 时间戳,精度为毫秒。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn now() -> Self {
        Self(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        )
    }

    pub fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    pub fn as_millis(&self) -> i64 {
        self.0
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::now()
    }
}

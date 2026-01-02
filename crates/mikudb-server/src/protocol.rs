//! MikuWire 二进制协议定义
//!
//! 本模块定义了 MikuDB 客户端-服务器通信的二进制协议,包括:
//! - 协议版本和魔术字节
//! - 操作码(OpCode)枚举
//! - 消息头(MessageHeader)结构
//! - 消息(Message)编解码
//! - 请求/响应数据结构

use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use std::io::{self};

/// MikuWire 协议版本号
pub const PROTOCOL_VERSION: u8 = 1;

/// 协议魔术字节,用于识别 MikuDB 协议消息
pub const MAGIC_BYTES: &[u8; 4] = b"MIKU";

/// 最大消息大小限制(64 MB),防止内存耗尽攻击
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

/// 操作码枚举
///
/// 定义了所有支持的客户端-服务器操作类型。
/// 使用 #[repr(u8)] 确保与字节表示一致,便于网络传输。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    // 心跳检测 (0x01-0x0F)
    Ping = 0x01,
    Pong = 0x02,

    // 认证操作 (0x10-0x1F)
    Auth = 0x10,
    AuthResponse = 0x11,

    // 数据操作 (0x20-0x2F)
    Query = 0x20,
    Insert = 0x21,
    Update = 0x22,
    Delete = 0x23,
    Find = 0x24,
    Aggregate = 0x25,

    // 集合操作 (0x30-0x3F)
    CreateCollection = 0x30,
    DropCollection = 0x31,
    ListCollections = 0x32,
    CreateIndex = 0x33,
    DropIndex = 0x34,
    ListIndexes = 0x35,

    // 数据库操作 (0x40-0x4F)
    CreateDatabase = 0x40,
    DropDatabase = 0x41,
    ListDatabases = 0x42,
    UseDatabase = 0x43,

    // 事务操作 (0x50-0x5F)
    BeginTransaction = 0x50,
    Commit = 0x51,
    Rollback = 0x52,

    // 响应类型 (0x80-0x8F)
    Response = 0x80,
    Error = 0x81,
    Cursor = 0x82,
    CursorNext = 0x83,
    CursorClose = 0x84,
}

impl TryFrom<u8> for OpCode {
    type Error = ();

    /// # Brief
    /// 将字节转换为操作码
    ///
    /// 从网络接收的单字节转换为对应的 OpCode 枚举值。
    ///
    /// # Arguments
    /// * `value` - 字节值
    ///
    /// # Returns
    /// 成功返回 OpCode,失败返回 Err(())
    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0x01 => Ok(OpCode::Ping),
            0x02 => Ok(OpCode::Pong),
            0x10 => Ok(OpCode::Auth),
            0x11 => Ok(OpCode::AuthResponse),
            0x20 => Ok(OpCode::Query),
            0x21 => Ok(OpCode::Insert),
            0x22 => Ok(OpCode::Update),
            0x23 => Ok(OpCode::Delete),
            0x24 => Ok(OpCode::Find),
            0x25 => Ok(OpCode::Aggregate),
            0x30 => Ok(OpCode::CreateCollection),
            0x31 => Ok(OpCode::DropCollection),
            0x32 => Ok(OpCode::ListCollections),
            0x33 => Ok(OpCode::CreateIndex),
            0x34 => Ok(OpCode::DropIndex),
            0x35 => Ok(OpCode::ListIndexes),
            0x40 => Ok(OpCode::CreateDatabase),
            0x41 => Ok(OpCode::DropDatabase),
            0x42 => Ok(OpCode::ListDatabases),
            0x43 => Ok(OpCode::UseDatabase),
            0x50 => Ok(OpCode::BeginTransaction),
            0x51 => Ok(OpCode::Commit),
            0x52 => Ok(OpCode::Rollback),
            0x80 => Ok(OpCode::Response),
            0x81 => Ok(OpCode::Error),
            0x82 => Ok(OpCode::Cursor),
            0x83 => Ok(OpCode::CursorNext),
            0x84 => Ok(OpCode::CursorClose),
            _ => Err(()),
        }
    }
}

/// 消息头结构
///
/// MikuWire 协议消息头,固定 20 字节:
/// - MAGIC (4 字节): "MIKU"
/// - version (1 字节): 协议版本
/// - opcode (1 字节): 操作码
/// - request_id (4 字节): 请求唯一标识
/// - response_to (4 字节): 响应对应的请求 ID
/// - flags (2 字节): 标志位(预留)
/// - payload_len (4 字节): 负载长度
#[derive(Debug, Clone)]
pub struct MessageHeader {
    pub version: u8,
    pub opcode: OpCode,
    pub request_id: u32,
    pub response_to: u32,
    pub flags: u16,
    pub payload_len: u32,
}

impl MessageHeader {
    /// 消息头固定大小 (20 字节)
    pub const SIZE: usize = 4 + 1 + 1 + 4 + 4 + 2 + 4;

    /// # Brief
    /// 创建新的消息头
    ///
    /// # Arguments
    /// * `opcode` - 操作码
    /// * `request_id` - 请求 ID
    /// * `payload_len` - 负载长度
    ///
    /// # Returns
    /// 初始化的消息头,version 设为 PROTOCOL_VERSION,response_to 和 flags 设为 0
    pub fn new(opcode: OpCode, request_id: u32, payload_len: u32) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            opcode,
            request_id,
            response_to: 0,
            flags: 0,
            payload_len,
        }
    }

    /// # Brief
    /// 将消息头编码为字节序列
    ///
    /// 按照 MikuWire 协议格式写入缓冲区,使用小端字节序。
    ///
    /// # Arguments
    /// * `buf` - 目标缓冲区
    pub fn encode(&self, buf: &mut BytesMut) {
        // 写入魔术字节 "MIKU"
        buf.put_slice(MAGIC_BYTES);
        // 写入协议版本
        buf.put_u8(self.version);
        // 写入操作码
        buf.put_u8(self.opcode as u8);
        // 写入请求 ID (小端)
        buf.put_u32_le(self.request_id);
        // 写入响应对应的请求 ID (小端)
        buf.put_u32_le(self.response_to);
        // 写入标志位 (小端)
        buf.put_u16_le(self.flags);
        // 写入负载长度 (小端)
        buf.put_u32_le(self.payload_len);
    }

    /// # Brief
    /// 从字节缓冲区解码消息头
    ///
    /// 验证魔术字节、操作码有效性和消息大小限制。
    ///
    /// # Arguments
    /// * `buf` - 源缓冲区
    ///
    /// # Returns
    /// - Ok(Some(header)): 成功解码消息头
    /// - Ok(None): 缓冲区数据不足,需要等待更多数据
    /// - Err: 协议错误(魔术字节错误、未知操作码、消息过大)
    pub fn decode(buf: &mut BytesMut) -> io::Result<Option<Self>> {
        // 检查缓冲区是否包含完整的消息头
        if buf.len() < Self::SIZE {
            return Ok(None);  // 需要等待更多数据
        }

        // 验证魔术字节
        let magic = &buf[0..4];
        if magic != MAGIC_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid magic bytes"));
        }

        // 解析协议版本
        let version = buf[4];
        // 解析操作码并验证有效性
        let opcode = OpCode::try_from(buf[5])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unknown opcode"))?;
        // 解析请求 ID (小端)
        let request_id = u32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]);
        // 解析响应对应的请求 ID (小端)
        let response_to = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
        // 解析标志位 (小端)
        let flags = u16::from_le_bytes([buf[14], buf[15]]);
        // 解析负载长度 (小端)
        let payload_len = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);

        // 检查消息大小,防止内存耗尽攻击
        if payload_len as usize > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Message too large"));
        }

        // 从缓冲区移除已解析的消息头
        buf.advance(Self::SIZE);

        Ok(Some(Self {
            version,
            opcode,
            request_id,
            response_to,
            flags,
            payload_len,
        }))
    }
}

/// 完整的协议消息
///
/// 包含消息头和负载数据。
#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Vec<u8>,
}

impl Message {
    /// # Brief
    /// 创建新的消息
    ///
    /// # Arguments
    /// * `opcode` - 操作码
    /// * `request_id` - 请求 ID
    /// * `payload` - 负载数据
    ///
    /// # Returns
    /// 新的消息实例
    pub fn new(opcode: OpCode, request_id: u32, payload: Vec<u8>) -> Self {
        let header = MessageHeader::new(opcode, request_id, payload.len() as u32);
        Self { header, payload }
    }

    /// # Brief
    /// 创建响应消息
    ///
    /// 使用 Response 操作码,设置 response_to 字段指向原始请求。
    ///
    /// # Arguments
    /// * `request_id` - 新的请求 ID
    /// * `response_to` - 响应对应的原始请求 ID
    /// * `payload` - 响应负载
    ///
    /// # Returns
    /// 响应消息实例
    pub fn response(request_id: u32, response_to: u32, payload: Vec<u8>) -> Self {
        let mut header = MessageHeader::new(OpCode::Response, request_id, payload.len() as u32);
        header.response_to = response_to;
        Self { header, payload }
    }

    /// # Brief
    /// 创建错误消息
    ///
    /// 使用 Error 操作码,负载为错误信息的 UTF-8 字节。
    ///
    /// # Arguments
    /// * `request_id` - 新的请求 ID
    /// * `response_to` - 响应对应的原始请求 ID
    /// * `error_msg` - 错误信息字符串
    ///
    /// # Returns
    /// 错误消息实例
    pub fn error(request_id: u32, response_to: u32, error_msg: &str) -> Self {
        let mut header = MessageHeader::new(OpCode::Error, request_id, 0);
        header.response_to = response_to;
        let payload = error_msg.as_bytes().to_vec();
        header.payload_len = payload.len() as u32;
        Self { header, payload }
    }

    /// # Brief
    /// 将完整消息编码为字节序列
    ///
    /// 编码消息头和负载数据,可直接发送到网络。
    ///
    /// # Returns
    /// 编码后的字节缓冲区
    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(MessageHeader::SIZE + self.payload.len());
        self.header.encode(&mut buf);
        buf.put_slice(&self.payload);
        buf
    }
}

/// 认证请求
///
/// 客户端发送的认证信息,JSON 序列化后作为消息负载。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub database: Option<String>,
}

/// 认证响应
///
/// 服务器返回的认证结果,包含会话 ID。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub session_id: Option<u64>,
    pub message: String,
}

/// MQL 查询请求
///
/// 包含数据库名称和 MQL 语句字符串。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub database: String,
    pub query: String,
}

/// 查询响应
///
/// 通用查询结果,包含文档列表、影响行数、游标等信息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub success: bool,
    pub affected: u64,
    pub documents: Vec<serde_json::Value>,
    pub cursor_id: Option<u64>,
    pub message: Option<String>,
}

/// 插入请求
///
/// 批量插入文档到指定集合。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertRequest {
    pub database: String,
    pub collection: String,
    pub documents: Vec<serde_json::Value>,
}

/// 更新请求
///
/// 根据过滤条件更新文档,支持 MongoDB 风格的更新操作符。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub database: String,
    pub collection: String,
    pub filter: serde_json::Value,
    pub update: serde_json::Value,
    /// 是否更新多个文档(false 只更新第一个匹配的)
    pub multi: bool,
    /// 如果不存在则插入
    pub upsert: bool,
}

/// 删除请求
///
/// 根据过滤条件删除文档。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub database: String,
    pub collection: String,
    pub filter: serde_json::Value,
    /// 是否删除多个文档(false 只删除第一个匹配的)
    pub multi: bool,
}

/// 查找请求
///
/// 支持过滤、投影、排序、限制等查询选项。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindRequest {
    pub database: String,
    pub collection: String,
    pub filter: Option<serde_json::Value>,
    pub projection: Option<serde_json::Value>,
    pub sort: Option<serde_json::Value>,
    pub limit: Option<u32>,
    pub skip: Option<u32>,
}

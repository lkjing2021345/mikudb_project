use bytes::{Buf, BufMut, BytesMut};
use mikudb_boml::{BomlValue, Document};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const PROTOCOL_VERSION: u8 = 1;
pub const MAGIC_BYTES: &[u8; 4] = b"MIKU";
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    Ping = 0x01,
    Pong = 0x02,
    Auth = 0x10,
    AuthResponse = 0x11,
    Query = 0x20,
    Insert = 0x21,
    Update = 0x22,
    Delete = 0x23,
    Find = 0x24,
    Aggregate = 0x25,
    CreateCollection = 0x30,
    DropCollection = 0x31,
    ListCollections = 0x32,
    CreateIndex = 0x33,
    DropIndex = 0x34,
    ListIndexes = 0x35,
    CreateDatabase = 0x40,
    DropDatabase = 0x41,
    ListDatabases = 0x42,
    UseDatabase = 0x43,
    BeginTransaction = 0x50,
    Commit = 0x51,
    Rollback = 0x52,
    Response = 0x80,
    Error = 0x81,
    Cursor = 0x82,
    CursorNext = 0x83,
    CursorClose = 0x84,
}

impl TryFrom<u8> for OpCode {
    type Error = ();

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
    pub const SIZE: usize = 4 + 1 + 1 + 4 + 4 + 2 + 4;

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

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_slice(MAGIC_BYTES);
        buf.put_u8(self.version);
        buf.put_u8(self.opcode as u8);
        buf.put_u32_le(self.request_id);
        buf.put_u32_le(self.response_to);
        buf.put_u16_le(self.flags);
        buf.put_u32_le(self.payload_len);
    }

    pub fn decode(buf: &mut BytesMut) -> io::Result<Option<Self>> {
        if buf.len() < Self::SIZE {
            return Ok(None);
        }

        let magic = &buf[0..4];
        if magic != MAGIC_BYTES {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid magic bytes"));
        }

        let version = buf[4];
        let opcode = OpCode::try_from(buf[5])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Unknown opcode"))?;
        let request_id = u32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]);
        let response_to = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
        let flags = u16::from_le_bytes([buf[14], buf[15]]);
        let payload_len = u32::from_le_bytes([buf[16], buf[17], buf[18], buf[19]]);

        if payload_len as usize > MAX_MESSAGE_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Message too large"));
        }

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

#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    pub payload: Vec<u8>,
}

impl Message {
    pub fn new(opcode: OpCode, request_id: u32, payload: Vec<u8>) -> Self {
        let header = MessageHeader::new(opcode, request_id, payload.len() as u32);
        Self { header, payload }
    }

    pub fn response(request_id: u32, response_to: u32, payload: Vec<u8>) -> Self {
        let mut header = MessageHeader::new(OpCode::Response, request_id, payload.len() as u32);
        header.response_to = response_to;
        Self { header, payload }
    }

    pub fn error(request_id: u32, response_to: u32, error_msg: &str) -> Self {
        let mut header = MessageHeader::new(OpCode::Error, request_id, 0);
        header.response_to = response_to;
        let payload = error_msg.as_bytes().to_vec();
        header.payload_len = payload.len() as u32;
        Self { header, payload }
    }

    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::with_capacity(MessageHeader::SIZE + self.payload.len());
        self.header.encode(&mut buf);
        buf.put_slice(&self.payload);
        buf
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
    pub database: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub success: bool,
    pub session_id: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub database: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub success: bool,
    pub affected: u64,
    pub documents: Vec<serde_json::Value>,
    pub cursor_id: Option<u64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertRequest {
    pub database: String,
    pub collection: String,
    pub documents: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub database: String,
    pub collection: String,
    pub filter: serde_json::Value,
    pub update: serde_json::Value,
    pub multi: bool,
    pub upsert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub database: String,
    pub collection: String,
    pub filter: serde_json::Value,
    pub multi: bool,
}

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

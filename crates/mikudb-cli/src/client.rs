//! 客户端连接模块
//!
//! 本模块实现 MikuDB 客户端的网络连接和协议通信:
//! - TCP 连接管理
//! - MikuWire 协议编解码
//! - 用户认证
//! - 查询请求/响应处理
//! - 自动重连和错误处理

use crate::formatter::QueryResult;
use crate::{CliError, CliResult, Config};
use bytes::BytesMut;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// 全局请求 ID 计数器,为每个请求生成唯一标识
static REQUEST_ID: AtomicU32 = AtomicU32::new(1);

/// MikuWire 协议魔术字节
const MAGIC_BYTES: &[u8; 4] = b"MIKU";
/// 协议版本号
const PROTOCOL_VERSION: u8 = 1;

/// MikuDB 客户端
///
/// 管理与 MikuDB 服务器的连接,处理认证和查询请求。
pub struct Client {
    /// TCP 连接流
    stream: TcpStream,
    /// 服务器主机名
    host: String,
    /// 服务器端口
    port: u16,
    /// 当前用户名
    user: String,
    /// 会话 ID(认证成功后设置)
    session_id: Option<u64>,
}

impl Client {
    /// # Brief
    /// 连接到 MikuDB 服务器并认证
    ///
    /// 建立 TCP 连接,执行认证,如果配置了数据库则自动切换。
    ///
    /// # Arguments
    /// * `config` - 客户端配置
    ///
    /// # Returns
    /// 已认证的客户端实例
    pub async fn connect(config: &Config) -> CliResult<Self> {
        // 连接到服务器
        let addr = format!("{}:{}", config.host, config.port);
        let stream = TcpStream::connect(&addr).await
            .map_err(|e| CliError::Connection(format!("Failed to connect to {}: {}", addr, e)))?;

        let mut client = Self {
            stream,
            host: config.host.clone(),
            port: config.port,
            user: config.user.clone(),
            session_id: None,
        };

        // 执行认证
        client.authenticate(&config.user, &config.password).await?;

        // 如果指定了数据库,自动切换
        if let Some(ref db) = config.database {
            client.use_database(db).await?;
        }

        Ok(client)
    }

    /// # Brief
    /// 获取服务器主机名
    pub fn host(&self) -> &str {
        &self.host
    }

    /// # Brief
    /// 获取服务器端口
    pub fn port(&self) -> u16 {
        self.port
    }

    /// # Brief
    /// 获取当前用户名
    pub fn user(&self) -> &str {
        &self.user
    }

    /// # Brief
    /// 执行用户认证
    ///
    /// 发送认证请求(OpCode 0x10),验证用户名和密码。
    ///
    /// # Arguments
    /// * `username` - 用户名
    /// * `password` - 密码
    ///
    /// # Returns
    /// 认证成功或失败
    async fn authenticate(&mut self, username: &str, password: &str) -> CliResult<()> {
        // 构造认证 JSON payload
        let auth_payload = serde_json::json!({
            "username": username,
            "password": password,
        });

        // 发送认证请求 (OpCode 0x10)
        let response = self.send_request(0x10, &serde_json::to_vec(&auth_payload).unwrap()).await?;

        // 解析认证响应
        let auth_response: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|e| CliError::Parse(format!("Invalid auth response: {}", e)))?;

        if auth_response["success"].as_bool().unwrap_or(false) {
            // 认证成功,保存会话 ID
            self.session_id = auth_response["session_id"].as_u64();
            Ok(())
        } else {
            // 认证失败
            Err(CliError::AuthFailed(
                auth_response["message"].as_str().unwrap_or("Unknown error").to_string()
            ))
        }
    }

    /// # Brief
    /// 切换当前数据库
    ///
    /// # Arguments
    /// * `database` - 数据库名称
    async fn use_database(&mut self, database: &str) -> CliResult<()> {
        // 发送 USE DATABASE 请求 (OpCode 0x43)
        let _ = self.send_request(0x43, database.as_bytes()).await?;
        Ok(())
    }

    /// # Brief
    /// 执行 MQL 查询
    ///
    /// 发送查询请求到服务器并解析结果。
    ///
    /// # Arguments
    /// * `query` - MQL 查询语句
    ///
    /// # Returns
    /// 查询结果
    pub async fn query(&mut self, query: &str) -> CliResult<QueryResult> {
        // 构造查询 JSON payload
        let query_payload = serde_json::json!({
            "database": "default",
            "query": query,
        });

        // 发送查询请求 (OpCode 0x20)
        let response = self.send_request(0x20, &serde_json::to_vec(&query_payload).unwrap()).await?;

        // 解析查询响应
        let result: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|e| CliError::Parse(format!("Invalid response: {}", e)))?;

        let success = result["success"].as_bool().unwrap_or(false);
        let message = result["message"].as_str().map(String::from);

        // 检查查询是否失败
        if !success {
            if let Some(msg) = message {
                return Err(CliError::Query(msg));
            }
        }

        // 返回查询结果
        Ok(QueryResult {
            success,
            affected: result["affected"].as_u64().unwrap_or(0),
            documents: result["documents"].as_array().cloned().unwrap_or_default(),
            message,
        })
    }

    /// # Brief
    /// 发送 MikuWire 协议请求并接收响应
    ///
    /// 实现完整的请求-响应周期:
    /// 1. 编码消息头(20 字节)和 payload
    /// 2. 发送到服务器
    /// 3. 读取响应头并验证
    /// 4. 读取响应 payload
    /// 5. 处理错误响应(OpCode 0x81)
    ///
    /// # Arguments
    /// * `opcode` - 操作码
    /// * `payload` - 请求负载
    ///
    /// # Returns
    /// 响应 payload
    async fn send_request(&mut self, opcode: u8, payload: &[u8]) -> CliResult<Vec<u8>> {
        // 生成唯一请求 ID
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);

        // 构造 MikuWire 消息头 (20 字节)
        let mut buf = BytesMut::with_capacity(20 + payload.len());
        buf.extend_from_slice(MAGIC_BYTES);                             // 魔术字节 "MIKU" (4 字节)
        buf.extend_from_slice(&[PROTOCOL_VERSION]);                     // 协议版本 (1 字节)
        buf.extend_from_slice(&[opcode]);                               // 操作码 (1 字节)
        buf.extend_from_slice(&request_id.to_le_bytes());               // 请求 ID (4 字节,小端)
        buf.extend_from_slice(&0u32.to_le_bytes());                     // response_to (4 字节)
        buf.extend_from_slice(&0u16.to_le_bytes());                     // flags (2 字节)
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());   // payload 长度 (4 字节)
        buf.extend_from_slice(payload);                                 // payload 数据

        // 发送请求
        self.stream.write_all(&buf).await.map_err(|e| {
            CliError::Connection(format!("Failed to send request: {}. Connection may be closed.", e))
        })?;
        self.stream.flush().await.map_err(|e| {
            CliError::Connection(format!("Failed to flush: {}. Connection may be closed.", e))
        })?;

        // 读取响应头 (20 字节)
        let mut header_buf = [0u8; 20];
        self.stream.read_exact(&mut header_buf).await.map_err(|e| {
            CliError::Connection(format!("Failed to read response header: {}. Server may have closed the connection.", e))
        })?;

        // 验证魔术字节
        if &header_buf[0..4] != MAGIC_BYTES {
            return Err(CliError::Parse("Invalid response magic bytes. Protocol mismatch or corrupted data.".into()));
        }

        // 解析响应头字段
        let response_opcode = header_buf[5];
        let payload_len = u32::from_le_bytes([header_buf[16], header_buf[17], header_buf[18], header_buf[19]]) as usize;

        // 检查 payload 大小限制 (防止内存耗尽)
        if payload_len > 64 * 1024 * 1024 {
            return Err(CliError::Parse(format!("Response payload too large: {} bytes", payload_len)));
        }

        // 读取响应 payload
        let mut payload_buf = vec![0u8; payload_len];
        self.stream.read_exact(&mut payload_buf).await.map_err(|e| {
            CliError::Connection(format!("Failed to read response payload: {}. Expected {} bytes.", e, payload_len))
        })?;

        // 检查是否为错误响应 (OpCode 0x81)
        if response_opcode == 0x81 {
            let error_msg = String::from_utf8_lossy(&payload_buf);
            return Err(CliError::Server(error_msg.to_string()));
        }

        Ok(payload_buf)
    }
}

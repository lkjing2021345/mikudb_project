//! 客户端请求处理模块
//!
//! 本模块负责处理来自客户端的所有请求,包括认证、查询、增删改查等操作。
//! 使用 MikuWire 二进制协议进行通信,支持异步处理和会话管理。

use crate::auth::UserManager;
use crate::config::ServerConfig;
use crate::protocol::*;
use crate::session::SessionManager;
use crate::{ServerError, ServerResult};
use bytes::BytesMut;
use mikudb_query::{Parser, QueryExecutor};
use mikudb_storage::StorageEngine;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{error, trace};

#[cfg(feature = "tls")]
use crate::network::StreamType;
#[cfg(feature = "tls")]
use tokio_rustls::server::TlsStream;

/// 全局请求 ID 计数器,用于为每个响应生成唯一 ID
static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

/// 客户端连接处理器
///
/// 每个客户端连接对应一个 ClientHandler 实例,负责处理该连接的所有请求。
/// 包含连接状态、认证信息、会话管理等。
pub struct ClientHandler {
    /// 连接 ID,用于日志追踪
    conn_id: u64,
    /// TCP 连接流
    stream: TcpStream,
    /// 存储引擎实例(共享)
    storage: Arc<StorageEngine>,
    /// 会话管理器(共享)
    session_manager: Arc<SessionManager>,
    /// 用户管理器(共享)
    user_manager: Arc<UserManager>,
    /// 服务器配置
    config: ServerConfig,
    /// 当前会话 ID(认证成功后设置)
    session_id: Option<u64>,
    /// 当前使用的数据库名称
    current_database: Option<String>,
    /// 是否已通过认证
    authenticated: bool,
}

impl ClientHandler {
    /// # Brief
    /// 创建新的客户端处理器
    ///
    /// # Arguments
    /// * `conn_id` - 连接唯一标识符
    /// * `stream` - TCP 连接流
    /// * `storage` - 存储引擎实例
    /// * `session_manager` - 会话管理器
    /// * `user_manager` - 用户管理器
    /// * `config` - 服务器配置
    ///
    /// # Returns
    /// 新的 ClientHandler 实例
    pub fn new(
        conn_id: u64,
        stream: TcpStream,
        storage: Arc<StorageEngine>,
        session_manager: Arc<SessionManager>,
        user_manager: Arc<UserManager>,
        config: ServerConfig,
    ) -> Self {
        // 如果认证未启用,则默认为已认证状态
        let auth_enabled = config.auth.enabled;
        Self {
            conn_id,
            stream,
            storage,
            session_manager,
            user_manager,
            config,
            session_id: None,
            current_database: None,
            authenticated: !auth_enabled,
        }
    }

    /// # Brief
    /// 处理客户端连接的主循环
    ///
    /// 持续读取客户端消息并处理,直到连接关闭或发生错误。
    /// 使用 MikuWire 协议进行消息帧解析。
    ///
    /// # Returns
    /// 连接关闭或发生错误时返回 ServerResult
    pub async fn handle(mut self) -> ServerResult<()> {
        // 创建 64KB 缓冲区用于接收数据
        let mut buf = BytesMut::with_capacity(64 * 1024);

        loop {
            // 从 TCP 流读取数据到缓冲区
            let bytes_read = self.stream.read_buf(&mut buf).await?;
            if bytes_read == 0 {
                // 客户端关闭连接
                return Err(ServerError::ConnectionClosed);
            }

            // 尝试从缓冲区解析完整的消息
            while let Some(header) = MessageHeader::decode(&mut buf)? {
                // 检查缓冲区是否包含完整的 payload
                if buf.len() < header.payload_len as usize {
                    break; // 需要等待更多数据
                }

                // 提取 payload 并构造消息
                let payload = buf.split_to(header.payload_len as usize).to_vec();
                let client_request_id = header.request_id;
                let message = Message { header, payload };

                // 处理消息并捕获错误
                let response = match self.process_message(message).await {
                    Ok(msg) => msg,
                    Err(e) => {
                        error!("Error processing message from conn {}: {}", self.conn_id, e);
                        let request_id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
                        Message::error(request_id, client_request_id, &format!("Internal error: {}", e))
                    }
                };

                // 编码并发送响应
                let encoded = response.encode();
                self.stream.write_all(&encoded).await?;
                self.stream.flush().await?;
            }
        }
    }

    /// # Brief
    /// 处理单个客户端消息
    ///
    /// 根据操作码(OpCode)分发到不同的处理函数,并进行认证检查。
    ///
    /// # Arguments
    /// * `msg` - 客户端消息
    ///
    /// # Returns
    /// 响应消息
    async fn process_message(&mut self, msg: Message) -> ServerResult<Message> {
        let request_id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        trace!("Processing {:?} from conn {}", msg.header.opcode, self.conn_id);

        match msg.header.opcode {
            // Ping-Pong 心跳检测
            OpCode::Ping => {
                Ok(Message::new(OpCode::Pong, request_id, vec![]))
            }

            // 用户认证
            OpCode::Auth => {
                self.handle_auth(&msg.payload, request_id, msg.header.request_id).await
            }

            // 以下操作均需要认证
            OpCode::Query => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_query(&msg.payload, request_id, msg.header.request_id).await
            }

            OpCode::Insert => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_insert(&msg.payload, request_id, msg.header.request_id).await
            }

            OpCode::Find => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_find(&msg.payload, request_id, msg.header.request_id).await
            }

            OpCode::Update => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_update(&msg.payload, request_id, msg.header.request_id).await
            }

            OpCode::Delete => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_delete(&msg.payload, request_id, msg.header.request_id).await
            }

            OpCode::UseDatabase => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                // 切换当前数据库
                let db_name = String::from_utf8_lossy(&msg.payload).to_string();
                self.current_database = Some(db_name.clone());
                let response = QueryResponse {
                    success: true,
                    affected: 0,
                    documents: vec![],
                    cursor_id: None,
                    message: Some(format!("Switched to database {}", db_name)),
                };
                let payload = serde_json::to_vec(&response).unwrap_or_default();
                Ok(Message::response(request_id, msg.header.request_id, payload))
            }

            OpCode::ListDatabases => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_list_databases(request_id, msg.header.request_id).await
            }

            OpCode::ListCollections => {
                if !self.authenticated {
                    return Ok(Message::error(request_id, msg.header.request_id, "Not authenticated"));
                }
                self.handle_list_collections(request_id, msg.header.request_id).await
            }

            _ => {
                Ok(Message::error(request_id, msg.header.request_id, "Unsupported operation"))
            }
        }
    }

    /// # Brief
    /// 处理用户认证请求
    ///
    /// 验证用户名和密码,成功后创建会话并设置认证状态。
    ///
    /// # Arguments
    /// * `payload` - 认证请求数据(JSON 格式)
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 认证响应消息
    async fn handle_auth(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let auth_req: AuthRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid auth request: {}", e)))?;

        match self.user_manager.authenticate(&auth_req.username, &auth_req.password).await {
            Ok(user) => {
                let session = self.session_manager.create_session(auth_req.username.clone());
                self.session_id = Some(session.id());
                self.authenticated = true;

                if let Some(db) = auth_req.database {
                    self.current_database = Some(db);
                }

                let response = AuthResponse {
                    success: true,
                    session_id: Some(session.id()),
                    message: "Authentication successful".to_string(),
                };

                let payload = serde_json::to_vec(&response).unwrap_or_default();
                Ok(Message::response(request_id, response_to, payload))
            }
            Err(_) => {
                let response = AuthResponse {
                    success: false,
                    session_id: None,
                    message: "Authentication failed".to_string(),
                };
                let payload = serde_json::to_vec(&response).unwrap_or_default();
                Ok(Message::response(request_id, response_to, payload))
            }
        }
    }

    /// # Brief
    /// 处理 MQL 查询请求
    ///
    /// 解析 MQL 语句,执行查询并返回结果。支持 CRUD、DDL、聚合等各种操作。
    ///
    /// # Arguments
    /// * `payload` - 查询请求数据(JSON 格式)
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 查询响应消息
    async fn handle_query(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        // 解析查询请求
        let query_req: QueryRequest = match serde_json::from_slice(payload) {
            Ok(req) => req,
            Err(e) => {
                let error_response = QueryResponse {
                    success: false,
                    affected: 0,
                    documents: vec![],
                    cursor_id: None,
                    message: Some(format!("Invalid query request: {}", e)),
                };
                let payload = serde_json::to_vec(&error_response).unwrap_or_default();
                return Ok(Message::response(request_id, response_to, payload));
            }
        };

        // 解析 MQL 语句
        let statement = match Parser::parse(&query_req.query) {
            Ok(stmt) => stmt,
            Err(e) => {
                let error_response = QueryResponse {
                    success: false,
                    affected: 0,
                    documents: vec![],
                    cursor_id: None,
                    message: Some(format!("Parse error: {}", e)),
                };
                let payload = serde_json::to_vec(&error_response).unwrap_or_default();
                return Ok(Message::response(request_id, response_to, payload));
            }
        };

        use mikudb_query::Statement;

        let result = match &statement {
            Statement::CreateUser(create_user) => {
                use crate::auth::RoleAssignment;
                let roles: Vec<RoleAssignment> = create_user.roles.iter().map(|r| RoleAssignment {
                    role: r.clone(),
                    db: "*".to_string(),
                }).collect();

                match self.user_manager.create_user(&create_user.username, &create_user.password, roles).await {
                    Ok(_) => mikudb_query::QueryResponse::Ok {
                        message: format!("User '{}' created successfully", create_user.username),
                    },
                    Err(e) => mikudb_query::QueryResponse::Ok {
                        message: format!("Error creating user: {}", e),
                    },
                }
            }
            Statement::AlterUser(alter_user) => {
                if let Some(ref password) = alter_user.password {
                    match self.user_manager.alter_user_password(&alter_user.username, password).await {
                        Ok(_) => mikudb_query::QueryResponse::Ok {
                            message: format!("User '{}' password updated", alter_user.username),
                        },
                        Err(e) => mikudb_query::QueryResponse::Ok {
                            message: format!("Error updating password: {}", e),
                        },
                    }
                } else {
                    mikudb_query::QueryResponse::Ok {
                        message: "No changes specified".to_string(),
                    }
                }
            }
            Statement::DropUser(username) => {
                match self.user_manager.drop_user(username).await {
                    Ok(_) => mikudb_query::QueryResponse::Ok {
                        message: format!("User '{}' dropped", username),
                    },
                    Err(e) => mikudb_query::QueryResponse::Ok {
                        message: format!("Error dropping user: {}", e),
                    },
                }
            }
            Statement::ShowUsers => {
                match self.user_manager.list_users().await {
                    Ok(users) => {
                        let user_docs: Vec<mikudb_boml::Document> = users.iter().map(|u| {
                            let mut doc = mikudb_boml::Document::new();
                            doc.insert("username".to_string(), mikudb_boml::BomlValue::String(u.username.clone().into()));
                            let roles_array: Vec<mikudb_boml::BomlValue> = u.roles.iter().map(|r| {
                                let mut role_doc = mikudb_boml::Document::new();
                                role_doc.insert("role".to_string(), mikudb_boml::BomlValue::String(r.role.clone().into()));
                                role_doc.insert("db".to_string(), mikudb_boml::BomlValue::String(r.db.clone().into()));
                                mikudb_boml::BomlValue::from(role_doc)
                            }).collect();
                            doc.insert("roles".to_string(), mikudb_boml::BomlValue::Array(roles_array));
                            doc
                        }).collect();
                        mikudb_query::QueryResponse::Documents(user_docs)
                    },
                    Err(e) => mikudb_query::QueryResponse::Ok {
                        message: format!("Error listing users: {}", e),
                    },
                }
            }
            Statement::ShowGrants(_username) => {
                mikudb_query::QueryResponse::Ok {
                    message: "SHOW GRANTS not yet implemented".to_string(),
                }
            }
            Statement::Grant(_) => {
                mikudb_query::QueryResponse::Ok {
                    message: "GRANT not yet implemented".to_string(),
                }
            }
            Statement::Revoke(_) => {
                mikudb_query::QueryResponse::Ok {
                    message: "REVOKE not yet implemented".to_string(),
                }
            }
            _ => {
                let executor = QueryExecutor::new(self.storage.clone());
                match executor.execute(&statement) {
                    Ok(res) => res,
                    Err(e) => {
                        let error_response = QueryResponse {
                            success: false,
                            affected: 0,
                            documents: vec![],
                            cursor_id: None,
                            message: Some(format!("Execution error: {}", e)),
                        };
                        let payload = serde_json::to_vec(&error_response).unwrap_or_default();
                        return Ok(Message::response(request_id, response_to, payload));
                    }
                }
            }
        };

        use mikudb_query::QueryResponse as QR;

        // 将查询结果转换为协议响应格式
        let response = match result {
            QR::Ok { message } => QueryResponse {
                success: true,
                affected: 0,
                documents: vec![],
                cursor_id: None,
                message: Some(message),
            },
            QR::Documents(docs) => QueryResponse {
                success: true,
                affected: docs.len() as u64,
                documents: docs.iter()
                    .filter_map(|d| serde_json::to_value(d).ok())
                    .collect(),
                cursor_id: None,
                message: None,
            },
            QR::Insert { inserted_count, .. } => QueryResponse {
                success: true,
                affected: inserted_count,
                documents: vec![],
                cursor_id: None,
                message: Some(format!("Inserted {} document(s)", inserted_count)),
            },
            QR::Update { matched_count, modified_count } => QueryResponse {
                success: true,
                affected: modified_count,
                documents: vec![],
                cursor_id: None,
                message: Some(format!("Matched {}, modified {}", matched_count, modified_count)),
            },
            QR::Delete { deleted_count } => QueryResponse {
                success: true,
                affected: deleted_count,
                documents: vec![],
                cursor_id: None,
                message: Some(format!("Deleted {} document(s)", deleted_count)),
            },
            QR::Databases(dbs) => QueryResponse {
                success: true,
                affected: dbs.len() as u64,
                documents: dbs.iter().map(|d| serde_json::json!({"name": d})).collect(),
                cursor_id: None,
                message: None,
            },
            QR::Collections(cols) => QueryResponse {
                success: true,
                affected: cols.len() as u64,
                documents: cols.iter().map(|c| serde_json::json!({"name": c})).collect(),
                cursor_id: None,
                message: None,
            },
            QR::Indexes(idxs) => QueryResponse {
                success: true,
                affected: idxs.len() as u64,
                documents: idxs.iter().map(|i| serde_json::json!({"name": &i.name, "fields": &i.fields})).collect(),
                cursor_id: None,
                message: None,
            },
            // SHOW STATUS 特殊处理:解析 RocksDB 统计信息
            QR::Status { size, stats } => {
                let mut status_info = serde_json::Map::new();

                // 基本信息
                status_info.insert("version".to_string(), serde_json::json!("0.1.1"));
                status_info.insert("engine".to_string(), serde_json::json!("RocksDB"));
                status_info.insert("compression".to_string(), serde_json::json!("LZ4"));

                // 存储大小
                status_info.insert("storage_size_bytes".to_string(), serde_json::json!(size));
                status_info.insert("storage_size_mb".to_string(), serde_json::json!(format!("{:.2}", size as f64 / 1024.0 / 1024.0)));

                // 遍历 RocksDB 统计信息的每一行并提取关键指标
                for line in stats.lines() {
                    let line = line.trim();

                    // 运行时间统计: "Uptime(secs): 123.4 total, 5.6 interval"
                    if line.starts_with("Uptime(secs):") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 1 {
                            let uptime_val = parts[1].trim_end_matches(',');
                            if let Ok(uptime_f) = uptime_val.parse::<f64>() {
                                status_info.insert("uptime_seconds".to_string(), serde_json::json!(format!("{:.1}", uptime_f)));
                            }
                        }
                        if parts.len() > 4 {
                            let interval_val = parts[4].trim_end_matches(',');
                            if let Ok(interval_f) = interval_val.parse::<f64>() {
                                status_info.insert("interval_seconds".to_string(), serde_json::json!(format!("{:.1}", interval_f)));
                            }
                        }
                    }

                    // 累计写入统计: "Cumulative writes: 100 writes, 200 keys, ..."
                    else if line.starts_with("Cumulative writes:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 2 {
                            status_info.insert("cumulative_writes".to_string(), serde_json::json!(parts[2]));
                        }
                        if parts.len() > 4 {
                            status_info.insert("cumulative_keys_written".to_string(), serde_json::json!(parts[4].trim_end_matches(',')));
                        }
                    }

                    // 区间写入统计: "Interval writes: 10 writes, 20 keys, ..."
                    else if line.starts_with("Interval writes:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 2 {
                            status_info.insert("interval_writes".to_string(), serde_json::json!(parts[2]));
                        }
                        if parts.len() > 4 {
                            status_info.insert("interval_keys_written".to_string(), serde_json::json!(parts[4].trim_end_matches(',')));
                        }
                    }

                    // 累计停顿时间: "Cumulative stall: 00:00:0.000 H:M:S, ..."
                    else if line.starts_with("Cumulative stall:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 2 {
                            status_info.insert("cumulative_stall_time".to_string(), serde_json::json!(parts[2].trim_end_matches(',')));
                        }
                    }

                    // 区间停顿时间
                    else if line.starts_with("Interval stall:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() > 2 {
                            status_info.insert("interval_stall_time".to_string(), serde_json::json!(parts[2].trim_end_matches(',')));
                        }
                    }

                    // 块缓存统计: "Block cache ... usage: 0.08 KB, capacity: 32.00 MB, ..."
                    else if line.contains("Block cache") && line.contains("usage:") {
                        // 提取使用量和单位
                        if let Some(usage_str) = line.split("usage:").nth(1) {
                            if let Some(usage_part) = usage_str.split_whitespace().next() {
                                status_info.insert("block_cache_usage".to_string(), serde_json::json!(usage_part));
                            }
                            if let Some(usage_remainder) = usage_str.split_whitespace().nth(1) {
                                status_info.insert("block_cache_usage_unit".to_string(), serde_json::json!(usage_remainder.trim_end_matches(',')));
                            }
                        }
                        // 提取容量和单位
                        if let Some(capacity_str) = line.split("capacity:").nth(1) {
                            if let Some(capacity_part) = capacity_str.split_whitespace().next() {
                                status_info.insert("block_cache_capacity".to_string(), serde_json::json!(capacity_part));
                            }
                            if let Some(capacity_remainder) = capacity_str.split_whitespace().nth(1) {
                                status_info.insert("block_cache_capacity_unit".to_string(), serde_json::json!(capacity_remainder.trim_end_matches(',')));
                            }
                        }
                    }

                    // 压缩 CPU 时间
                    else if line.contains("compaction.CPU") {
                        if let Some(cpu_str) = line.split(':').nth(1) {
                            status_info.insert("compaction_cpu_time".to_string(), serde_json::json!(cpu_str.trim()));
                        }
                    }

                    // 压缩写入字节数
                    else if line.contains("compaction.bytes.written") {
                        if let Some(bytes_str) = line.split(':').nth(1) {
                            status_info.insert("compaction_bytes_written".to_string(), serde_json::json!(bytes_str.trim()));
                        }
                    }

                    // 刷写 CPU 时间
                    else if line.contains("flush.CPU") {
                        if let Some(cpu_str) = line.split(':').nth(1) {
                            status_info.insert("flush_cpu_time".to_string(), serde_json::json!(cpu_str.trim()));
                        }
                    }

                    // LSM 树层级信息: "Level Files Size ..."
                    else if line.starts_with("Level") && line.contains("Files") {
                        let level_info = line.replace("  ", " ");
                        status_info.insert("storage_levels".to_string(), serde_json::json!(level_info));
                    }
                }

                QueryResponse {
                    success: true,
                    affected: 0,
                    documents: vec![serde_json::Value::Object(status_info)],
                    cursor_id: None,
                    message: None,
                }
            },
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    /// # Brief
    /// 处理文档插入请求
    ///
    /// 将 JSON 文档转换为 BOML 格式并存储到指定集合。
    ///
    /// # Arguments
    /// * `payload` - 插入请求数据(JSON 格式)
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 插入响应消息
    async fn handle_insert(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let insert_req: InsertRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid insert request: {}", e)))?;

        // 获取或创建集合
        let collection = self.storage.get_or_create_collection(&insert_req.collection)?;
        let mut inserted = 0u64;

        // 遍历并插入每个文档
        for doc_value in insert_req.documents {
            let mut doc = mikudb_boml::Document::new();
            if let serde_json::Value::Object(map) = doc_value {
                // 将 JSON 对象转换为 BOML 文档
                for (k, v) in map {
                    doc.insert(&k, json_to_boml(v));
                }
            }
            collection.insert(&mut doc)?;
            inserted += 1;
        }

        let response = QueryResponse {
            success: true,
            affected: inserted,
            documents: vec![],
            cursor_id: None,
            message: Some(format!("Inserted {} document(s)", inserted)),
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    /// # Brief
    /// 处理文档查找请求
    ///
    /// 从指定集合查找所有文档并返回。
    ///
    /// # Arguments
    /// * `payload` - 查找请求数据(JSON 格式)
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 查找响应消息,包含匹配的文档列表
    async fn handle_find(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let find_req: FindRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid find request: {}", e)))?;

        let collection = self.storage.get_collection(&find_req.collection)?;

        // 获取所有文档(后续可添加过滤器支持)
        let docs = collection.find_all()?;

        let response = QueryResponse {
            success: true,
            affected: docs.len() as u64,
            documents: docs.iter()
                .filter_map(|d| serde_json::to_value(d).ok())
                .collect(),
            cursor_id: None,
            message: None,
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    /// # Brief
    /// 处理文档更新请求
    ///
    /// 根据过滤条件更新匹配的文档,支持 $set, $inc, $unset, $push 等操作符。
    ///
    /// # Arguments
    /// * `payload` - 更新请求数据(JSON 格式)
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 更新响应消息,包含匹配和修改的文档数量
    async fn handle_update(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let update_req: UpdateRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid update request: {}", e)))?;

        let collection = self.storage.get_collection(&update_req.collection)?;
        let docs = collection.find_all()?;

        let filter_value = update_req.filter;
        let update_value = update_req.update;

        let mut modified_count = 0u64;
        let mut matched_count = 0u64;

        for mut doc in docs {
            // 应用过滤条件
            if filter_value != serde_json::Value::Null && !match_filter(&doc, &filter_value) {
                continue;
            }
            matched_count += 1;

            // 应用更新操作
            if apply_update(&mut doc, &update_value) {
                if let Some(id) = doc.id() {
                    collection.update(id, &doc)?;
                    modified_count += 1;
                }
            }

            // 如果不是多文档更新,只更新第一个匹配的文档
            if !update_req.multi {
                break;
            }
        }

        let response = QueryResponse {
            success: true,
            affected: modified_count,
            documents: vec![],
            cursor_id: None,
            message: Some(format!("Matched {}, modified {}", matched_count, modified_count)),
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    /// # Brief
    /// 处理文档删除请求
    ///
    /// 根据过滤条件删除匹配的文档。
    ///
    /// # Arguments
    /// * `payload` - 删除请求数据(JSON 格式)
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 删除响应消息,包含删除的文档数量
    async fn handle_delete(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let delete_req: DeleteRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid delete request: {}", e)))?;

        let collection = self.storage.get_collection(&delete_req.collection)?;
        let docs = collection.find_all()?;

        let filter_value = delete_req.filter;
        let mut deleted_count = 0u64;

        for doc in docs {
            // 应用过滤条件
            if filter_value != serde_json::Value::Null && !match_filter(&doc, &filter_value) {
                continue;
            }

            // 删除文档
            if let Some(id) = doc.id() {
                collection.delete(id)?;
                deleted_count += 1;
            }

            // 如果不是多文档删除,只删除第一个匹配的文档
            if !delete_req.multi {
                break;
            }
        }

        let response = QueryResponse {
            success: true,
            affected: deleted_count,
            documents: vec![],
            cursor_id: None,
            message: Some(format!("Deleted {} document(s)", deleted_count)),
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    /// # Brief
    /// 处理列出数据库请求
    ///
    /// 返回所有数据库列表(当前只有 default 数据库)。
    ///
    /// # Arguments
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 数据库列表响应消息
    async fn handle_list_databases(&mut self, request_id: u32, response_to: u32) -> ServerResult<Message> {
        let databases = vec!["default".to_string()];

        let response = QueryResponse {
            success: true,
            affected: databases.len() as u64,
            documents: databases.iter()
                .map(|d| serde_json::json!({"name": d}))
                .collect(),
            cursor_id: None,
            message: None,
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    /// # Brief
    /// 处理列出集合请求
    ///
    /// 返回当前数据库中的所有集合列表。
    ///
    /// # Arguments
    /// * `request_id` - 服务器生成的请求 ID
    /// * `response_to` - 客户端请求 ID
    ///
    /// # Returns
    /// 集合列表响应消息
    async fn handle_list_collections(&mut self, request_id: u32, response_to: u32) -> ServerResult<Message> {
        let collections = self.storage.list_collections()?;

        let response = QueryResponse {
            success: true,
            affected: collections.len() as u64,
            documents: collections.iter()
                .map(|c| serde_json::json!({"name": c}))
                .collect(),
            cursor_id: None,
            message: None,
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }
}

/// # Brief
/// 将 JSON 值转换为 BOML 值
///
/// 递归转换 JSON 数据结构为 MikuDB 的 BOML 格式。
///
/// # Arguments
/// * `value` - JSON 值
///
/// # Returns
/// 对应的 BOML 值
fn json_to_boml(value: serde_json::Value) -> mikudb_boml::BomlValue {
    use mikudb_boml::BomlValue;

    match value {
        serde_json::Value::Null => BomlValue::Null,
        serde_json::Value::Bool(b) => BomlValue::Boolean(b),
        serde_json::Value::Number(n) => {
            // 优先尝试作为整数,否则作为浮点数
            if let Some(i) = n.as_i64() {
                BomlValue::Int64(i)
            } else if let Some(f) = n.as_f64() {
                BomlValue::Float64(f)
            } else {
                BomlValue::Null
            }
        }
        serde_json::Value::String(s) => BomlValue::String(s.into()),
        serde_json::Value::Array(arr) => {
            // 递归转换数组元素
            BomlValue::Array(arr.into_iter().map(json_to_boml).collect())
        }
        serde_json::Value::Object(map) => {
            // 递归转换对象字段
            let mut doc = indexmap::IndexMap::new();
            for (k, v) in map {
                doc.insert(k.into(), json_to_boml(v));
            }
            BomlValue::Document(doc)
        }
    }
}

/// # Brief
/// 检查文档是否匹配过滤条件
///
/// 实现简单的相等匹配逻辑,支持 null、boolean、number、string 类型。
///
/// # Arguments
/// * `doc` - BOML 文档
/// * `filter` - JSON 过滤条件
///
/// # Returns
/// true 表示匹配,false 表示不匹配
fn match_filter(doc: &mikudb_boml::Document, filter: &serde_json::Value) -> bool {
    use mikudb_boml::BomlValue;

    // 过滤条件必须是对象
    let serde_json::Value::Object(filter_map) = filter else {
        return false;
    };

    // 检查每个过滤字段
    for (key, expected) in filter_map {
        let actual = match doc.get(key) {
            Some(v) => v,
            None => return false, // 文档缺少该字段
        };

        match expected {
            serde_json::Value::Null => {
                if !matches!(actual, BomlValue::Null) {
                    return false;
                }
            }
            serde_json::Value::Bool(b) => {
                if let BomlValue::Boolean(actual_b) = actual {
                    if actual_b != b {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            serde_json::Value::Number(n) => {
                // 支持整数和浮点数匹配
                let matches = if let Some(i) = n.as_i64() {
                    matches!(actual, BomlValue::Int64(v) if *v == i)
                } else if let Some(f) = n.as_f64() {
                    // 浮点数使用近似相等(避免精度问题)
                    matches!(actual, BomlValue::Float64(v) if (*v - f).abs() < 1e-10)
                } else {
                    false
                };
                if !matches {
                    return false;
                }
            }
            serde_json::Value::String(s) => {
                if let BomlValue::String(actual_s) = actual {
                    if actual_s.as_str() != s.as_str() {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            _ => return false, // 不支持的过滤类型
        }
    }

    true
}

/// # Brief
/// 应用更新操作到文档
///
/// 支持 MongoDB 风格的更新操作符:
/// - `$set`: 设置字段值
/// - `$inc`: 增加数值字段
/// - `$unset`: 删除字段
/// - `$push`: 向数组追加元素
///
/// # Arguments
/// * `doc` - 要更新的 BOML 文档(可变引用)
/// * `update` - JSON 更新操作
///
/// # Returns
/// true 表示文档被修改,false 表示未修改
fn apply_update(doc: &mut mikudb_boml::Document, update: &serde_json::Value) -> bool {
    use mikudb_boml::BomlValue;

    // 更新操作必须是对象
    let serde_json::Value::Object(update_map) = update else {
        return false;
    };

    let mut modified = false;

    for (key, value) in update_map {
        if key == "$set" {
            // $set 操作:设置字段值
            if let serde_json::Value::Object(set_map) = value {
                for (field, val) in set_map {
                    doc.insert(field, json_to_boml(val.clone()));
                    modified = true;
                }
            }
        } else if key == "$inc" {
            // $inc 操作:增加数值字段
            if let serde_json::Value::Object(inc_map) = value {
                for (field, val) in inc_map {
                    if let Some(current) = doc.get(field) {
                        // 只支持 Int64 类型的增量操作
                        if let (BomlValue::Int64(curr_i), serde_json::Value::Number(inc_n)) = (current, val) {
                            if let Some(inc_i) = inc_n.as_i64() {
                                doc.insert(field, BomlValue::Int64(curr_i + inc_i));
                                modified = true;
                            }
                        }
                    }
                }
            }
        } else if key == "$unset" {
            // $unset 操作:删除字段
            if let serde_json::Value::Object(unset_map) = value {
                for (field, _) in unset_map {
                    if doc.remove(field).is_some() {
                        modified = true;
                    }
                }
            }
        } else if key == "$push" {
            // $push 操作:向数组追加元素
            if let serde_json::Value::Object(push_map) = value {
                for (field, val) in push_map {
                    if let Some(BomlValue::Array(arr)) = doc.get_mut(field) {
                        arr.push(json_to_boml(val.clone()));
                        modified = true;
                    }
                }
            }
        } else {
            // 非操作符:直接设置字段
            doc.insert(key, json_to_boml(value.clone()));
            modified = true;
        }
    }

    modified
}

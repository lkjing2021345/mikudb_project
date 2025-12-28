use crate::auth::Authenticator;
use crate::config::ServerConfig;
use crate::protocol::*;
use crate::session::SessionManager;
use crate::{ServerError, ServerResult};
use bytes::BytesMut;
use mikudb_query::{Parser, QueryExecutor, Statement};
use mikudb_storage::StorageEngine;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, trace, warn};

static REQUEST_ID_COUNTER: AtomicU32 = AtomicU32::new(1);

pub struct ClientHandler {
    conn_id: u64,
    stream: TcpStream,
    storage: Arc<StorageEngine>,
    session_manager: Arc<SessionManager>,
    config: ServerConfig,
    session_id: Option<u64>,
    current_database: Option<String>,
    authenticated: bool,
}

impl ClientHandler {
    pub fn new(
        conn_id: u64,
        stream: TcpStream,
        storage: Arc<StorageEngine>,
        session_manager: Arc<SessionManager>,
        config: ServerConfig,
    ) -> Self {
        let auth_enabled = config.auth.enabled;
        Self {
            conn_id,
            stream,
            storage,
            session_manager,
            config,
            session_id: None,
            current_database: None,
            authenticated: !auth_enabled,
        }
    }

    pub async fn handle(mut self) -> ServerResult<()> {
        let mut buf = BytesMut::with_capacity(64 * 1024);

        loop {
            let bytes_read = self.stream.read_buf(&mut buf).await?;
            if bytes_read == 0 {
                return Err(ServerError::ConnectionClosed);
            }

            while let Some(header) = MessageHeader::decode(&mut buf)? {
                if buf.len() < header.payload_len as usize {
                    break;
                }

                let payload = buf.split_to(header.payload_len as usize).to_vec();
                let message = Message { header, payload };

                let response = self.process_message(message).await?;

                let encoded = response.encode();
                self.stream.write_all(&encoded).await?;
                self.stream.flush().await?;
            }
        }
    }

    async fn process_message(&mut self, msg: Message) -> ServerResult<Message> {
        let request_id = REQUEST_ID_COUNTER.fetch_add(1, Ordering::SeqCst);

        trace!("Processing {:?} from conn {}", msg.header.opcode, self.conn_id);

        match msg.header.opcode {
            OpCode::Ping => {
                Ok(Message::new(OpCode::Pong, request_id, vec![]))
            }

            OpCode::Auth => {
                self.handle_auth(&msg.payload, request_id, msg.header.request_id).await
            }

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

    async fn handle_auth(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let auth_req: AuthRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid auth request: {}", e)))?;

        let authenticator = Authenticator::new(&self.config.auth);

        if authenticator.verify(&auth_req.username, &auth_req.password) {
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
        } else {
            let response = AuthResponse {
                success: false,
                session_id: None,
                message: "Authentication failed".to_string(),
            };
            let payload = serde_json::to_vec(&response).unwrap_or_default();
            Ok(Message::response(request_id, response_to, payload))
        }
    }

    async fn handle_query(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
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

        let executor = QueryExecutor::new(self.storage.clone());
        let result = match executor.execute(&statement) {
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
        };

        use mikudb_query::QueryResponse as QR;

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
            QR::Status { size, stats } => QueryResponse {
                success: true,
                affected: 0,
                documents: vec![serde_json::json!({"size": size, "stats": stats})],
                cursor_id: None,
                message: None,
            },
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    async fn handle_insert(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let insert_req: InsertRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid insert request: {}", e)))?;

        let collection = self.storage.get_or_create_collection(&insert_req.collection)?;
        let mut inserted = 0u64;

        for doc_value in insert_req.documents {
            let mut doc = mikudb_boml::Document::new();
            if let serde_json::Value::Object(map) = doc_value {
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

    async fn handle_find(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let find_req: FindRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid find request: {}", e)))?;

        let collection = self.storage.get_collection(&find_req.collection)?;

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

    async fn handle_update(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let _update_req: UpdateRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid update request: {}", e)))?;

        let response = QueryResponse {
            success: true,
            affected: 0,
            documents: vec![],
            cursor_id: None,
            message: Some("Update not yet implemented".to_string()),
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

    async fn handle_delete(&mut self, payload: &[u8], request_id: u32, response_to: u32) -> ServerResult<Message> {
        let _delete_req: DeleteRequest = serde_json::from_slice(payload)
            .map_err(|e| ServerError::Protocol(format!("Invalid delete request: {}", e)))?;

        let response = QueryResponse {
            success: true,
            affected: 0,
            documents: vec![],
            cursor_id: None,
            message: Some("Delete not yet implemented".to_string()),
        };

        let payload = serde_json::to_vec(&response).unwrap_or_default();
        Ok(Message::response(request_id, response_to, payload))
    }

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

fn json_to_boml(value: serde_json::Value) -> mikudb_boml::BomlValue {
    use mikudb_boml::BomlValue;

    match value {
        serde_json::Value::Null => BomlValue::Null,
        serde_json::Value::Bool(b) => BomlValue::Boolean(b),
        serde_json::Value::Number(n) => {
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
            BomlValue::Array(arr.into_iter().map(json_to_boml).collect())
        }
        serde_json::Value::Object(map) => {
            let mut doc = indexmap::IndexMap::new();
            for (k, v) in map {
                doc.insert(k.into(), json_to_boml(v));
            }
            BomlValue::Document(doc)
        }
    }
}

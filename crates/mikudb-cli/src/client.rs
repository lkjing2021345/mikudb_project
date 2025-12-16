use crate::formatter::QueryResult;
use crate::{CliError, CliResult, Config};
use bytes::BytesMut;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

static REQUEST_ID: AtomicU32 = AtomicU32::new(1);

const MAGIC_BYTES: &[u8; 4] = b"MIKU";
const PROTOCOL_VERSION: u8 = 1;

pub struct Client {
    stream: TcpStream,
    host: String,
    port: u16,
    user: String,
    session_id: Option<u64>,
}

impl Client {
    pub async fn connect(config: &Config) -> CliResult<Self> {
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

        client.authenticate(&config.user, &config.password).await?;

        if let Some(ref db) = config.database {
            client.use_database(db).await?;
        }

        Ok(client)
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn user(&self) -> &str {
        &self.user
    }

    async fn authenticate(&mut self, username: &str, password: &str) -> CliResult<()> {
        let auth_payload = serde_json::json!({
            "username": username,
            "password": password,
        });

        let response = self.send_request(0x10, &serde_json::to_vec(&auth_payload).unwrap()).await?;

        let auth_response: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|e| CliError::Parse(format!("Invalid auth response: {}", e)))?;

        if auth_response["success"].as_bool().unwrap_or(false) {
            self.session_id = auth_response["session_id"].as_u64();
            Ok(())
        } else {
            Err(CliError::AuthFailed(
                auth_response["message"].as_str().unwrap_or("Unknown error").to_string()
            ))
        }
    }

    async fn use_database(&mut self, database: &str) -> CliResult<()> {
        let _ = self.send_request(0x43, database.as_bytes()).await?;
        Ok(())
    }

    pub async fn query(&mut self, query: &str) -> CliResult<QueryResult> {
        let query_payload = serde_json::json!({
            "database": "default",
            "query": query,
        });

        let response = self.send_request(0x20, &serde_json::to_vec(&query_payload).unwrap()).await?;

        let result: serde_json::Value = serde_json::from_slice(&response)
            .map_err(|e| CliError::Parse(format!("Invalid response: {}", e)))?;

        Ok(QueryResult {
            success: result["success"].as_bool().unwrap_or(false),
            affected: result["affected"].as_u64().unwrap_or(0),
            documents: result["documents"].as_array().cloned().unwrap_or_default(),
            message: result["message"].as_str().map(String::from),
        })
    }

    async fn send_request(&mut self, opcode: u8, payload: &[u8]) -> CliResult<Vec<u8>> {
        let request_id = REQUEST_ID.fetch_add(1, Ordering::SeqCst);

        let mut buf = BytesMut::with_capacity(20 + payload.len());
        buf.extend_from_slice(MAGIC_BYTES);
        buf.extend_from_slice(&[PROTOCOL_VERSION]);
        buf.extend_from_slice(&[opcode]);
        buf.extend_from_slice(&request_id.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&0u16.to_le_bytes());
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(payload);

        self.stream.write_all(&buf).await?;
        self.stream.flush().await?;

        let mut header_buf = [0u8; 20];
        self.stream.read_exact(&mut header_buf).await?;

        if &header_buf[0..4] != MAGIC_BYTES {
            return Err(CliError::Parse("Invalid response magic bytes".into()));
        }

        let payload_len = u32::from_le_bytes([header_buf[16], header_buf[17], header_buf[18], header_buf[19]]) as usize;

        let mut payload_buf = vec![0u8; payload_len];
        self.stream.read_exact(&mut payload_buf).await?;

        Ok(payload_buf)
    }
}

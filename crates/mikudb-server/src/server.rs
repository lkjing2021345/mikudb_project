use crate::config::ServerConfig;
use crate::handler::ClientHandler;
use crate::network::TcpListener;
use crate::session::SessionManager;
use crate::{ServerError, ServerResult};
use mikudb_core::Database;
use mikudb_storage::{StorageEngine, StorageOptions};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

pub struct Server {
    config: ServerConfig,
    databases: RwLock<HashMap<String, Arc<Database>>>,
    storage: Arc<StorageEngine>,
    session_manager: Arc<SessionManager>,
    connection_semaphore: Arc<Semaphore>,
    running: AtomicBool,
    connections_count: AtomicU64,
    requests_count: AtomicU64,
    start_time: std::time::Instant,
}

impl Server {
    pub async fn new(config: ServerConfig) -> ServerResult<Self> {
        std::fs::create_dir_all(&config.data_dir)?;

        #[cfg(target_os = "linux")]
        {
            crate::openeuler::apply_optimizations(&config)?;
        }

        let storage_opts = StorageOptions {
            data_dir: config.data_dir.clone(),
            cache_size: config.parse_cache_size(),
            ..Default::default()
        };

        info!("Initializing storage engine at {:?}", config.data_dir);
        let storage = Arc::new(StorageEngine::open(storage_opts)?);

        let session_manager = Arc::new(SessionManager::new(
            std::time::Duration::from_secs(3600),
        ));

        let connection_semaphore = Arc::new(Semaphore::new(config.max_connections));

        Ok(Self {
            config,
            databases: RwLock::new(HashMap::new()),
            storage,
            session_manager,
            connection_semaphore,
            running: AtomicBool::new(false),
            connections_count: AtomicU64::new(0),
            requests_count: AtomicU64::new(0),
            start_time: std::time::Instant::now(),
        })
    }

    pub async fn run(self: Arc<Self>) -> ServerResult<()> {
        self.running.store(true, Ordering::SeqCst);

        let addr = format!("{}:{}", self.config.bind, self.config.port);
        let listener = TcpListener::bind(&addr, &self.config).await?;

        info!("MikuDB server listening on {}", addr);

        #[cfg(target_os = "linux")]
        if let Some(ref socket_path) = self.config.unix_socket {
            info!("Unix socket enabled at {}", socket_path);
        }

        while self.running.load(Ordering::SeqCst) {
            let permit = self.connection_semaphore.clone().acquire_owned().await;

            match listener.accept().await {
                Ok((stream, addr)) => {
                    let permit = permit.map_err(|_| ServerError::Internal("Semaphore closed".into()))?;
                    let server = self.clone();
                    let conn_id = self.connections_count.fetch_add(1, Ordering::SeqCst);

                    debug!("New connection {} from {}", conn_id, addr);

                    tokio::spawn(async move {
                        let handler = ClientHandler::new(
                            conn_id,
                            stream,
                            server.storage.clone(),
                            server.session_manager.clone(),
                            server.config.clone(),
                        );

                        if let Err(e) = handler.handle().await {
                            if !matches!(e, ServerError::ConnectionClosed) {
                                warn!("Connection {} error: {}", conn_id, e);
                            }
                        }

                        debug!("Connection {} closed", conn_id);
                        drop(permit);
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }

        Ok(())
    }

    pub fn shutdown(&self) {
        info!("Shutting down server...");
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn stats(&self) -> ServerStats {
        ServerStats {
            uptime_secs: self.start_time.elapsed().as_secs(),
            total_connections: self.connections_count.load(Ordering::Relaxed),
            total_requests: self.requests_count.load(Ordering::Relaxed),
            active_sessions: self.session_manager.active_count(),
        }
    }

    pub fn increment_requests(&self) {
        self.requests_count.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone)]
pub struct ServerStats {
    pub uptime_secs: u64,
    pub total_connections: u64,
    pub total_requests: u64,
    pub active_sessions: usize,
}

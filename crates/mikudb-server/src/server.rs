//! 数据库服务器主模块
//!
//! 本模块实现 MikuDB 服务器核心逻辑:
//! - 服务器生命周期管理(启动、运行、关闭)
//! - 连接池管理(使用 Semaphore 限制并发连接数)
//! - 存储引擎初始化
//! - 会话管理
//! - 统计信息收集

use crate::config::ServerConfig;
use crate::handler::ClientHandler;
use crate::network::TcpListener;
use crate::session::SessionManager;
use crate::auth::UserManager;
use crate::{ServerError, ServerResult};
use mikudb_core::Database;
use mikudb_storage::{StorageEngine, StorageOptions};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

#[cfg(feature = "tls")]
use crate::network::StreamType;
#[cfg(feature = "tls")]
use tokio::sync::OwnedSemaphorePermit;

/// MikuDB 服务器
///
/// 管理所有客户端连接、数据库实例、存储引擎和会话。
/// 使用 Arc 包裹以支持多线程共享。
pub struct Server {
    /// 服务器配置
    config: ServerConfig,
    /// 数据库实例映射表(database_name -> Database)
    databases: RwLock<HashMap<String, Arc<Database>>>,
    /// 存储引擎(共享)
    storage: Arc<StorageEngine>,
    /// 会话管理器(共享)
    session_manager: Arc<SessionManager>,
    /// 用户管理器(共享)
    user_manager: Arc<UserManager>,
    /// 连接信号量,限制最大并发连接数
    connection_semaphore: Arc<Semaphore>,
    /// 服务器运行状态
    running: AtomicBool,
    /// 累计连接数
    connections_count: AtomicU64,
    /// 累计请求数
    requests_count: AtomicU64,
    /// 服务器启动时间
    start_time: std::time::Instant,
}

impl Server {
    /// # Brief
    /// 创建新的服务器实例
    ///
    /// 初始化存储引擎、会话管理器和连接池。
    /// 在 Linux 系统上会应用 OpenEuler 性能优化。
    ///
    /// # Arguments
    /// * `config` - 服务器配置
    ///
    /// # Returns
    /// 初始化好的服务器实例
    pub async fn new(config: ServerConfig) -> ServerResult<Self> {
        // 创建数据目录(如果不存在)
        std::fs::create_dir_all(&config.data_dir)?;

        // 在 Linux 系统上应用性能优化(OpenEuler 特定)
        #[cfg(target_os = "linux")]
        {
            crate::openeuler::apply_optimizations(&config)?;
        }

        // 配置存储引擎选项
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

        let user_manager = Arc::new(UserManager::new(storage.clone()));

        if config.auth.enabled {
            user_manager.initialize().await?;
        }

        let connection_semaphore = Arc::new(Semaphore::new(config.max_connections));

        Ok(Self {
            config,
            databases: RwLock::new(HashMap::new()),
            storage,
            session_manager,
            user_manager,
            connection_semaphore,
            running: AtomicBool::new(false),
            connections_count: AtomicU64::new(0),
            requests_count: AtomicU64::new(0),
            start_time: std::time::Instant::now(),
        })
    }

    /// # Brief
    /// 启动服务器主循环
    ///
    /// 监听 TCP 端口,接受客户端连接并为每个连接创建独立的处理任务。
    /// 使用 Semaphore 限制并发连接数,防止资源耗尽。
    ///
    /// # Returns
    /// 服务器关闭或发生错误时返回
    pub async fn run(self: Arc<Self>) -> ServerResult<()> {
        // 设置运行状态
        self.running.store(true, Ordering::SeqCst);

        // 绑定 TCP 监听地址
        let addr = format!("{}:{}", self.config.bind, self.config.port);
        let listener = TcpListener::bind(&addr, &self.config).await?;

        info!("MikuDB server listening on {}", addr);

        #[cfg(feature = "tls")]
        if self.config.tls.enabled {
            info!("TLS enabled - accepting encrypted connections");
        }

        // 在 Linux 上同时启用 Unix Socket 支持
        #[cfg(target_os = "linux")]
        if let Some(ref socket_path) = self.config.unix_socket {
            info!("Unix socket enabled at {}", socket_path);
        }

        // 主循环:接受客户端连接
        while self.running.load(Ordering::SeqCst) {
            // 获取连接许可(阻塞直到有可用槽位)
            let permit = self.connection_semaphore.clone().acquire_owned().await;

            #[cfg(feature = "tls")]
            let accept_result = if self.config.tls.enabled {
                listener.accept_tls().await.map(|(stream, addr)| (Some(stream), addr))
            } else {
                listener.accept().await.map(|(stream, addr)| (None, addr))
            };

            #[cfg(not(feature = "tls"))]
            let accept_result = listener.accept().await.map(|(stream, addr)| (stream, addr));

            // 接受新连接
            match accept_result {
                #[cfg(feature = "tls")]
                Ok((Some(tls_stream), addr)) => {
                    let permit = permit.map_err(|_| ServerError::Internal("Semaphore closed".into()))?;
                    let server = self.clone();
                    let conn_id = self.connections_count.fetch_add(1, Ordering::SeqCst);

                    debug!("New TLS connection {} from {}", conn_id, addr);

                    tokio::spawn(async move {
                        if let Err(e) = handle_tls_connection(conn_id, tls_stream, server, permit).await {
                            if !matches!(e, ServerError::ConnectionClosed) {
                                warn!("TLS connection {} error: {}", conn_id, e);
                            }
                        }
                        debug!("TLS connection {} closed", conn_id);
                    });
                }
                #[cfg(not(feature = "tls"))]
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
                            server.user_manager.clone(),
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
                #[cfg(feature = "tls")]
                Ok((None, addr)) => {
                    let permit = permit.map_err(|_| ServerError::Internal("Semaphore closed".into()))?;
                    let server = self.clone();
                    let conn_id = self.connections_count.fetch_add(1, Ordering::SeqCst);

                    debug!("New connection {} from {}", conn_id, addr);

                    tokio::spawn(async move {
                        if let Ok((stream, _)) = listener.accept().await {
                            let handler = ClientHandler::new(
                                conn_id,
                                stream,
                                server.storage.clone(),
                                server.session_manager.clone(),
                                server.user_manager.clone(),
                                server.config.clone(),
                            );

                            if let Err(e) = handler.handle().await {
                                if !matches!(e, ServerError::ConnectionClosed) {
                                    warn!("Connection {} error: {}", conn_id, e);
                                }
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

    /// # Brief
    /// 关闭服务器
    ///
    /// 设置运行状态为 false,主循环将在下次检查时退出。
    pub fn shutdown(&self) {
        info!("Shutting down server...");
        self.running.store(false, Ordering::SeqCst);
    }

    /// # Brief
    /// 获取服务器统计信息
    ///
    /// # Returns
    /// 包含运行时间、连接数、请求数、活跃会话数的统计结构
    pub fn stats(&self) -> ServerStats {
        ServerStats {
            uptime_secs: self.start_time.elapsed().as_secs(),
            total_connections: self.connections_count.load(Ordering::Relaxed),
            total_requests: self.requests_count.load(Ordering::Relaxed),
            active_sessions: self.session_manager.active_count(),
        }
    }

    /// # Brief
    /// 增加请求计数器
    ///
    /// 由各个请求处理器调用以统计总请求数。
    pub fn increment_requests(&self) {
        self.requests_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// 服务器统计信息
///
/// 包含服务器运行时的各项指标。
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub uptime_secs: u64,
    pub total_connections: u64,
    pub total_requests: u64,
    pub active_sessions: usize,
}

#[cfg(feature = "tls")]
async fn handle_tls_connection(
    conn_id: u64,
    stream: StreamType,
    server: Arc<Server>,
    permit: OwnedSemaphorePermit,
) -> ServerResult<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use crate::protocol::*;

    match stream {
        StreamType::Tls(mut tls_stream) => {
            let mut buf = vec![0u8; 4096];
            loop {
                match tls_stream.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if let Err(e) = tls_stream.write_all(&buf[..n]).await {
                            warn!("TLS write error on connection {}: {}", conn_id, e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("TLS read error on connection {}: {}", conn_id, e);
                        break;
                    }
                }
            }
        }
        StreamType::Tcp(stream) => {
            let handler = ClientHandler::new(
                conn_id,
                stream,
                server.storage.clone(),
                server.session_manager.clone(),
                server.user_manager.clone(),
                server.config.clone(),
            );
            handler.handle().await?;
        }
    }

    drop(permit);
    Ok(())
}

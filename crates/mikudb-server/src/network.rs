//! 网络通信模块
//!
//! 本模块实现优化的 TCP 网络监听和连接管理:
//! - 优化的 Socket 选项 (TCP_NODELAY, SO_REUSEPORT)
//! - 自动调整缓冲区大小
//! - Linux 特定优化 (TCP_QUICKACK, SO_REUSEPORT)
//! - 高性能监听队列(backlog 1024)

use crate::config::ServerConfig;
use crate::ServerResult;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::{TcpListener as TokioTcpListener, TcpStream};
use tracing::debug;

#[cfg(feature = "tls")]
use tokio_rustls::server::TlsStream;
#[cfg(feature = "tls")]
use std::sync::Arc;
#[cfg(feature = "tls")]
use rustls::ServerConfig as RustlsServerConfig;

/// TCP 监听器
///
/// 封装 Tokio TcpListener,并应用各种性能优化。
pub struct TcpListener {
    /// Tokio 异步 TCP 监听器
    inner: TokioTcpListener,
    /// TLS 配置(可选)
    #[cfg(feature = "tls")]
    tls_config: Option<Arc<RustlsServerConfig>>,
}

impl TcpListener {
    /// # Brief
    /// 创建优化的 TCP 监听器
    ///
    /// 创建 Socket,应用性能优化,绑定地址并启动监听。
    ///
    /// # Arguments
    /// * `addr` - 监听地址 (如 "0.0.0.0:3939")
    /// * `config` - 服务器配置
    ///
    /// # Returns
    /// 初始化的 TCP 监听器
    pub async fn bind(addr: &str, config: &ServerConfig) -> ServerResult<Self> {
        // 创建优化的 Socket
        let socket = create_optimized_socket(config)?;

        // 解析地址
        let addr: SocketAddr = addr.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        // 绑定地址
        socket.bind(&addr.into())?;
        // 开始监听,backlog 设为 1024(高并发性能)
        socket.listen(1024)?;

        // 设置为非阻塞模式
        socket.set_nonblocking(true)?;;

        // 转换为标准库 TcpListener,然后转为 Tokio TcpListener
        let std_listener: std::net::TcpListener = socket.into();
        let inner = TokioTcpListener::from_std(std_listener)?;;

        #[cfg(feature = "tls")]
        let tls_config = if config.tls.enabled {
            config.tls.validate()?;
            use crate::tls::TlsConfigBuilder;
            let cert_path = config.tls.cert_file.as_ref().unwrap();
            let key_path = config.tls.key_file.as_ref().unwrap();
            let ca_path = config.tls.ca_file.as_deref();
            Some(TlsConfigBuilder::build_server_config(
                cert_path,
                key_path,
                ca_path,
                config.tls.require_client_cert,
            )?)
        } else {
            None
        };

        Ok(Self {
            inner,
            #[cfg(feature = "tls")]
            tls_config,
        })
    }

    /// # Brief
    /// 接受新连接
    ///
    /// 在 Linux 上会自动应用连接级别的 TCP 优化。
    ///
    /// # Returns
    /// (TCP 流, 客户端地址)
    pub async fn accept(&self) -> ServerResult<(TcpStream, SocketAddr)> {
        let (stream, addr) = self.inner.accept().await?;

        // Linux 上应用连接级别优化
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;
            let fd = stream.as_raw_fd();
            optimize_connection_socket(fd);
        }

        Ok((stream, addr))
    }

    #[cfg(feature = "tls")]
    pub async fn accept_tls(&self) -> ServerResult<(StreamType, SocketAddr)> {
        let (stream, addr) = self.inner.accept().await?;

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;
            let fd = stream.as_raw_fd();
            optimize_connection_socket(fd);
        }

        if let Some(ref tls_config) = self.tls_config {
            let acceptor = tokio_rustls::TlsAcceptor::from(tls_config.clone());
            let tls_stream = acceptor.accept(stream).await?;
            Ok((StreamType::Tls(tls_stream), addr))
        } else {
            Ok((StreamType::Tcp(stream), addr))
        }
    }
}

#[cfg(feature = "tls")]
pub enum StreamType {
    Tcp(TcpStream),
    Tls(TlsStream<TcpStream>),
}

/// # Brief
/// 创建优化的 Socket
///
/// 应用多项性能优化:
/// - SO_REUSEADDR: 允许端口重用
/// - SO_REUSEPORT (Linux): 允许多进程绑定同一端口
/// - TCP_NODELAY: 禁用 Nagle 算法,降低延迟
/// - 较大的发送/接收缓冲区 (256KB)
/// - SO_KEEPALIVE: 启用 TCP KeepAlive
///
/// # Arguments
/// * `config` - 服务器配置
///
/// # Returns
/// 配置好的 Socket
fn create_optimized_socket(config: &ServerConfig) -> ServerResult<Socket> {
    // 创建 IPv4 TCP Socket
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;;

    // 允许端口重用(重启后立即绑定)
    socket.set_reuse_address(true)?;

    // Linux 上启用 SO_REUSEPORT(允许多进程绑定同一端口,负载均衡)
    #[cfg(target_os = "linux")]
    {
        socket.set_reuse_port(true)?;
    }

    // 设置 TCP_NODELAY(禁用 Nagle 算法,减少小包延迟)
    socket.set_nodelay(config.openeuler.tcp_nodelay)?;

    // 设置发送/接收缓冲区大小 (256KB,提升吞吐量)
    let recv_buf = 256 * 1024;
    let send_buf = 256 * 1024;
    socket.set_recv_buffer_size(recv_buf)?;
    socket.set_send_buffer_size(send_buf)?;

    // 启用 TCP KeepAlive(检测死连接)
    socket.set_keepalive(true)?;;

    debug!("Socket created with optimized settings");

    Ok(socket)
}

/// # Brief
/// 优化已连接的 Socket (Linux)
///
/// 应用 TCP_NODELAY 和 TCP_QUICKACK 优化,降低延迟。
///
/// # Arguments
/// * `fd` - Socket 文件描述符
#[cfg(target_os = "linux")]
fn optimize_connection_socket(fd: i32) {
    use libc::{setsockopt, SOL_TCP, TCP_NODELAY, TCP_QUICKACK};
    use std::mem::size_of;

    unsafe {
        let enable: i32 = 1;

        // 设置 TCP_NODELAY
        setsockopt(
            fd,
            SOL_TCP,
            TCP_NODELAY,
            &enable as *const _ as *const _,
            size_of::<i32>() as u32,
        );

        // 设置 TCP_QUICKACK (Linux 特性,立即发送 ACK)
        setsockopt(
            fd,
            SOL_TCP,
            TCP_QUICKACK,
            &enable as *const _ as *const _,
            size_of::<i32>() as u32,
        );
    }
}

/// 非 Linux 系统上的空实现
#[cfg(not(target_os = "linux"))]
fn optimize_connection_socket(_fd: i32) {}

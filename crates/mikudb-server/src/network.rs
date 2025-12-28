use crate::config::ServerConfig;
use crate::ServerResult;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::SocketAddr;
use tokio::net::{TcpListener as TokioTcpListener, TcpStream};
use tracing::debug;

pub struct TcpListener {
    inner: TokioTcpListener,
}

impl TcpListener {
    pub async fn bind(addr: &str, config: &ServerConfig) -> ServerResult<Self> {
        let socket = create_optimized_socket(config)?;

        let addr: SocketAddr = addr.parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        socket.bind(&addr.into())?;
        socket.listen(1024)?;

        socket.set_nonblocking(true)?;

        let std_listener: std::net::TcpListener = socket.into();
        let inner = TokioTcpListener::from_std(std_listener)?;

        Ok(Self { inner })
    }

    pub async fn accept(&self) -> ServerResult<(TcpStream, SocketAddr)> {
        let (stream, addr) = self.inner.accept().await?;

        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;
            let fd = stream.as_raw_fd();
            optimize_connection_socket(fd);
        }

        Ok((stream, addr))
    }
}

fn create_optimized_socket(config: &ServerConfig) -> ServerResult<Socket> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

    socket.set_reuse_address(true)?;

    #[cfg(target_os = "linux")]
    {
        socket.set_reuse_port(true)?;
    }

    socket.set_nodelay(config.openeuler.tcp_nodelay)?;

    let recv_buf = 256 * 1024;
    let send_buf = 256 * 1024;
    socket.set_recv_buffer_size(recv_buf)?;
    socket.set_send_buffer_size(send_buf)?;

    socket.set_keepalive(true)?;

    debug!("Socket created with optimized settings");

    Ok(socket)
}

#[cfg(target_os = "linux")]
fn optimize_connection_socket(fd: i32) {
    use libc::{setsockopt, SOL_TCP, TCP_NODELAY, TCP_QUICKACK};
    use std::mem::size_of;

    unsafe {
        let enable: i32 = 1;

        setsockopt(
            fd,
            SOL_TCP,
            TCP_NODELAY,
            &enable as *const _ as *const _,
            size_of::<i32>() as u32,
        );

        setsockopt(
            fd,
            SOL_TCP,
            TCP_QUICKACK,
            &enable as *const _ as *const _,
            size_of::<i32>() as u32,
        );
    }
}

#[cfg(not(target_os = "linux"))]
fn optimize_connection_socket(_fd: i32) {}


use crate::deps;

use std::net::SocketAddr;
use std::net::{
    TcpListener,
    TcpStream,
};

use deps::socket2;
use deps::tokio;

use socket2::{
    Socket,
    Domain,
    Type,
};

use std::io::Error;

pub const DEFAULT_BACKLOG: i32 = 1024;

/// bind with port 0 to get an available port for client connections
pub fn listen(port: u16, backlog: Option<i32>, device: Option<&[u8]>) -> Result<TcpListener, Error> {
    let socket_addr = crate::inet::socket_addr_unspecified(port);
    let backlog = backlog.unwrap_or(DEFAULT_BACKLOG);

    let socket = Socket::new(Domain::IPV6, Type::STREAM, None)?;
    socket.set_only_v6(false)?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;

    #[cfg(target_os = "linux")]
    if device.is_some() {
        socket.bind_device(device)?;
    }

    #[cfg(not(target_os = "linux"))]
    if device.is_some() {
        log::warn!("Ignoring device binding on non-linux platform");
    }

    socket.bind(&socket_addr.into())?;

    socket.listen(backlog.into())?;
    let listener: TcpListener = socket.into();

    Ok(listener)
}

pub async fn connect(addr: SocketAddr, device: Option<&[u8]>) -> Result<tokio::net::TcpStream, Error> {
    let socket = match &addr {
        SocketAddr::V4(_) => Socket::new(Domain::IPV4, Type::STREAM, None)?,
        SocketAddr::V6(_) => Socket::new(Domain::IPV6, Type::STREAM, None)?,
    };

    #[cfg(target_os = "linux")]
    if device.is_some() {
        socket.bind_device(device)?;
    }

    #[cfg(not(target_os = "linux"))]
    if device.is_some() {
        log::warn!("Ignoring device binding on non-linux platform");
    }

    let socket: TcpStream = socket.into();
    let socket = tokio::net::TcpSocket::from_std_stream(socket);

    let stream = socket.connect(addr).await?;
    Ok(stream)
}

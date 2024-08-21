
use crate::deps;
use crate::inet;

use deps::tokio::net::UdpSocket;

use deps::net2::UdpBuilder;

use std::io::Error;

/// bind with port 0 to get an available port for client connections
pub fn bind_socket(port: u16, device: Option<&[u8]>) -> Result<UdpSocket, Error> {
    let socket_addr = inet::socket_addr_unspecified(port);
    let socket = UdpBuilder::new_v6()?
        .only_v6(false)?
        .bind(socket_addr)?;
    socket.set_nonblocking(true)?;
    let socket = UdpSocket::from_std(socket)?;

    #[cfg(target_os = "linux")]
    if device.is_some() {
        socket.bind_device(device)?;
    }

    #[cfg(not(target_os = "linux"))]
    if device.is_some() {
        log::warn!("Ignoring device binding on non-linux platform");
    }

    Ok(socket)
}

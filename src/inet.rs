
use std::net::{
    IpAddr,
    Ipv6Addr,
    SocketAddr,
};

use std::time::Duration;

pub const V6_UNSPECIFIED: IpAddr = IpAddr::V6(Ipv6Addr::UNSPECIFIED);

pub const fn socket_addr_unspecified(port: u16) -> SocketAddr {
    SocketAddr::new(V6_UNSPECIFIED, port)
}

pub const CONNECTION_ATTEMPT_DELAY: Duration = Duration::from_millis(250);

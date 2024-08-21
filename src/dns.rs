
use crate::deps;

use std::net::{
    Ipv4Addr,
    Ipv6Addr,
    SocketAddr,
    ToSocketAddrs,
};

use std::io::Error;
use std::sync::Arc;

use deps::threadpool::ThreadPool;
use deps::tokio::sync::oneshot;
use deps::parking_lot::RwLock;

#[derive(Debug, Clone)]
pub struct HostAddrs {
    pub addrs_v4: Vec<Ipv4Addr>,
    pub addrs_v6: Vec<Ipv6Addr>,
}

#[derive(Debug, Clone)]
pub struct DnsResolver {
    pool: Arc<RwLock<ThreadPool>>,
}

impl DnsResolver {
    pub const DEFAULT_THREADS: usize = 8;

    pub fn new(threads: usize) -> Self {
        let pool = Arc::new(RwLock::new(ThreadPool::new(threads)));
        Self { pool }
    }

    pub fn new_auto() -> Self {
        let num = std::thread::available_parallelism().ok().map(|n| n.get()).unwrap_or(Self::DEFAULT_THREADS);
        assert!(num > 0, "available_parallelism() returned 0");
        Self::new(num)
    }

    pub async fn resolve_host(&self, host: &str) -> Result<HostAddrs, Error> {
        let (tx, rx) = oneshot::channel();
        let host = host.to_owned();
        self.pool.read().execute(move || {
            let addrs = match host.to_socket_addrs() {
                Ok(addrs) => addrs,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };

            let mut addrs_v4 = Vec::new();
            let mut addrs_v6 = Vec::new();
            let mut count: usize = 0;

            for addr in addrs {
                count += 1;
                match addr {
                    SocketAddr::V4(v4) => {
                        addrs_v4.push(*v4.ip());
                    }
                    SocketAddr::V6(v6) => {
                        addrs_v6.push(*v6.ip());
                    }
                }
            }

            if count == 0 {
                let _ = tx.send(Err(Error::new(std::io::ErrorKind::AddrNotAvailable, "no addresses found")));
                return;
            }

            let _ = tx.send(Ok(HostAddrs { addrs_v4, addrs_v6 }));
        });

        rx.await.map_err(|_| Error::new(std::io::ErrorKind::Interrupted, "thread interrupted"))?
    }
}

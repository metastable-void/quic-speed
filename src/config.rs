
use crate::deps;

use std::path::{
    PathBuf,
    Path,
};
use std::str::FromStr;
use std::io::{
    Error,
    ErrorKind,
};

use deps::toml;
use deps::serde::Deserialize;
use deps::tokio_rustls::TlsAcceptor;
use deps::rustls_pemfile;
use deps::tokio_rustls::rustls;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub tls_cert: PathBuf,
    pub tls_key: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
}

impl Config {
    /// blockingly load config from file
    pub fn load(path: &Path) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path)?;
        let config = toml::from_str(&content).map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
        Ok(config)
    }

    pub fn tls_acceptor(&self) -> Result<TlsAcceptor, Error> {
        let tls_cert = self.server.tls_cert.clone();
        let tls_key = self.server.tls_key.clone();
        let certfile = std::fs::File::open(&tls_cert)
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to open {}: {}", tls_cert.to_string_lossy(), e)))?;
        let mut reader = std::io::BufReader::new(certfile);
        let certs = rustls_pemfile::certs(&mut reader).filter_map(|item| item.ok()).collect();

        let keyfile = std::fs::File::open(&tls_key)
            .map_err(|e| Error::new(ErrorKind::Other, format!("failed to open {}: {}", tls_key.to_string_lossy(), e)))?;
        let mut reader = std::io::BufReader::new(keyfile);
        let key = rustls_pemfile::private_key(&mut reader)?;
        let key = if let Some(key) = key {
            key
        } else {
            return Err(Error::new(ErrorKind::NotFound, "no private key found"));
        };

        let mut server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| Error::new(ErrorKind::InvalidData, e.to_string()))?;
        server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec(), b"http/1.0".to_vec()];

        let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));
        Ok(tls_acceptor)
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

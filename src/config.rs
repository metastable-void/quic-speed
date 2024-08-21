
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
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        toml::from_str(s)
    }
}

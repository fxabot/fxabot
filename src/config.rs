use std::fs::File;
use std::io::{self, Read};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use toml;

// should configs ever big bigger than 50mb?
const MAX_CONFIG_FILE_SIZE: u64 = 1024 * 1024 * 50;

#[derive(Clone, Debug)]
pub struct Config(Arc<Inner>);


#[derive(Debug, Deserialize)]
pub struct Inner {
    github: Option<Github>,
    server: Server,
}

#[derive(Debug, Deserialize)]
struct Github {
    username: String,
    authorized: Vec<String>,
    api: Option<String>,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Server {
    host: IpAddr,
    port: Option<u16>,
}

impl Config {

    pub fn parse_file<T: AsRef<Path>>(path: T) -> Result<Config, Error> {
        let mut file = File::open(path)?;
        let file_size = file.metadata()?.len();
        assert!(file_size <= MAX_CONFIG_FILE_SIZE);

        let mut contents = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut contents)?;
        toml::from_slice(&contents)
            .map(|inner| Config(Arc::new(inner)))
            .map_err(From::from)
    }

    pub fn server_addr(&self) -> SocketAddr {
        SocketAddr::new(self.0.server.host, self.0.server.port.unwrap_or(0))
    }

    pub fn github_name(&self) -> &str {
        self.0.github.as_ref().map(|g| g.username.as_ref()).unwrap_or("")
    }

    pub fn github_authorized(&self) -> &[String] {
        self.0.github.as_ref().map(|g| g.authorized.as_ref()).unwrap_or(&[])
    }

    pub fn github_api(&self) -> &str {
        self.0.github.as_ref()
            .and_then(|g| g.api.as_ref().map(AsRef::as_ref))
            .unwrap_or("https://api.github.com")
    }

    pub fn github_token(&self) -> Option<&str> {
        self.0.github.as_ref()
            .and_then(|g| g.token.as_ref().map(AsRef::as_ref))
    }
}

impl FromStr for Config {
    type Err = toml::de::Error;

    fn from_str(s: &str) -> Result<Config, Self::Err> {
        toml::from_str(s)
            .map(|inner| Config(Arc::new(inner)))
    }
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    Io(io::Error),
    Toml(toml::de::Error),
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error {
            kind: ErrorKind::Io(e),
        }
    }
}

impl From<toml::de::Error> for Error {
    fn from(e: toml::de::Error) -> Error {
        Error {
            kind: ErrorKind::Toml(e),
        }
    }
}

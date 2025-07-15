use crate::peer::Peer;
use log;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time;
use std::{fs, io};
use toml;
use url::Url;

use crate::errors::{ConfigError, NetworkTargetError};
use crate::security::Security;

const LOG_FILE_SIZE_HARD_LIMIT_MB: usize = 10;

#[derive(Debug, Deserialize, Default)]
enum JalbConfigVersion {
    #[default]
    #[serde(rename = "1")]
    V1,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum LoadBalancerType {
    #[serde(rename = "application")]
    Application,
    #[serde(rename = "network")]
    Network,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum LoadBalancerStrategy {
    #[serde(rename = "round_robin")]
    RoundRobin,
    #[serde(rename = "least_used")]
    LeastUsed,
    #[serde(rename = "weighted_average")]
    WeightedAverage,
    #[serde(rename = "geo")]
    Geolocation,
}

#[derive(Debug, Deserialize)]
pub struct LoadBalancerConfig {
    #[serde(rename = "type")]
    load_balancer_type: LoadBalancerType,
    strategy: LoadBalancerStrategy,
    listener_address: Option<IpAddr>,
    port: Option<u16>,
    max_connections: u32,
    max_requests_per_connection: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BackendOptions {
    pub name: String,
    pub health_endpoint: Option<String>,
    health_check_interval_seconds: Option<u32>,
    health_check_timeout_seconds: Option<u32>,
    pub failed_request_threshold: Option<u32>,
    request_timeout_seconds: Option<u32>,
    pub rate_limit: Option<u64>,
    pub peers: Vec<PeerConfig>,
}

impl BackendOptions {
    pub fn get_health_check_interval(&self) -> Option<time::Duration> {
        if let Some(interval) = self.health_check_interval_seconds {
            return Some(time::Duration::from_secs(interval.into()));
        }

        None
    }

    pub fn get_health_check_timeout(&self) -> Option<time::Duration> {
        if let Some(timeout) = self.health_check_timeout_seconds {
            return Some(time::Duration::from_secs(timeout.into()));
        }

        None
    }

    pub fn get_request_timeout(&self) -> Option<time::Duration> {
        if let Some(timeout) = self.health_check_timeout_seconds {
            return Some(time::Duration::from_secs(timeout.into()));
        }

        None
    }

    pub fn peers(&self) -> Vec<Peer> {
        let mut peers = Vec::with_capacity(self.peers.len());
        for option in self.peers.as_slice() {
            match Peer::from_config(option, self) {
                Ok(peer) => {
                    peers.push(peer);
                }
                Err(e) => {
                    panic!("Failed to create Peer from config {:?}", e)
                }
            }
        }

        peers
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkTarget {
    Url(url::Url),
    SocketAddr(std::net::SocketAddr),
}

impl NetworkTarget {
    pub fn as_string(&self) -> String {
        match self {
            Self::SocketAddr(addr) => addr.to_string(),
            Self::Url(url) => url.to_string(),
        }
    }

    pub fn to_socket_addrs(&self) -> Option<SocketAddr> {
        match self {
            Self::SocketAddr(addr) => Some(*addr),
            Self::Url(url) => url
                .socket_addrs(|| url.port_or_known_default())
                .ok()?
                .into_iter()
                .next(),
        }
    }

    /// Appends a path segment to the NetworkTarget.
    ///
    /// This operation is only valid for the `Url` variant and will silently return if performed
    /// on a SocketAddr
    ///
    ///
    /// # Example
    /// ```
    /// let mut target = NetworkTarget::from_str("http://example.com").unwrap();
    /// target.push("api/v1/users").unwrap();
    /// assert_eq!(target.as_string(), "http://example.com/api/v1/users");
    /// ```
    pub fn push(&mut self, path: &str) -> Result<(), NetworkTargetError> {
        // TODO: fought the borrow checker for awhile on this one. This shouldn't harm performance much
        // unless you're constantly pushing paths during runtime.
        let str = self.clone().as_string();
        match self {
            Self::Url(url) => match url.path_segments_mut() {
                Ok(mut segments) => {
                    segments.push(path);
                    Ok(())
                }

                Err(_) => Err(NetworkTargetError::InvalidUrlBase(str)),
            },
            Self::SocketAddr(_) => Err(NetworkTargetError::PushToSocketAddr),
        }
    }
}

impl FromStr for NetworkTarget {
    type Err = NetworkTargetError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Url::parse(s)
            .map(NetworkTarget::Url)
            .or_else(|_| s.parse::<SocketAddr>().map(NetworkTarget::SocketAddr))
            .map_err(|_| NetworkTargetError::InvalidTargetError(s.to_owned()))
    }
}

impl<'de> Deserialize<'de> for NetworkTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;

        s.parse().map_err(D::Error::custom)
    }
}

impl Hash for NetworkTarget {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            NetworkTarget::SocketAddr(socket_addr) => socket_addr.hash(state),
            NetworkTarget::Url(url) => url.hash(state),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct PeerConfig {
    address: NetworkTarget,
    weight: Option<u32>,
    coordinates: Option<geo::Coord>,
}

impl PeerConfig {
    pub fn get_addr(&self) -> NetworkTarget {
        self.address.clone()
    }

    pub fn get_weight(&self) -> Option<u32> {
        self.weight
    }

    pub fn get_coordinates(&self) -> Option<geo::Coord> {
        self.coordinates
    }
}

#[derive(Debug, Deserialize, Clone)]
struct LoggingPath(PathBuf);

impl LoggingPath {
    /// Returns the conventional default path for a log file for the given program name.
    ///
    /// This function uses conditional compilation to provide the correct path based on
    /// the target operating system, following platform-specific conventions.
    ///
    /// - **Linux**: `$XDG_DATA_HOME/jalb/logs/jalb.log` or falls back to `~/.local/share/jalb/logs/jalb.log`
    ///
    /// - **Windows**: C:\Users\{User}\AppData\Local\jalb\logs\jalb.log
    ///
    /// - **macOS**: /Users/{User}/Library/Logs/jalb/jalb.log
    ///
    /// Returns an `io::Error` if the user's home directory or required environment
    /// variables cannot be found.
    fn new_with_default_log_path() -> Result<Self, io::Error> {
        let mut path: PathBuf;

        const PROGRAM_NAME: &'static str = "jalb";
        // --- Windows ---
        #[cfg(target_os = "windows")]
        {
            let appdata_path = env::var("LOCALAPPDATA").map_err(|e| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Could not find LOCALAPPDATA env var: {}", e),
                )
            })?;
            path = PathBuf::from(appdata_path);
            path.push(PROGRAM_NAME);
        }

        // --- macOS ---
        #[cfg(target_os = "macos")]
        {
            use std::env;

            let home_dir = env::var("HOME").map_err(|e| {
                use std::io;

                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Could not find HOME env var: {}", e),
                )
            })?;
            path = PathBuf::from(home_dir);
            path.push("Library/Logs");
            path.push(PROGRAM_NAME);
        }

        // --- Linux (and other Unix-like systems) ---
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            let xdg_data_home = env::var("XDG_DATA_HOME");
            let home_dir = env::var("HOME");

            let base_path_str = match xdg_data_home {
                Ok(p) => p,
                Err(_) => match home_dir {
                    Ok(p) => format!("{}/.local/share", p),
                    Err(e) => {
                        return Err(io::Error::new(
                            io::ErrorKind::NotFound,
                            format!("Could not find HOME env var: {}", e),
                        ));
                    }
                },
            };

            path = PathBuf::from(base_path_str);
            path.push(PROGRAM_NAME);
        }

        let log_dir = path.join("logs");
        fs::create_dir_all(&log_dir)?;

        let log_file_name = format!("{}.log", PROGRAM_NAME);

        let log_file = log_dir.join(log_file_name);

        Ok(Self(log_file))
    }
}

impl Default for LoggingPath {
    fn default() -> Self {
        Self::new_with_default_log_path()
        .map_err(|e| {
            log::error!("failed to create logfile at default path {:?}", e)
        })
        .unwrap()
    }
}

#[derive(Debug, Deserialize)]
struct LoggingConfig {
    log_level: Option<log::Level>,
    rotate_logs: bool,
    log_capacity_mb: Option<usize>,
    path: Option<LoggingPath>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    loadbalancer: LoadBalancerConfig,
    logging: LoggingConfig,
    pub security: Security,
    pub backend: BackendOptions,
}

impl Config {
    pub fn load_from_file(path: &str) -> Result<Config, ConfigError> {
        let toml_str = fs::read_to_string(path)?;

        let config = toml::from_str::<Config>(&toml_str)?;

        Ok(config)
    }

    pub fn strategy(&self) -> LoadBalancerStrategy {
        self.loadbalancer.strategy
    }

    pub fn load_balancer_type(&self) -> LoadBalancerType {
        self.loadbalancer.load_balancer_type
    }

    pub fn ip(&self) -> IpAddr {
        self.loadbalancer
            .listener_address
            .unwrap_or(IpAddr::from_str("127.0.0.1").unwrap())
    }

    pub fn port(&self) -> u16 {
        self.loadbalancer.port.unwrap_or(9220)
    }

    pub fn listener_address(&self) -> std::net::SocketAddr {
        let ip = self.ip();
        let port = self.port();

        std::net::SocketAddr::new(ip, port)
    }

    pub fn rotate_logs(&self) -> bool {
        self.logging.rotate_logs
    }

    pub fn log_file_max_size(&self) -> usize {
        const BYTES_PER_MEGABYTE: usize = 1024 * 1024;

        if let Some(max_size) = self.logging.log_capacity_mb {
            println!("max size req: {}", max_size);
            return max_size * BYTES_PER_MEGABYTE;
        }

        LOG_FILE_SIZE_HARD_LIMIT_MB * BYTES_PER_MEGABYTE
    }

    pub fn logfile_path(&self) -> LoggingPath {
        self.logging.path.clone().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_should_load_from_file() -> Result<(), ConfigError> {
        let config = Config::load_from_file("jalb.toml")?;
        config.load_balancer_type();
        assert!(config.rotate_logs() == true);
        assert!(config.log_file_max_size() == 10485760);
        let ip = config.ip();
        assert!(ip.is_ipv4());
        assert!(ip.to_string() == "127.0.0.1");

        Ok(())
    }
}

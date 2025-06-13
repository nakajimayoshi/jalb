use crate::LoadBalancerError;
use clap::error;
use core::time;
use geo::Coord;
use log;
use log::error;
use serde::Deserialize;
use std::fs;
use std::io;
use std::net::Ipv4Addr;
use std::net::{AddrParseError, IpAddr};
use std::str::FromStr;
use tokio::net::TcpSocket;
use tokio::net::TcpStream;
use tokio::time::timeout;
use toml;
use url::Url;

const VALID_VERSION_STRS: [&str; 1] = ["1"];

const LOG_FILE_SIZE_HARD_LIMIT_MB: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum JalbConfigError {
    #[error("could not open config file")]
    FileIOError(#[from] io::Error),
    #[error("failed to deserialize from config file")]
    DeserializationError(#[from] toml::de::Error),
    #[error("the provided address was neither a valid url or socket address {0}")]
    InvalidNetworkTarget(String),
    #[error("unknown load balancer strategy specified {0}")]
    UnknownLoadBalancerStrategy(String),
    #[error("unknown jalb config version specified {0}. Valid versions are {1}")]
    UnknownConfigVersion(String, String),
}

#[derive(Debug, Deserialize)]
enum JalbConfigVersion {
    #[serde(rename = "1")]
    V1,
}

impl Default for JalbConfigVersion {
    fn default() -> Self {
        JalbConfigVersion::V1
    }
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
struct LoadBalancerConfig {
    #[serde(rename = "type")]
    load_balancer_type: LoadBalancerType,
    strategy: LoadBalancerStrategy,
    listener_address: Option<IpAddr>,
    port: Option<u16>,
    max_connections: u32,
    max_requests_per_connection: u32,
}

#[derive(Debug, Deserialize)]
struct SecurityConfig {
    ip_whitelist: Vec<String>,
    ip_blacklist: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct BackendOptions {
    pub name: String,
    pub health_endpoint: Option<String>,
    pub health_check_interval_seconds: Option<u32>,
    pub health_check_timeout_seconds: Option<u32>,
    pub failed_request_threshold: Option<u32>,
    pub request_timeout_seconds: Option<u32>,
    pub rate_limit: Option<u64>,
    pub nodes: Vec<Node>,
}

#[derive(Debug, Deserialize, Clone)]
enum NetworkTarget {
    Url(url::Url),
    SocketAddr(std::net::SocketAddr),
}

impl NetworkTarget {
    fn as_string(&self) -> String {
        match self {
            Self::SocketAddr(addr) => addr.to_string(),
            Self::Url(url) => url.to_string(),
        }
    }
}

impl FromStr for NetworkTarget {
    type Err = JalbConfigError;
    fn from_str(s: &str) -> Result<Self, JalbConfigError> {
        type Err = JalbConfigError;

        if let Ok(url) = Url::parse(s) {
            return Ok(NetworkTarget::Url(url));
        }

        if let Ok(socket_addr) = s.parse::<std::net::SocketAddr>() {
            return Ok(Self::SocketAddr(socket_addr));
        }

        Err(JalbConfigError::InvalidNetworkTarget(s.to_string()))
    }
}

#[derive(Debug, Deserialize)]
pub struct Node {
    address: NetworkTarget,
    weight: Option<u32>,
    coordinates: Option<[f32; 2]>,
}

fn tcpsocket_from_address(addr: &std::net::SocketAddr) -> Result<TcpSocket, io::Error> {
    if addr.is_ipv4() {
        return TcpSocket::new_v4();
    }

    return TcpSocket::new_v6();
}

impl Node {
    async fn get_health_tcp(&self, connect_timeout: time::Duration) -> Result<bool, io::Error> {
        match self.address {
            NetworkTarget::SocketAddr(socket_addr) => {
                let socket = tcpsocket_from_address(&socket_addr)?;

                let future = socket.connect(socket_addr);
                match timeout(connect_timeout, future).await {
                    Ok(Ok(stream)) => return Ok(true),
                    Ok(Err(e)) => {
                        error!("health check for {} failed: {}", socket_addr, e);
                        return Err(e);
                    }
                    Err(_) => {
                        error!(
                            "tcp health check for {} timed out after {:?}",
                            socket_addr, connect_timeout
                        );
                        return Ok(false);
                    }
                }
            }
            NetworkTarget::Url(ref url) => {
                let future = TcpStream::connect(url.to_string());
                match timeout(connect_timeout, future).await {
                    Ok(Ok(stream)) => return Ok(true),
                    Ok(Err(e)) => {
                        error!("health check for {} failed: {}", url.as_str(), e);
                        return Err(e);
                    }
                    Err(_) => {
                        error!(
                            "tcp health check for {} timed out after {:?}",
                            url.as_str(),
                            connect_timeout
                        );
                        return Ok(false);
                    }
                }
            }
        }

        Ok(false)
    }
}

impl Node {
    pub fn get_addr(&self) -> NetworkTarget {
        self.address.clone()
    }

    pub fn get_weight(&self) -> Option<u32> {
        self.weight
    }

    pub fn get_coordinates(&self) -> Option<Coord<f32>> {
        if let Some(coordinates) = self.coordinates {
            return Some(Coord {
                x: coordinates[0],
                y: coordinates[1],
            });
        }

        None
    }
}

#[derive(Debug, Deserialize)]
struct LoggingConfig {
    log_level: Option<log::Level>,
    rotate_logs: bool,
    log_capacity_mb: Option<usize>,
    path: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let default_logfile_path = "./jalb_log.txt";

        println!("DEFAULT INVOKED!");
        Self {
            log_level: Some(log::Level::Error),
            log_capacity_mb: Some(LOG_FILE_SIZE_HARD_LIMIT_MB * 1024 * 1024),
            rotate_logs: true,
            path: default_logfile_path.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct JalbConfig {
    loadbalancer: LoadBalancerConfig,
    #[serde(default)]
    logging: LoggingConfig,
    security: SecurityConfig,
    backends: Vec<BackendOptions>,
}

impl JalbConfig {
    pub fn load_from_config_file(path: &str) -> Result<JalbConfig, JalbConfigError> {
        let toml_str = fs::read_to_string(path)?;

        let mut config = toml::from_str::<JalbConfig>(&toml_str)?;

        println!("{:#?}", config);

        Ok(config)
    }

    pub fn strategy(&self) -> LoadBalancerStrategy {
        self.loadbalancer.strategy
    }

    pub fn load_balancer_type(&self) -> LoadBalancerType {
        self.loadbalancer.load_balancer_type
    }

    pub fn listener_address(&self) -> IpAddr {
        self.loadbalancer
            .listener_address
            .unwrap_or(IpAddr::from_str("127.0.0.1").unwrap())
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
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_should_load_from_file() -> Result<(), JalbConfigError> {
        let config = JalbConfig::load_from_config_file("jalb.toml")?;
        config.load_balancer_type();
        assert!(config.rotate_logs() == true);
        assert!(config.log_file_max_size() == 10485760);
        let ip = config.listener_address();
        assert!(ip.is_ipv4());
        assert!(ip.to_string() == "127.0.0.1");

        Ok(())
    }
}

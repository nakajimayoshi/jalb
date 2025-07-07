use crate::peer::{HashableCoord, Peer};
use log;
use serde::Deserialize;
use std::fs;
use std::hash::{Hash, Hasher};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time;
use toml;
use url::Url;

use crate::errors::{CoordinateError, JalbConfigError, NetworkTargetError};
use crate::security::Security;

const VALID_VERSION_STRS: [&str; 1] = ["1"];

const LOG_FILE_SIZE_HARD_LIMIT_MB: usize = 10;

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

#[derive(Debug, Deserialize, Clone)]
pub struct BackendOptions {
    pub name: String,
    health_endpoint: Option<NetworkTarget>,
    health_check_interval_seconds: Option<u32>,
    health_check_timeout_seconds: Option<u32>,
    pub failed_request_threshold: Option<u32>,
    request_timeout_seconds: Option<u32>,
    pub rate_limit: Option<u64>,
    pub node_options: Vec<NodeOptions>,
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

    pub fn nodes(&self) -> Vec<Peer> {
        let mut nodes = Vec::new();
        for option in self.node_options.as_slice() {
            nodes.push(Peer::from_config(&option));
        }

        nodes
    }

    pub fn get_health_endpoint(&self) -> Option<NetworkTarget> {
        self.health_endpoint.clone()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
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

    pub fn to_socket_addrs(&self) -> Option<&SocketAddr> {
        match self {
            Self::SocketAddr(addr) => Some(addr),
            Self::Url(url) => {
                if let Some(port) = url.port_or_known_default() {}

                None
            }
        }
    }
}

impl FromStr for NetworkTarget {
    type Err = NetworkTargetError;
    fn from_str(s: &str) -> Result<Self, NetworkTargetError> {
        type Err = JalbConfigError;

        if let Ok(url) = Url::parse(s) {
            return Ok(NetworkTarget::Url(url));
        }

        if let Ok(socket_addr) = s.parse::<std::net::SocketAddr>() {
            return Ok(Self::SocketAddr(socket_addr));
        }

        Err(NetworkTargetError::InvalidTargetError(s.to_string()))
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
pub struct NodeOptions {
    address: NetworkTarget,
    weight: Option<u32>,
    coordinates: Option<[f32; 2]>,
}

impl NodeOptions {
    pub fn get_addr(&self) -> NetworkTarget {
        self.address.clone()
    }

    pub fn get_weight(&self) -> Option<u32> {
        self.weight
    }

    pub fn get_coordinates(&self) -> Option<HashableCoord> {
        if let Some(coordinates) = self.coordinates {
            if let Ok(coord) = HashableCoord::new(coordinates[0], coordinates[1]) {
                return Some(coord);
            }

            return None;
        }

        None
    }
}

impl Hash for NodeOptions {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.address.hash(state);
    }
}

impl PartialEq for NodeOptions {
    fn eq(&self, other: &Self) -> bool {
        self.address.as_string() == other.address.as_string()
    }

    fn ne(&self, other: &Self) -> bool {
        self.address.as_string() != other.address.as_string()
    }
}

impl Eq for NodeOptions {}

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
    security: Security,
    backends: Vec<BackendOptions>,
}

impl JalbConfig {
    pub fn load_from_file(path: &str) -> Result<JalbConfig, JalbConfigError> {
        let toml_str = fs::read_to_string(path)?;

        let config = toml::from_str::<JalbConfig>(&toml_str)?;

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

    pub fn security(&self) -> Security {
        self.security.clone()
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_should_load_from_file() -> Result<(), JalbConfigError> {
        let config = JalbConfig::load_from_file("jalb.toml")?;
        config.load_balancer_type();
        assert!(config.rotate_logs() == true);
        assert!(config.log_file_max_size() == 10485760);
        let ip = config.ip();
        assert!(ip.is_ipv4());
        assert!(ip.to_string() == "127.0.0.1");

        Ok(())
    }
}

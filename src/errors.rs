use std::{error, io, net::AddrParseError};
use thiserror;
use toml;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("could not open config file")]
    IOError(#[from] io::Error),
    #[error("failed to deserialize from config file")]
    DeserializationError(#[from] toml::de::Error),
    #[error("unknown load balancer strategy specified {0}")]
    InvalidStrategy(String),
    #[error("unknown jalb config version specified {0}. Valid versions are {1}")]
    InvalidVersion(String, String),
}

#[derive(Debug, thiserror::Error)]
pub enum LoadBalancerError {
    #[error("failed to connect to backend")]
    IOError(#[from] io::Error),
    #[error("failed to open tcp socket")]
    SocketOpenError(String),
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkTargetError {
    #[error("The provided string cannot be parsed as either a url or socket address {0}")]
    InvalidTargetError(String),
    #[error("The existing URL cannot be a base for a path {0}")]
    InvalidUrlBase(String),
    #[error("You cannot push paths to a socket addr")]
    PushToSocketAddr,
}

#[derive(Debug, thiserror::Error)]
pub enum PeerError {
    #[error("The provided health endpoint cannot be represented as a Url or socket address")]
    InvalidHealthEndpointError(NetworkTargetError),
}

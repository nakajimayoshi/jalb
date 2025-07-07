use std::io;
use thiserror;
use toml;

#[derive(Debug, thiserror::Error)]
pub enum JalbConfigError {
    #[error("could not open config file")]
    FileIOError(#[from] io::Error),
    #[error("failed to deserialize from config file")]
    DeserializationError(#[from] toml::de::Error),
    #[error("unknown load balancer strategy specified {0}")]
    UnknownLoadBalancerStrategy(String),
    #[error("unknown jalb config version specified {0}. Valid versions are {1}")]
    UnknownConfigVersion(String, String),
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
}

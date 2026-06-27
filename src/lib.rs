//! # jalb — **J**ust **A**nother **L**oad **B**alancer
//!
//! `jalb` is a configurable TCP proxy and load balancer that sits between your
//! clients and your backend services. It can be driven entirely from a TOML
//! configuration file or assembled programmatically from the building blocks
//! exposed by this crate.
//!
//! > ⚠️ This library is in very early development. The API is unstable and it is
//! > not yet suitable for production use.
//!
//! ## Quick start (from a config file)
//!
//! ```no_run
//! use jalb::{Config, NetworkLoadBalancer};
//! use tokio::net::TcpListener;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cfg = Config::load_from_file("jalb.toml")?;
//!     let listener = TcpListener::bind(cfg.listener_address()).await?;
//!
//!     let mut lb = NetworkLoadBalancer::from_config(&cfg);
//!     lb.run_forever(listener).await;
//!     Ok(())
//! }
//! ```
//!
//! ## Building blocks
//!
//! - [`Config`] — the parsed `jalb.toml` configuration.
//! - [`NetworkLoadBalancer`] — the layer-4 TCP load balancer / proxy.
//! - [`Selector`] / [`RoundRobin`] — peer selection strategies.
//! - [`Peer`] — an upstream backend instance.
//! - [`Backend`] — health-check and rate-limit settings for an upstream pool.
//! - [`Security`] — IP allow/deny lists.

pub mod backend;
pub mod config;
pub mod errors;
pub mod load_balancer;
pub mod peer;
pub mod security;
pub mod selector;

pub use backend::Backend;
pub use config::{
    BackendOptions, Config, JalbConfigVersion, LoadBalancerStrategy, LoadBalancerType,
    NetworkTarget, PeerConfig,
};
pub use errors::{ConfigError, LoadBalancerError, NetworkTargetError, PeerError};
pub use load_balancer::{NetworkLoadBalancer, TcpProxy};
pub use peer::Peer;
pub use security::Security;
pub use selector::{RoundRobin, Selector};

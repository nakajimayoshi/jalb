use std::{collections::HashSet, fmt::Debug, io, net::{IpAddr, ToSocketAddrs}, path::Path, sync::Arc};

use serde::Deserialize;
use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream},
};

use config::{JalbConfig, LoadBalancerStrategy};

use clap::{self, builder::Str, error, Parser};

use log::{error, info};
use url::Url;

use crate::config::{BackendOptions, JalbConfigError, Node};

mod config;

// make a load balancer with the following requirements:
// 1. Multi-strategy (e.g. Round Robin, Least Connections, Weighted Round Robin, Geo-based, etc.)
// 2. Secure. No taking arbitrary strings as input. Protection against Ddos with optional rate-limiting, IP whitelisting.
// 3. two varieties: stateful, and stateless
// 4. Configurable via toml file or cli args
// 5. FFFFFFFAST ZOOOM
// 6. Built-in monitoring & analytics

struct Source {
    ip_addr: IpAddr,
}

impl Source {
    pub fn new(ip_addr: IpAddr) -> Source {
        Self {
            ip_addr,
        }
    }
}

struct LoadBalancerBuilder {
    // backends: Vec<Backend>,
    // selector: dyn Selector,
}

impl LoadBalancerBuilder {
    
}

#[derive(Debug, thiserror::Error)]
enum LoadBalancerError {
    #[error("failed to connect to backend")]
    IOError(#[from] io::Error),
    #[error("failed to open tcp socket")]
    SocketOpenError(String)
}



pub trait LoadBalancer {
    fn select_addr(&mut self) -> Option<&url::Url>;
    fn add_backend(&mut self, config: &BackendOptions) -> Result<(), LoadBalancerError>;
}

pub struct NetworkLoadBalancerOptions {
    ip_blacklist: Option<Vec<IpAddr>>,
    ip_whitelist: Option<Vec<IpAddr>>,
}



pub struct Backend {
    options: BackendOptions
}

impl Backend {
    pub fn new(options: BackendOptions) -> Self {
        Self { options: options }
    }


}


#[derive(Debug)]
struct RoundRobin {
    last_idx: usize,
    pool: HashSet<url::Url> 
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self {
            last_idx: 0,
            pool: HashSet::new(),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn select_addr(&mut self) -> Option<&url::Url> {
        if self.pool.is_empty() {
            return None;
        }

        return Some(url::Url)
    }

    fn add_backend(&mut self, config: &BackendOptions) -> Result<(), JalbConfigError> {
       
       
        self.pool.insert()
    }
}


struct NetworkLoadBalancer<T: LoadBalancer> {
    inner: T
}

impl<T: LoadBalancer> NetworkLoadBalancer<T> {
}


#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    listener_addr: String,

    #[arg(long)]
    port: u16,

    #[arg(long)]
    worker_threads: usize, // log_level: LogLevel
}


#[tokio::main]
async fn main() {
    println!("hello world");
}
use std::{fmt::Debug, io, net::IpAddr, sync::Arc};

use tokio::{
    self,
    io::{AsyncReadExt, AsyncWriteExt},
};

use backend::Backend;
use clap::{self, Parser, builder::Str};
use isocountry::CountryCode;
use selectors::{RoundRobinSelector, Selector};

use log::{error, info};

mod backend;
mod config;
mod selectors;
mod tests;

// make a load balancer with the following requirements:
// 1. Multi-strategy (e.g. Round Robin, Least Connections, Weighted Round Robin, Geo-based, etc.)
// 2. Secure. No taking arbitrary strings as input. Protection against Ddos with optional rate-limiting, IP whitelisting.
// 3. two varieties: stateful, and stateless
// 4. Configurable via toml file or cli args
// 5. FFFFFFFAST ZOOOM
// 6. Built-in monitoring & analytics

enum Region {
    Asia(CountryCode),
    Africa(CountryCode),
    Europe(CountryCode),
    Oceania(CountryCode),
    NorthAmerica(CountryCode),
    SouthAmerica(CountryCode),
    Unknown,
}

struct Source {
    ip_addr: IpAddr,
    region: Option<Region>,
}

impl Source {
    pub fn new(ip_addr: IpAddr) -> Source {
        Self {
            ip_addr,
            region: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Strategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    Geospatial,
}

#[derive(Debug, Clone)]
struct LoadBalancer<T>
where
    T: Clone + std::fmt::Debug + Send,
    T: selectors::Selector,
{
    services: Vec<Backend>,
    whitelist: Option<Vec<IpAddr>>,
    blacklist: Option<Vec<IpAddr>>,
    selector: T,
    session_requests: u64,
}

impl<T> LoadBalancer<T>
where
    T: Clone + std::fmt::Debug + Send + Default,
    T: Selector,
{
    pub fn new() -> LoadBalancer<T> {
        Self {
            services: vec![],
            whitelist: None,
            blacklist: None,
            selector: T::default(),
            session_requests: 0,
        }
    }

    pub fn with_backends(services: Vec<Backend>) -> LoadBalancer<T> {
        Self {
            services,
            whitelist: None,
            blacklist: None,
            selector: T::default(),
            session_requests: 0,
        }
    }

    fn increment_session_requests(&mut self) {
        self.session_requests += 1
    }

    pub fn get_session_requests(&self) -> u64 {
        self.session_requests
    }

    fn select_backend(&mut self) -> usize {
        self.selector.select_service(&self.services)
    }

    pub async fn serve_request(
        &mut self,
        client_stream: &mut tokio::net::TcpStream,
    ) -> Result<(), io::Error> {
        if let Ok(addr) = client_stream.peer_addr() {
            let client_ip = addr.ip();

            if let Some(blacklist) = &self.blacklist {
                if blacklist.contains(&client_ip) {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "IP is blacklisted",
                    ));
                }
            }

            // Check if whitelist is enabled and IP is not in whitelist
            if let Some(whitelist) = &self.whitelist {
                if !whitelist.is_empty() && !whitelist.contains(&client_ip) {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "IP is not in whitelist",
                    ));
                }
            }
        }

        let backend_idx = self.select_backend();
        let backend = self
            .services
            .get(backend_idx)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No backend available"))?;

        let mut server_stream = tokio::net::TcpStream::connect(&backend.uri.to_string()).await?;

        // Set reasonable timeouts
        // client_stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        // client_stream.set_write_timeout(Some(Duration::from_secs(10)))?;
        // server_stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        // server_stream.set_write_timeout(Some(Duration::from_secs(10)))?;

        // Buffer for data transfer
        let mut buffer = [0; 8192]; // 8KB buffer

        let mut client_closed = false;
        let mut server_closed = false;

        // Proxy loop
        while !client_closed && !server_closed {
            // Client -> Server
            if !client_closed {
                match client_stream.read(&mut buffer).await {
                    Ok(0) => {
                        // Client closed connection
                        client_closed = true;
                        server_stream.shutdown().await?;
                    }
                    Ok(n) => {
                        // Forward data to server
                        match server_stream.write_all(&buffer[0..n]).await {
                            Ok(_) => {
                                // Successful write to server
                                self.increment_session_requests();
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                continue;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // No data available, continue
                    }
                    Err(e) => return Err(e),
                }
            }

            // Server -> Client
            if !server_closed {
                match server_stream.read(&mut buffer).await {
                    Ok(0) => {
                        // Server closed connection
                        server_closed = true;
                        client_stream.shutdown().await?;
                    }
                    Ok(n) => {
                        // Forward data to client
                        match client_stream.write_all(&buffer[0..n]).await {
                            Ok(_) => {
                                // Successful write to client
                            }
                            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                                // Would block, try again later
                                continue;
                            }
                            Err(e) => return Err(e),
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        // No data available, continue
                    }
                    Err(e) => return Err(e),
                }
            }

            // Small sleep to avoid CPU spin
            // std::thread::sleep(Duration::from_millis(1));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
enum LogLevel {
    Error,
    Warning,
    Debug,
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

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<(), io::Error> {
    let args = Args::parse();
    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", args.listener_addr, args.port)).await?;

    let listener_addr = listener.local_addr()?;

    info!("Jalb balancer is listening on {:?}", listener_addr);

    let backends = vec![
        Backend::new("127.0.0.1:3001").unwrap(),
        Backend::new("127.0.0.1:3002").unwrap(),
        Backend::new("127.0.0.1:3003").unwrap(),
        Backend::new("127.0.0.1:3004").unwrap(),
        Backend::new("127.0.0.1:3005").unwrap(),
        Backend::new("127.0.0.1:3006").unwrap(),
        Backend::new("127.0.0.1:3007").unwrap(),
        Backend::new("127.0.0.1:3008").unwrap(),
        Backend::new("127.0.0.1:3009").unwrap(),
        Backend::new("127.0.0.1:3010").unwrap(),
    ];

    let balancer = LoadBalancer::<RoundRobinSelector>::with_backends(backends);
    let balancer = Arc::new(tokio::sync::Mutex::new(balancer));

    while let Ok((mut stream, _addr)) = listener.accept().await {
        let balancer_clone = Arc::clone(&balancer);
        tokio::spawn(async move {
            let mut balancer = balancer_clone.lock().await;
            balancer.serve_request(&mut stream).await.unwrap();
        });
    }

    Ok(())
}

use crate::metrics::{ClientMetrics, MetricsEngine, PrometheusMetrics, ServerMetrics};
use crate::{
    backend::Backend,
    config::{Config, LoadBalancerStrategy},
    peer::{Peer, tcpsocket_from_address},
    security::Security,
    selector::{RoundRobin, Selector},
};
use log::error;
use std::time::UNIX_EPOCH;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::error::Error;
use http::HeaderMap;
use tokio::{
    io::{self, copy_bidirectional},
    net::TcpStream,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub trait TcpProxy {
    async fn proxy_connection(
        incoming: TcpStream,
        upstream: std::net::SocketAddr,
    ) -> Result<(), io::Error>;
}



pub struct NetworkRequest {
    pub address: std::net::SocketAddr,
    pub upstream: TcpStream,
    pub received: Instant,
}

impl NetworkRequest {
    pub fn new(client_address: std::net::SocketAddr, upstream: TcpStream) -> Self {
        Self {
            address: client_address,
            upstream,
            received: Instant::now(),
        }
    }
}

pub struct NetworkLoadBalancer {
    pub security: Security,
    selector: Box<dyn Selector>,
    metrics_enabled: bool, // TODO: I don't like this
    metrics_engine: Option<Arc<dyn MetricsEngine>>,
}

impl NetworkLoadBalancer {
    pub(crate) fn new_from_config(cfg: &Config) -> Self {
        let mut selector = match cfg.strategy() {
            LoadBalancerStrategy::RoundRobin => RoundRobin::new(),
            LoadBalancerStrategy::WeightedAverage => todo!(),
            LoadBalancerStrategy::LeastUsed => todo!(),
            LoadBalancerStrategy::Geolocation => todo!(),
            _ => todo!(),
        };

        cfg.backend.peers().drain(0..).for_each(|p| {
            selector.add_peer(p);
        });

        Self {
            security: cfg.security.to_owned(),
            selector: Box::new(selector),
            metrics_enabled: cfg.metrics_enabled(),
            metrics_engine: None,
        }
    }

    fn is_allowed(&self, ip: &IpAddr) -> bool {
        !self.security.is_blacklisted(ip) && self.security.is_whitelisted(ip)
    }

    fn listener_task(&mut self, request: NetworkRequest) {
        let ip = request.address.ip();
        if !self.is_allowed(&ip) {
            if let Some(metrics) = &self.metrics_engine {
                let metrics_instance = Arc::clone(&metrics);
                match metrics_instance
                    .put_client_metrics(ClientMetrics {
                        client_address: request.address.clone(),
                        is_blocked: true,
                        served_at: UNIX_EPOCH.elapsed().unwrap().as_millis() as u64,
                        request_dropped: false,
                        response_time: request.received.elapsed(),
                    }) {
                    Err(e) => {
                        error!("error collecting metrics {:?}", e);
                    }
                    Ok(_) => {}
                }
            }
            return;
        }

        if let Some(peer) = self.selector.next() {
            let metrics_engine: Option<Arc<dyn MetricsEngine>> = match &self.metrics_engine {
                Some(engine) => Some(Arc::clone(engine)),
                None => None,
            };

            tokio::spawn(async move {
                let socket_addr = peer
                    .address
                    .to_socket_addrs()
                    .expect("peer does not contain valid socket address");

                match NetworkLoadBalancer::proxy_connection(request.upstream, socket_addr).await {
                    Err(e) => {
                        error!("proxy error {:?}", e);
                        if let Some(metrics) = metrics_engine {
                            match metrics.put_server_metrics(ServerMetrics {
                                upstream_address: socket_addr,
                                response_time: request.received.elapsed(),
                                dropped_request: true,
                            }) {
                                Err(e) => {
                                    error!("error collecting metrics {:?}", e);
                                }
                                Ok(_) => {}
                            }
                        }
                    }
                    _ => {}
                }
            });
        }
    }

    fn setup_metrics(&mut self) -> Result<(), Box<dyn Error>> {
        let metrics_engine = PrometheusMetrics::new()?; // TODO: implement metrics engine in Postgres, MySql, Sqlite, & Redis
        self.metrics_engine = Some(Arc::new(metrics_engine));
        Ok(())
    }

    async fn handle_metrics_request(
        &self,
        mut socket: tokio::net::TcpStream,
        metrics_engine: Arc<dyn MetricsEngine>,
    ) -> std::io::Result<()> {
        let mut buffer = [0u8; 1024];

        let n = socket.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..n]);

        if request.starts_with("GET /metrics") {
            let headers = request.lines().collect::<Vec<&str>>();
            let body = metrics_engine.metrics();

            let response = format!(
                "HTTP/1.1 200 OK\r\n\
             Content-Type: text/plain; version=0.0.4; charset=utf-8\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             \r\n\
             {}",
                body.len(),
                body
            );

            socket.write_all(response.as_bytes()).await?;
        } else {
            // 404 for other paths
            let response = "HTTP/1.1 404 Not Found\r\n\
                       Content-Length: 9\r\n\
                       Connection: close\r\n\
                       \r\n\
                       Not Found";
            socket.write_all(response.as_bytes()).await?;
        }

        socket.shutdown().await?;
        Ok(())
    }

    pub async fn run_forever(&mut self, listener: tokio::net::TcpListener) {
        if self.metrics_enabled {
            if let Err(e) =  self.setup_metrics() {
                error!("setup metrics error {:?}", e);
            }
        }

        while let Ok((stream, addr)) = listener.accept().await {
            let req = NetworkRequest::new(addr, stream);
            self.listener_task(req);
        }
    }

    pub async fn run_until(&mut self, listener: tokio::net::TcpListener, duration: Duration) {
        let now = Instant::now();
        while let Ok((stream, addr)) = listener.accept().await {
            if now.elapsed() > duration {
                break;
            }

            let req = NetworkRequest::new(addr, stream);
            self.listener_task(req);
        }
    }
}

impl TcpProxy for NetworkLoadBalancer {
    async fn proxy_connection(
        mut incoming: TcpStream,
        upstream: std::net::SocketAddr,
    ) -> Result<(), io::Error> {
        let socket = tcpsocket_from_address(&upstream)?;
        let mut outgoing = socket.connect(upstream).await?;

        let (_, _) = copy_bidirectional(&mut incoming, &mut outgoing).await?;

        Ok(())
    }
}

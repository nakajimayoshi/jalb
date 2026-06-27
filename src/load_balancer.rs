use std::{
    net::IpAddr,
    time::{Duration, Instant},
};
use tokio::{
    io::{self, copy_bidirectional},
    net::TcpStream,
};

use crate::{
    backend::Backend,
    config::{Config, LoadBalancerStrategy},
    peer::tcpsocket_from_address,
    security::Security,
    selector::{RoundRobin, Selector},
};

#[allow(async_fn_in_trait)]
pub trait TcpProxy {
    async fn proxy_connection(
        incoming: TcpStream,
        upstream: std::net::SocketAddr,
    ) -> Result<(), io::Error>;
}

pub struct NetworkLoadBalancer {
    pub security: Security,
    backend: Backend,
    selector: Box<dyn Selector>,
}

impl NetworkLoadBalancer {
    /// Builds a [`NetworkLoadBalancer`] from a parsed [`Config`].
    ///
    /// The configured [`LoadBalancerStrategy`] determines which [`Selector`]
    /// backs peer selection. Only [`LoadBalancerStrategy::RoundRobin`] is
    /// currently implemented; the other strategies are not yet available.
    pub fn from_config(cfg: &Config) -> Self {
        let backend = Backend::from_config(&cfg.backend);

        let mut selector = match cfg.strategy() {
            LoadBalancerStrategy::RoundRobin => RoundRobin::new(),
            LoadBalancerStrategy::WeightedAverage => todo!(),
            LoadBalancerStrategy::LeastUsed => todo!(),
            LoadBalancerStrategy::Geolocation => todo!(),
        };

        cfg.backend.peers().drain(0..).for_each(|p| {
            selector.add_peer(p);
        });

        Self {
            security: cfg.security.to_owned(),
            backend,
            selector: Box::new(selector),
        }
    }

    /// Returns the upstream backend pool settings (health checks, timeouts,
    /// rate limit) this load balancer was built with.
    pub fn backend(&self) -> &Backend {
        &self.backend
    }

    fn is_allowed(&self, ip: &IpAddr) -> bool {
        !self.security.is_blacklisted(ip) && self.security.is_whitelisted(ip)
    }

    fn listener_task(&mut self, stream: TcpStream, downstream: std::net::SocketAddr) {
        let ip = downstream.ip();

        if !self.is_allowed(&ip) {
            return;
        }

        if let Some(peer) = self.selector.next() {
            tokio::spawn(async move {
                let Some(socket_addr) = peer.address.to_socket_addrs() else {
                    log::error!(
                        "peer {} does not resolve to a socket address",
                        peer.address.as_string()
                    );
                    return;
                };

                if let Err(e) = NetworkLoadBalancer::proxy_connection(stream, socket_addr).await {
                    log::error!("error proxying connection to {socket_addr}: {e}");
                }
            });
        }
    }

    pub async fn run_forever(&mut self, listener: tokio::net::TcpListener) {
        while let Ok((stream, addr)) = listener.accept().await {
            self.listener_task(stream, addr);
        }
    }

    pub async fn run_until(&mut self, listener: tokio::net::TcpListener, duration: Duration) {
        let now = Instant::now();
        while let Ok((stream, addr)) = listener.accept().await {
            
            if now.elapsed() > duration {
                break;
            }

            self.listener_task(stream, addr);
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

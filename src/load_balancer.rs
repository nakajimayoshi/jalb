use std::{net::IpAddr, os::unix::net::SocketAddr, sync::Arc, time::{self, Duration, Instant}};
use tokio::{
    io::{self, copy_bidirectional},
    net::TcpStream,
};

use crate::{
    backend::Backend,
    config::{Config, LoadBalancerStrategy},
    peer::{Peer, tcpsocket_from_address},
    security::Security,
    selector::{RoundRobin, Selector},
};

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
    balancer_task: Option<tokio::task::JoinHandle<()>>,
}

impl NetworkLoadBalancer {
    pub(crate) fn new_from_config(cfg: &Config) -> Self {
        let backend = Backend::from_config(&cfg.backend);

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
            backend: backend,
            balancer_task: None,
            selector: Box::new(selector),
        }
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
                let socket_addr = peer 
                    .address
                    .to_socket_addrs()
                    .expect("peer does not contain valid socket address");

                match NetworkLoadBalancer::proxy_connection(stream, socket_addr).await {
                    Err(e) => {
                        println!("Error proxying {:?}", e)
                    }
                    _ => {}
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

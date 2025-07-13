use std::{io::Error, net::IpAddr, os::unix::net::SocketAddr};


pub trait TcpProxy {
    async fn proxy_connection(&self, incoming: TcpStream, upstream: std::net::SocketAddr) -> Result<(), io::Error>;
}

use tokio::{
    io::{self, copy_bidirectional},
    net::TcpStream,
    sync::oneshot,
};

use crate::{
    backend::Backend,
    config::{BackendOptions, Config, LoadBalancerStrategy},
    peer::{Peer, tcpsocket_from_address},
    security::Security,
    selector::{RoundRobin, Selector},
};

type SelectionRequest = oneshot::Sender<Option<std::net::SocketAddr>>;

pub struct NetworkLoadBalancer {
    pub security: Security,
    sender: tokio::sync::mpsc::UnboundedSender<SelectionRequest>,
    backend: Backend,
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

        let (request_tx, mut request_rx) =
            tokio::sync::mpsc::unbounded_channel::<SelectionRequest>();

        let balancer_task = tokio::spawn(async move {
            while let Some(response_tx) = request_rx.recv().await {
                let peer_addr = selector
                    .select_peer()
                    .and_then(|peer| peer.address.to_socket_addrs());

                let _ = response_tx.send(peer_addr);
            }
        });

        Self {
            security: cfg.security.to_owned(),
            backend: backend,
            sender: request_tx,
            balancer_task: Some(balancer_task),
        }
    }

    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        !self.security.is_blacklisted(ip) && self.security.is_whitelisted(ip)
    }

    pub async fn select_peer_address(&self) -> Option<std::net::SocketAddr> {
        let (response_tx, response_rx) = oneshot::channel();

        if self.sender.send(response_tx).is_ok() {
            return response_rx
                .await
                .ok()
                .expect("failed to get response from rx channel");
        }

        None
    }


}


impl TcpProxy for NetworkLoadBalancer {
    async fn proxy_connection(
        &self,
        mut incoming: TcpStream,
        upstream: std::net::SocketAddr,
    ) -> Result<(), io::Error> {

        let socket = tcpsocket_from_address(&upstream)?;
        let mut outgoing = socket.connect(upstream).await?;

        let (_, _) = copy_bidirectional(&mut incoming, &mut outgoing).await?;

        Ok(())
    }
}
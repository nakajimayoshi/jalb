use std::{io::Error, net::IpAddr};

use tokio::{io::{self, copy_bidirectional}, net::TcpStream};

use crate::{
    backend::Backend, config::{BackendOptions, Config, LoadBalancerStrategy}, peer::tcpsocket_from_address, security::Security, selector::{RoundRobin, Selector}
};

pub struct NetworkLoadBalancer {
    security: Security,
    selector: Box<dyn Selector>,
    backend: Backend,
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
            selector: Box::new(selector),
        }
    }

    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        !self.security.is_blacklisted(ip) && self.security.is_whitelisted(ip)
    }

    pub async fn proxy_tcp(&mut self, incoming: &mut TcpStream) -> Result<(), io::Error> {

        let peer = match self.selector.select_peer() {
            Some(peer) => peer,
            None => {
                return Err(Error::new(io::ErrorKind::Other, "failed to select peer"))
            }
        };

        println!("selected peer: {:?}", peer.address);

        let outgoing_addr = match peer.address.to_socket_addrs() {
            Some(addr) => addr,
            None => {
                return Err(Error::new(io::ErrorKind::Unsupported, "the peer does not have a supported L4 address"))
            }
        };

        let socket = tcpsocket_from_address(&outgoing_addr)?;
        let mut outgoing = socket.connect(outgoing_addr).await?;

        let (_, _) = copy_bidirectional(incoming, &mut outgoing).await?;
     
        Ok(())
    }
}

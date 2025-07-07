use crate::{
    config::{NetworkTarget, NodeOptions},
    errors::NetworkTargetError,
};
use geo::Coord;
use log::error;
use ordered_float::OrderedFloat;
use std::{io, str::FromStr, time::Duration};
use tokio::{
    net::{TcpSocket, TcpStream},
    time::timeout,
};

use crate::errors::CoordinateError;

fn tcpsocket_from_address(addr: &std::net::SocketAddr) -> Result<TcpSocket, io::Error> {
    if addr.is_ipv4() {
        return tokio::net::TcpSocket::new_v4();
    }

    TcpSocket::new_v6()
}

#[derive(Debug, Hash)]
pub struct HashableCoord {
    inner: Coord<OrderedFloat<f32>>,
}

impl HashableCoord {
    pub fn new(latitude: f32, longitude: f32) -> Result<Self, CoordinateError> {
        if latitude.abs() > 90.0 {
            return Err(CoordinateError::InvalidLatitude(latitude));
        }

        if longitude.abs() > 180.0 {
            return Err(CoordinateError::InvalidLongitude(longitude));
        }

        Ok(Self {
            inner: Coord {
                x: OrderedFloat(latitude),
                y: OrderedFloat(longitude),
            },
        })
    }
}

#[derive(Debug)]
pub struct Peer {
    pub healthy: bool,
    pub address: NetworkTarget,
    pub weight: u32,
    pub coordinates: Option<HashableCoord>,
}

impl Peer {
    pub fn new(addr: &str) -> Result<Self, NetworkTargetError> {
        let target = NetworkTarget::from_str(addr)?;

        Ok(Self {
            healthy: false,
            address: target,
            weight: 1,
            coordinates: None,
        })
    }

    pub(crate) fn from_config(options: &NodeOptions) -> Self {
        Self {
            healthy: false,
            address: options.get_addr(),
            weight: options.get_weight().unwrap_or(1),
            coordinates: options.get_coordinates(),
        }
    }

    pub async fn health_check(&mut self, connect_timeout: Duration) -> Result<bool, io::Error> {
        match self.address {
            NetworkTarget::SocketAddr(socket_addr) => {
                let socket = tcpsocket_from_address(&socket_addr)?;
                let future = socket.connect(socket_addr);
                match timeout(connect_timeout, future).await {
                    Ok(Ok(_stream)) => {
                        self.healthy = true;
                        Ok(true)
                    }
                    Ok(Err(e)) => {
                        error!("health check for {} failed: {}", socket_addr, e);
                        self.healthy = false;
                        Err(e)
                    }
                    Err(_) => {
                        error!(
                            "tcp health check for {} timed out after {:?}",
                            socket_addr, connect_timeout
                        );
                        self.healthy = false;
                        Ok(false)
                    }
                }
            }
            NetworkTarget::Url(ref url) => {
                if url.port_or_known_default().is_none() {
                    let error = io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "url has no associated port. Network balancers require all urls to have a port",
                    );
                    return Err(error);
                }

                let future = TcpStream::connect(url.as_str());
                match timeout(connect_timeout, future).await {
                    Ok(Ok(_stream)) => {
                        self.healthy = true;
                        Ok(true)
                    }
                    Ok(Err(e)) => {
                        error!("health check for {} failed: {}", url.as_str(), e);
                        self.healthy = false;
                        Err(e)
                    }
                    Err(_) => {
                        error!(
                            "tcp health check for {} timed out after {:?}",
                            url.as_str(),
                            connect_timeout
                        );
                        self.healthy = false;
                        Ok(false)
                    }
                }
            }
        }
    }
}

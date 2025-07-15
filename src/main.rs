use std::{
    io,
    sync::{
        atomic::{AtomicI32, AtomicI64, Ordering}, Arc
    },
    time::{Duration, Instant},
};

use tokio::{self, net::TcpListener};

use config::Config;

use load_balancer::NetworkLoadBalancer;

use crate::load_balancer::TcpProxy;

mod backend;
mod config;
mod errors;
mod load_balancer;
mod peer;
mod security;
mod selector;

// make a load balancer with the following requirements:
// 1. Multi-strategy (e.g. Round Robin, Least Connections, Weighted Round Robin, Geo-based, etc.)
// 2. Secure. No taking arbitrary strings as input. Protection against Ddos with optional rate-limiting, IP whitelisting/blacklisting, TLS.
// 3. Configurable via toml file or cli args
// 4. FFFFFFFAST ZOOOM
// 5. Built-in monitoring & analytics

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

pub async fn listen_and_serve(
    listener: tokio::net::TcpListener,
    load_balancer: NetworkLoadBalancer,
) -> Result<(), io::Error> {
    while let Ok((stream, addr)) = listener.accept().await {}

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load_from_file("./jalb.toml")?;
    let listener_addr = cfg.listener_address();
    let listener = TcpListener::bind(listener_addr).await?;

    let mut load_balancer = NetworkLoadBalancer::new_from_config(&cfg);

    println!(
        "load balancer listening on {}:{}",
        listener_addr.ip(),
        listener_addr.port()
    );


    load_balancer.run_forever(listener).await;

    Ok(())
}

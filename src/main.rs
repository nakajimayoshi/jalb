use std::sync::Arc;

use tokio::{self, net::TcpListener};

use config::Config;

use load_balancer::NetworkLoadBalancer;

use crate::selector::RoundRobin;

mod backend;
mod config;
mod errors;
mod load_balancer;
mod peer;
mod security;
mod selector;

// make a load balancer with the following requirements:
// 1. Multi-strategy (e.g. Round Robin, Least Connections, Weighted Round Robin, Geo-based, etc.)
// 2. Secure. No taking arbitrary strings as input. Protection against Ddos with optional rate-limiting, IP whitelisting.
// 3. two varieties: stateful, and stateless
// 4. Configurable via toml file or cli args
// 5. FFFFFFFAST ZOOOM
// 6. Built-in monitoring & analytics

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

    while let Ok((mut stream, addr)) = listener.accept().await {
        let ip = addr.ip();

        if !load_balancer.is_allowed(&ip) {
            continue;
        }

        match load_balancer.proxy_tcp(&mut stream).await {
            Ok(_) => {},
            Err(e) => {
                println!("failed to proxy peer {:?}", e)
            }
        }
    }

    Ok(())
}
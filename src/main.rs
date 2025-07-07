use tokio::{self, net::TcpListener};

use config::JalbConfig;

use load_balancer::NetworkLoadBalancer;

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
    let cfg = JalbConfig::load_from_file("./jalb.toml")?;

    let listener_addr = cfg.listener_address();
    let listener = TcpListener::bind(listener_addr).await?;

    // let load_balancer = NetworkLoadBalancer::new::<RoundRobin>(&cfg);

    // while let Ok((stream, peer)) = listener.accept().await {}

    Ok(())
}

use std::path::PathBuf;

use clap::Parser;
use tokio::net::TcpListener;

use jalb::{Config, NetworkLoadBalancer};

/// jalb — Just Another Load Balancer.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the jalb TOML configuration file.
    #[arg(short, long, default_value = "./jalb.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let cfg = Config::load_from_file(&args.config.to_string_lossy())?;
    let listener_addr = cfg.listener_address();
    let listener = TcpListener::bind(listener_addr).await?;

    let mut load_balancer = NetworkLoadBalancer::from_config(&cfg);

    println!("load balancer listening on {listener_addr}");

    load_balancer.run_forever(listener).await;

    Ok(())
}

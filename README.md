# Jalb

- **J**ust
- **A**nother
- **L**oad
- **B**alancer

Jalb is a multithreaded, configurable, and easy-to-monitor TCP proxy and load
balancer that sits between your clients and your backend services. It can be
embedded in another Rust project as a **library**, or run directly as a small
**CLI binary** driven by a TOML config file.

> ⚠️ This project is in **very early development**. The API is unstable and it is
> not yet suitable for production use.

## Using jalb as a library

Add it to your `Cargo.toml` (via git while it is unpublished):

```toml
[dependencies]
jalb = { git = "https://github.com/nakajimayoshi/jalb" }
tokio = { version = "1", features = ["full"] }
```

Build a load balancer from a config file and run it:

```rust
use jalb::{Config, NetworkLoadBalancer};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load_from_file("jalb.toml")?;
    let listener = TcpListener::bind(cfg.listener_address()).await?;

    let mut lb = NetworkLoadBalancer::from_config(&cfg);
    lb.run_forever(listener).await;
    Ok(())
}
```

The crate re-exports its building blocks at the top level so you can assemble or
inspect a balancer programmatically:

- [`Config`] — the parsed `jalb.toml` configuration.
- [`NetworkLoadBalancer`] — the layer-4 TCP load balancer / proxy.
- [`Selector`] / [`RoundRobin`] — peer selection strategies.
- [`Peer`] — an upstream backend instance.
- [`Backend`] — health-check, timeout, and rate-limit settings for a pool.
- [`Security`] — IP allow/deny lists.

[`Config`]: https://docs.rs/jalb
[`NetworkLoadBalancer`]: https://docs.rs/jalb
[`Selector`]: https://docs.rs/jalb
[`RoundRobin`]: https://docs.rs/jalb
[`Peer`]: https://docs.rs/jalb
[`Backend`]: https://docs.rs/jalb
[`Security`]: https://docs.rs/jalb

## Running the CLI

```sh
# Uses ./jalb.toml by default
cargo run

# Or point it at a specific config file
cargo run -- --config /path/to/jalb.toml
```

## Configuration

Jalb is configured with a TOML file. See [`jalb.toml`](./jalb.toml) for a fully
worked example. The top-level sections are:

| Section          | Purpose                                                  |
| ---------------- | -------------------------------------------------------- |
| `[loadbalancer]` | Listener address/port, balancer type, selection strategy |
| `[logging]`      | Log level, rotation, capacity, and log file path         |
| `[security]`     | IP whitelist / blacklist                                 |
| `[backend]`      | Upstream pool: health checks, timeouts, peers            |

### Selection strategies

Only `round_robin` is implemented today. `least_used`, `weighted_average`, and
`geo` are recognized in config but not yet wired up.

## Development

```sh
cargo build
cargo test
cargo clippy --all-targets
```

use prometheus::{CounterVec, HistogramOpts, HistogramVec, Opts, Registry};
use prometheus::{Encoder, TextEncoder};
use std::net::{SocketAddr, TcpListener};
use std::time::Duration;

pub trait MetricsEngine: Send + Sync {
    fn put_client_metrics(&self, client_metrics: ClientMetrics) -> std::io::Result<()>;
    fn put_server_metrics(&self, server_metrics: ServerMetrics) -> std::io::Result<()>;
    fn metrics(&self) -> String;
}

#[derive(Debug, Clone, Copy)]
pub struct ClientMetrics {
    pub client_address: SocketAddr,
    pub is_blocked: bool,
    pub served_at: u64,
    pub request_dropped: bool,
    pub response_time: Duration,
}

#[derive(Debug, Clone, Copy)]
pub struct ServerMetrics {
    pub upstream_address: SocketAddr,
    pub response_time: Duration,
    pub dropped_request: bool,
}

pub struct PrometheusMetrics {
    requests_total: CounterVec,
    requests_dropped: CounterVec,
    response_time: HistogramVec,
    registry: Registry,
}


impl PrometheusMetrics {
    pub fn new() ->  Result<Self, prometheus::Error> {
        let registry = Registry::new();

        let requests_total = CounterVec::new(
            Opts::new("requests_total", "Total requests by client"),
            &["client_addr", "upstream_addr"]
        )?;

        let requests_dropped = CounterVec::new(
            Opts::new("requests_dropped_total", "Dropped requests"),
            &["client_addr", "reason"]
        )?;

        let response_time = HistogramVec::new(
            HistogramOpts::new("response_time_ms", "Response time distribution"),
            &["client_addr", "upstream_addr"]
        )?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(requests_dropped.clone()))?;
        registry.register(Box::new(response_time.clone()))?;

        Ok(Self {
            requests_total,
            requests_dropped,
            response_time,
            registry,
        })
    }
}

impl MetricsEngine for PrometheusMetrics {

    fn put_client_metrics(&self, client_metrics: ClientMetrics) -> std::io::Result<()> {
        if client_metrics.is_blocked {
            self.requests_dropped
                .with_label_values(&[client_metrics.client_address.to_string(), "blocked".to_string()])
                .inc()
        }

        if client_metrics.request_dropped {
            self.requests_dropped
                .with_label_values(&[client_metrics.client_address.to_string(), "dropped".to_string()])
                .inc()
        }

        Ok(())
    }

    fn put_server_metrics(&self, server_metrics: ServerMetrics) -> std::io::Result<()> {
        if server_metrics.dropped_request {
            self.requests_dropped
                .with_label_values(&[server_metrics.upstream_address.to_string(), "dropped".to_string()])
                .inc()
        }

        self.response_time
            .with_label_values(&[server_metrics.upstream_address.to_string(), "response_time".to_string()])
            .observe(server_metrics.response_time.as_millis() as f64);

        Ok(())
    }

    fn metrics(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();

        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();

        String::from_utf8(buffer).unwrap()
    }
}
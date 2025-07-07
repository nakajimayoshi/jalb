use crate::config::{BackendOptions, NetworkTarget};
use std::{str::FromStr, time::Duration};

#[derive(Debug)]
pub struct Backend {
    pub health_endpoint: Option<String>,
    pub health_check_interval: Option<Duration>,
    pub health_check_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
    pub failed_request_threshold: Option<u32>,
    pub rate_limit: Option<u64>,
}

impl Backend {
    pub(crate) fn from_config(config: &BackendOptions) -> Self {
        Self {
            health_endpoint: config.health_endpoint.clone(),
            health_check_interval: config.get_health_check_interval(),
            health_check_timeout: config.get_health_check_timeout(),
            request_timeout: config.get_request_timeout(),
            failed_request_threshold: config.failed_request_threshold,
            rate_limit: config.rate_limit,
        }
    }

    pub fn with_health_endpoint(mut self, endpoint: &str) -> Self {
        self.health_endpoint = Some(endpoint.to_string());
        self
    }

    pub fn with_health_check_interval(mut self, interval: Duration) -> Self {
        self.health_check_interval = Some(interval);
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    pub fn with_failed_request_threshold(mut self, threshold: u32) -> Self {
        self.failed_request_threshold = Some(threshold);
        self
    }

    pub fn with_rate_limit(mut self, max_requests_per_second: u64) -> Self {
        self.rate_limit = Some(max_requests_per_second);
        self
    }
}

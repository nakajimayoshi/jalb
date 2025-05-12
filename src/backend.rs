use http::uri::InvalidUri;

#[derive(Debug, Clone)]
pub struct Backend {
    pub uri: http::Uri,
    request_limit_per_second: u64,
    served_requests: u64,
}

impl Backend {
    pub fn new(addrs: &str) -> Result<Backend, InvalidUri> {
        let uri = addrs.parse::<http::Uri>()?;

        Ok(Self {
            uri,
            request_limit_per_second: 10,
            served_requests: 0,
        })
    }
}

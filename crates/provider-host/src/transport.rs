#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use std::time::Duration;

/// Common HTTP transport owned by provider-host.
/// Extension providers do NOT create their own HTTP clients.
#[derive(Debug, Clone)]
pub struct HttpTransport {
    pub client: reqwest::Client,
    pub base_url: String,
    pub timeout: Duration,
}

impl HttpTransport {
    pub fn new(base_url: &str, timeout: Duration) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .expect("reqwest client must build"),
            base_url: base_url.to_string(),
            timeout,
        }
    }
}
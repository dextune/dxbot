#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use std::fmt;
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

/// Stable transport-construction failure that never echoes credential bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportBuildError {
    InvalidCredential,
    ClientBuild,
}

/// Common HTTP transport owned by provider-host.
/// Extension providers do NOT create their own HTTP clients.
#[derive(Clone)]
pub struct HttpTransport {
    pub client: reqwest::Client,
    pub base_url: String,
    pub timeout: Duration,
}

impl fmt::Debug for HttpTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpTransport")
            .field("base_url", &self.base_url)
            .field("timeout", &self.timeout)
            .finish_non_exhaustive()
    }
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

    /// Rebuilds the Common-owned client with one scoped bearer credential.
    /// Neither the error nor `Debug` representation contains the raw secret.
    pub fn with_bearer(&self, secret: &str) -> Result<Self, TransportBuildError> {
        let value = HeaderValue::from_str(&format!("Bearer {secret}"))
            .map_err(|_| TransportBuildError::InvalidCredential)?;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, value);
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .default_headers(headers)
            .build()
            .map_err(|_| TransportBuildError::ClientBuild)?;
        Ok(Self {
            client,
            base_url: self.base_url.clone(),
            timeout: self.timeout,
        })
    }
}

//! Model adapter SPI for HTTP-protocol providers.
//!
//! A `ModelAdapter` supplies only model-specific request building and event
//! mapping. The Common [`HttpExecuteProvider`](crate::http_provider) owns the
//! transport, protocol execution, deadline/cancellation, and output bounding,
//! and drives the canonical async [`ExecuteProvider`](crate::execute) contract.

#![forbid(unsafe_code)]

use std::fmt::Debug;

use dxbot_core::types::ProviderId;

use crate::execute::ExecuteRequest;
use crate::protocol::{ProviderEvent, ProviderRequest};

/// Model-specific adapter for the Common HTTP protocol handler.
///
/// Retained public name `RealProvider` for compatibility with existing direct
/// adapters and external references.
pub trait RealProvider: Debug + Send + Sync {
    fn id(&self) -> &ProviderId;
    fn capability(&self) -> &str;
    fn generation(&self) -> i64;

    /// Build model-specific fields for the Common protocol handler.
    fn build_request(&self, request: &ExecuteRequest) -> ProviderRequest;

    /// Optional: post-process raw events into provider-specific evidence
    /// (default: passthrough).
    fn map_event(&self, event: ProviderEvent) -> ProviderEvent {
        event
    }
}

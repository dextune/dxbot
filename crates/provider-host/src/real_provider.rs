#![forbid(unsafe_code)]

use std::fmt::Debug;

use dxbot_core::types::ProviderId;

use crate::harness::TaskDescription;
use crate::protocol::{ProviderEvent, ProviderRequest};

/// Extension providers implement this trait.
/// Common handler owns transport, lifecycle, and output bounding.
pub trait RealProvider: Debug + Send + Sync {
    fn id(&self) -> &ProviderId;
    fn capability(&self) -> &str;
    fn generation(&self) -> i64;

    /// Build model-specific fields for the common protocol handler.
    fn build_request(&self, task: &TaskDescription) -> ProviderRequest;

    /// Optional: post-process raw events into provider-specific
    /// evidence (default: passthrough).
    fn map_event(&self, event: ProviderEvent) -> ProviderEvent {
        event
    }
}
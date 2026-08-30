#![forbid(unsafe_code)]

use dxbot_core::types::ProviderId;

use crate::execute::ExecuteRequest;
use crate::protocol::{ChatMessage, ProviderRequest};
use crate::real_provider::RealProvider;

/// MiniMax M3 adapter for the Anthropic-compatible Messages endpoint.
/// Common owns transport, credentials, lifecycle, bounds, and error mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MiniMaxM3Adapter {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    model: String,
}

impl MiniMaxM3Adapter {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self::configured(id, capability, generation, "MiniMax-M3")
    }

    pub fn configured(id: ProviderId, capability: &str, generation: i64, model: &str) -> Self {
        Self {
            id,
            capability: capability.to_owned(),
            generation,
            model: model.to_owned(),
        }
    }
}

impl RealProvider for MiniMaxM3Adapter {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn capability(&self) -> &str {
        &self.capability
    }

    fn generation(&self) -> i64 {
        self.generation
    }

    fn build_request(&self, request: &ExecuteRequest) -> ProviderRequest {
        ProviderRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_owned(),
                content: format!("{}\n\nContext: {}", request.intent, request.context),
            }],
            max_tokens: request
                .budget
                .map(|budget| u32::try_from(budget).unwrap_or(u32::MAX)),
            temperature: Some(0.0),
            stream: false,
        }
    }
}

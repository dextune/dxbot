#![forbid(unsafe_code)]

use dxbot_core::types::ProviderId;

use crate::execute::ExecuteRequest;
use crate::protocol::{ChatMessage, ProviderEvent, ProviderRequest};
use crate::real_provider::RealProvider;

/// DeepSeek v4 Flash 0731 adapter.
/// Model-specific configuration only; Common owns transport and execution policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekFlashAdapter {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    model: String,
}

impl DeepSeekFlashAdapter {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self::configured(id, capability, generation, "alibaba/deepseek-v4-flash-0731")
    }

    pub fn configured(id: ProviderId, capability: &str, generation: i64, model: &str) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            model: model.to_string(),
        }
    }
}

impl RealProvider for DeepSeekFlashAdapter {
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
                role: "user".to_string(),
                content: format!("{}\n\nContext: {}", request.intent, request.context),
            }],
            max_tokens: request
                .budget
                .map(|budget| u32::try_from(budget).unwrap_or(u32::MAX)),
            temperature: Some(0.0),
            stream: false,
        }
    }

    fn map_event(&self, event: ProviderEvent) -> ProviderEvent {
        event
    }
}

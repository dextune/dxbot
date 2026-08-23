#![forbid(unsafe_code)]

use dxbot_core::types::ProviderId;

use crate::harness::TaskDescription;
use crate::protocol::{ChatMessage, ProviderEvent, ProviderRequest};
use crate::real_provider::RealProvider;

/// DeepSeek v4 Flash 0731 adapter.
/// Implements `RealProvider` trait — first real provider canary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepSeekFlashAdapter {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    model: String,
}

impl DeepSeekFlashAdapter {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            model: "alibaba/deepseek-v4-flash-0731".to_string(),
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

    fn build_request(&self, task: &TaskDescription) -> ProviderRequest {
        ProviderRequest {
            model: self.model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: format!("{}\n\nContext: {}", task.intent, task.context),
            }],
            max_tokens: task.budget.map(|b| b as u32),
            temperature: Some(0.0),
            stream: false,
        }
    }

    fn map_event(&self, event: ProviderEvent) -> ProviderEvent {
        // deepseek reasoning tokens are already separated by Common handler.
        // Pass through without modification.
        event
    }
}
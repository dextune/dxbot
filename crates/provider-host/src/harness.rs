//! Provider-host harness (`AT-HARNESS-001`): Reference Provider plus one real
//! Harness Adapter canary.
//!
//! ProviderHost owns provider registration, common transport/protocol execution,
//! deadline interpretation, and fallback ordering. Extension providers supply
//! model-specific request/event mapping only.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::expect_used)]

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dxbot_core::types::ProviderId;

use crate::protocol::{ProviderError, ProviderEvent, ProviderExecuteConfig};
use crate::real_provider::RealProvider;
use crate::transport::HttpTransport;

/// A bounded task description executed by a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDescription {
    /// The task's stated intent/goal.
    pub intent: String,
    /// Bounded context the provider may use.
    pub context: String,
    /// Optional execution budget.
    pub budget: Option<u64>,
    /// Optional wall-clock deadline (unix seconds).
    pub deadline: Option<i64>,
}

/// A piece of provider-produced evidence attached to a [`TaskResult`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub provider: ProviderId,
    pub observation: String,
}

/// Terminal execution status of a bounded task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Completed,
    Failed,
    Cancelled,
}

/// The result of executing a [`TaskDescription`] through a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub output: String,
    pub evidence: Vec<Evidence>,
    pub status: TaskStatus,
}

/// Availability of a registered provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Ready,
    Unavailable,
}

/// Read-only registration record for a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    pub status: ProviderStatus,
}

/// Errors produced by the provider-host harness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessError {
    AlreadyRegistered { id: ProviderId },
    InvalidProvider { reason: String },
    NoReferenceProvider,
    NoHarnessAdapter,
    ProviderUnavailable { id: ProviderId },
    ProviderNotFound { id: ProviderId },
    DeadlineExceeded,
    ExecutionFailed { id: ProviderId, reason: String },
}

/// The deterministic Reference Provider used by explicit test/canary policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceProvider {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
}

impl ReferenceProvider {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
        }
    }

    pub fn execute(&self, task: &TaskDescription) -> Result<TaskResult, HarnessError> {
        Ok(self.completed(task.intent.clone()))
    }

    fn completed(&self, output: String) -> TaskResult {
        TaskResult {
            output,
            evidence: vec![Evidence {
                provider: self.id.clone(),
                observation: "reference-provider".to_string(),
            }],
            status: TaskStatus::Completed,
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id.clone(),
            capability: self.capability.clone(),
            generation: self.generation,
            status: ProviderStatus::Ready,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdapterMode {
    Succeed,
    Unavailable,
    Fail,
}

/// The real Harness Adapter canary used by `AT-HARNESS-001`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessAdapter {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    mode: AdapterMode,
}

impl HarnessAdapter {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            mode: AdapterMode::Succeed,
        }
    }

    pub fn unavailable(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            mode: AdapterMode::Unavailable,
        }
    }

    pub fn failing(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            mode: AdapterMode::Fail,
        }
    }

    pub fn execute(&self, task: &TaskDescription) -> Result<TaskResult, HarnessError> {
        match self.mode {
            AdapterMode::Succeed => Ok(TaskResult {
                output: format!("adapter:{}", task.intent),
                evidence: vec![Evidence {
                    provider: self.id.clone(),
                    observation: "harness-adapter-canary".to_string(),
                }],
                status: TaskStatus::Completed,
            }),
            AdapterMode::Unavailable => Err(HarnessError::ProviderUnavailable {
                id: self.id.clone(),
            }),
            AdapterMode::Fail => Err(HarnessError::ExecutionFailed {
                id: self.id.clone(),
                reason: "canary execution failed".to_string(),
            }),
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: self.id.clone(),
            capability: self.capability.clone(),
            generation: self.generation,
            status: match self.mode {
                AdapterMode::Unavailable => ProviderStatus::Unavailable,
                AdapterMode::Succeed | AdapterMode::Fail => ProviderStatus::Ready,
            },
        }
    }
}

/// Provider host and common execution authority.
pub struct ProviderHost {
    reference: Option<ReferenceProvider>,
    adapter: Option<HarnessAdapter>,
    protocol: Option<crate::protocol::ChatCompletionProtocol>,
    real_providers: Vec<Box<dyn RealProvider>>,
    rt: tokio::runtime::Runtime,
}

impl fmt::Debug for ProviderHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderHost")
            .field("reference", &self.reference)
            .field("adapter", &self.adapter)
            .field("protocol", &self.protocol)
            .field("real_providers_count", &self.real_providers.len())
            .finish()
    }
}

impl Default for ProviderHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderHost {
    pub fn new() -> Self {
        Self {
            reference: None,
            adapter: None,
            protocol: None,
            real_providers: Vec::new(),
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("tokio runtime must build"),
        }
    }

    pub fn register_reference_provider(
        &mut self,
        provider: ReferenceProvider,
    ) -> Result<(), HarnessError> {
        if self.provider_id_registered(&provider.id) {
            return Err(HarnessError::AlreadyRegistered {
                id: provider.id.clone(),
            });
        }
        validate_provider(&provider.id, &provider.capability, provider.generation)?;
        self.reference = Some(provider);
        Ok(())
    }

    pub fn register_harness_adapter(
        &mut self,
        adapter: HarnessAdapter,
    ) -> Result<(), HarnessError> {
        if self.provider_id_registered(&adapter.id) {
            return Err(HarnessError::AlreadyRegistered {
                id: adapter.id.clone(),
            });
        }
        validate_provider(&adapter.id, &adapter.capability, adapter.generation)?;
        self.adapter = Some(adapter);
        Ok(())
    }

    /// Installs the single Common-owned transport/protocol state.
    pub fn set_transport(&mut self, transport: HttpTransport) {
        self.protocol = Some(crate::protocol::ChatCompletionProtocol::new(
            transport,
            1024 * 1024,
            1000,
        ));
    }

    pub fn register_real_provider(
        &mut self,
        provider: Box<dyn RealProvider>,
    ) -> Result<(), HarnessError> {
        if self.provider_id_registered(provider.id()) {
            return Err(HarnessError::AlreadyRegistered {
                id: provider.id().clone(),
            });
        }
        validate_provider(provider.id(), provider.capability(), provider.generation())?;
        self.real_providers.push(provider);
        Ok(())
    }

    /// Executes through real providers first, then the explicit canary adapter,
    /// then the explicitly registered Reference Provider test fallback.
    pub fn execute_task(&self, task: &TaskDescription) -> Result<TaskResult, HarnessError> {
        let deadline = deadline_instant(task.deadline)?;

        for provider in &self.real_providers {
            let protocol = self
                .protocol
                .as_ref()
                .ok_or_else(|| HarnessError::ExecutionFailed {
                    id: provider.id().clone(),
                    reason: "common provider transport is not configured".to_string(),
                })?;
            let request = provider.build_request(task);
            let config = ProviderExecuteConfig {
                deadline,
                cancel_token: None,
                max_output_bytes: protocol.max_output_bytes,
                max_output_items: protocol.max_output_items,
            };

            match self.rt.block_on(protocol.execute(&request, &config)) {
                Ok(events) => return real_provider_result(provider.as_ref(), events),
                Err(ProviderError::TransportUnavailable { .. })
                | Err(ProviderError::UpstreamUnavailable { .. }) => continue,
                Err(ProviderError::DeadlineExceeded) => return Err(HarnessError::DeadlineExceeded),
                Err(error) => {
                    return Err(HarnessError::ExecutionFailed {
                        id: provider.id().clone(),
                        reason: format!("{error:?}"),
                    });
                }
            }
        }

        if let Some(adapter) = &self.adapter {
            match adapter.execute(task) {
                Ok(result) => return Ok(result),
                Err(HarnessError::ProviderUnavailable { .. }) => {}
                Err(error) => return Err(error),
            }
        }

        self.reference
            .as_ref()
            .ok_or(HarnessError::NoReferenceProvider)?
            .execute(task)
    }

    pub fn list_providers(&self, capability: Option<&str>) -> Vec<ProviderInfo> {
        let mut out = Vec::new();
        if let Some(reference) = &self.reference {
            if capability.is_none_or(|requested| reference.capability == requested) {
                out.push(reference.info());
            }
        }
        if let Some(adapter) = &self.adapter {
            if capability.is_none_or(|requested| adapter.capability == requested) {
                out.push(adapter.info());
            }
        }
        for provider in &self.real_providers {
            if capability.is_none_or(|requested| provider.capability() == requested) {
                out.push(ProviderInfo {
                    id: provider.id().clone(),
                    capability: provider.capability().to_string(),
                    generation: provider.generation(),
                    status: ProviderStatus::Ready,
                });
            }
        }
        out
    }

    pub fn get_provider(&self, provider_id: &ProviderId) -> Result<ProviderInfo, HarnessError> {
        if let Some(reference) = &self.reference {
            if &reference.id == provider_id {
                return Ok(reference.info());
            }
        }
        if let Some(adapter) = &self.adapter {
            if &adapter.id == provider_id {
                return Ok(adapter.info());
            }
        }
        for provider in &self.real_providers {
            if provider.id() == provider_id {
                return Ok(ProviderInfo {
                    id: provider.id().clone(),
                    capability: provider.capability().to_string(),
                    generation: provider.generation(),
                    status: ProviderStatus::Ready,
                });
            }
        }
        Err(HarnessError::ProviderNotFound {
            id: provider_id.clone(),
        })
    }

    fn provider_id_registered(&self, provider_id: &ProviderId) -> bool {
        self.reference
            .as_ref()
            .is_some_and(|provider| &provider.id == provider_id)
            || self
                .adapter
                .as_ref()
                .is_some_and(|provider| &provider.id == provider_id)
            || self
                .real_providers
                .iter()
                .any(|provider| provider.id() == provider_id)
    }
}

fn real_provider_result(
    provider: &dyn RealProvider,
    events: Vec<ProviderEvent>,
) -> Result<TaskResult, HarnessError> {
    let mut output = String::new();
    let mut completed = false;
    for event in events {
        match provider.map_event(event) {
            ProviderEvent::ContentDelta { text, .. } => output.push_str(&text),
            ProviderEvent::ReasoningDelta { .. } => {}
            ProviderEvent::Completed { .. } => completed = true,
            ProviderEvent::Failed { reason } => {
                return Err(HarnessError::ExecutionFailed {
                    id: provider.id().clone(),
                    reason,
                });
            }
            ProviderEvent::Cancelled => {
                return Err(HarnessError::ExecutionFailed {
                    id: provider.id().clone(),
                    reason: "provider execution cancelled".to_string(),
                });
            }
        }
    }
    if !completed {
        return Err(HarnessError::ExecutionFailed {
            id: provider.id().clone(),
            reason: "provider stream ended without completion".to_string(),
        });
    }
    Ok(TaskResult {
        output,
        evidence: vec![Evidence {
            provider: provider.id().clone(),
            observation: format!("real-provider:{}", provider.capability()),
        }],
        status: TaskStatus::Completed,
    })
}

fn deadline_instant(deadline: Option<i64>) -> Result<Option<tokio::time::Instant>, HarnessError> {
    let Some(deadline) = deadline else {
        return Ok(None);
    };
    let now_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let now_seconds = i64::try_from(now_seconds).unwrap_or(i64::MAX);
    let remaining = deadline
        .checked_sub(now_seconds)
        .filter(|seconds| *seconds > 0)
        .ok_or(HarnessError::DeadlineExceeded)?;
    let remaining = u64::try_from(remaining).map_err(|_| HarnessError::DeadlineExceeded)?;
    Ok(Some(
        tokio::time::Instant::now() + Duration::from_secs(remaining),
    ))
}

fn validate_provider(
    id: &ProviderId,
    capability: &str,
    generation: i64,
) -> Result<(), HarnessError> {
    if id.0.trim().is_empty() {
        return Err(HarnessError::InvalidProvider {
            reason: "provider id must not be empty".to_string(),
        });
    }
    if capability.trim().is_empty() {
        return Err(HarnessError::InvalidProvider {
            reason: "capability must not be empty".to_string(),
        });
    }
    if generation < 0 {
        return Err(HarnessError::InvalidProvider {
            reason: "generation must not be negative".to_string(),
        });
    }
    Ok(())
}

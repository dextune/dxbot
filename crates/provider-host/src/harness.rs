//! Provider-host harness (`AT-HARNESS-001`): Reference Provider plus one real
//! Harness Adapter canary.
//!
//! The Reference Provider is deterministic and always available once
//! registered; the Harness Adapter is the "real" canary that completes a
//! bounded task through the provider chain. When the real adapter is not
//! registered or is temporarily unavailable, execution falls back to the
//! Reference Provider so the host never silently loses the task.
//!
//! All types are data values: `Debug + Clone + PartialEq`.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::expect_used)]

use std::fmt;

use dxbot_core::types::ProviderId;

use crate::protocol::ProviderExecuteConfig;
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
    /// The provider that produced this evidence.
    pub provider: ProviderId,
    /// Bounded observation text.
    pub observation: String,
}

/// Terminal execution status of a bounded task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// The task completed successfully.
    Completed,
    /// The task failed.
    Failed,
    /// The task was cancelled.
    Cancelled,
}

/// The result of executing a [`TaskDescription`] through a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    /// The produced output.
    pub output: String,
    /// Provider-attributed evidence for the result.
    pub evidence: Vec<Evidence>,
    /// Terminal status.
    pub status: TaskStatus,
}

/// Availability of a registered provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    /// The provider can execute tasks.
    Ready,
    /// The provider is registered but unavailable.
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
    /// A provider with the same id is already registered.
    AlreadyRegistered { id: ProviderId },
    /// A registration failed validation.
    InvalidProvider { reason: String },
    /// No Reference Provider is registered.
    NoReferenceProvider,
    /// No real Harness Adapter is registered.
    NoHarnessAdapter,
    /// The adapter is temporarily unavailable; execution should fall back.
    ProviderUnavailable { id: ProviderId },
    /// No provider matches the requested id.
    ProviderNotFound { id: ProviderId },
    /// A provider failed to execute the bounded task.
    ExecutionFailed { id: ProviderId, reason: String },
}

/// The deterministic Reference Provider: always completes a bounded task
/// against its `intent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceProvider {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
}

impl ReferenceProvider {
    /// Creates a reference provider with the given identity.
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
        }
    }

    /// Deterministically completes the task, echoing the intent as output.
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

/// Behaviour selectable for the real Harness Adapter canary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdapterMode {
    /// The adapter completes the bounded task.
    Succeed,
    /// The adapter is temporarily unavailable; the chain falls back.
    Unavailable,
    /// The adapter fails execution.
    Fail,
}

/// The "real" Harness Adapter canary. It executes a bounded task through the
/// provider chain and, when unavailable, triggers fallback to the Reference
/// Provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessAdapter {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    mode: AdapterMode,
}

impl HarnessAdapter {
    /// Creates a succeeding adapter (the default canary).
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            mode: AdapterMode::Succeed,
        }
    }

    /// Creates an adapter that is temporarily unavailable (triggers fallback).
    pub fn unavailable(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            mode: AdapterMode::Unavailable,
        }
    }

    /// Creates an adapter that fails execution.
    pub fn failing(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_string(),
            generation,
            mode: AdapterMode::Fail,
        }
    }

    /// Executes a bounded task through the real adapter canary.
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

/// The provider host: owns the Reference Provider, the real Harness Adapter,
/// common transport/protocol infrastructure, and registered real providers.
///
/// The execution chain is:
/// 1. Real providers (registered extension providers)
/// 2. HarnessAdapter (canary)
/// 3. ReferenceProvider (deterministic fallback)
pub struct ProviderHost {
    reference: Option<ReferenceProvider>,
    adapter: Option<HarnessAdapter>,
    transport: Option<HttpTransport>,
    protocol: Option<crate::protocol::ChatCompletionProtocol>,
    real_providers: Vec<Box<dyn RealProvider>>,
    rt: tokio::runtime::Runtime,
}

impl fmt::Debug for ProviderHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProviderHost")
            .field("reference", &self.reference)
            .field("adapter", &self.adapter)
            .field("transport", &self.transport)
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
    /// Creates an empty host with a single-threaded async runtime.
    /// Register the Reference Provider and at least one real provider or
    /// Harness Adapter before the canary test.
    pub fn new() -> Self {
        Self {
            reference: None,
            adapter: None,
            transport: None,
            protocol: None,
            real_providers: Vec::new(),
            rt: tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("tokio runtime must build"),
        }
    }

    /// Registers the deterministic Reference Provider.
    pub fn register_reference_provider(
        &mut self,
        provider: ReferenceProvider,
    ) -> Result<(), HarnessError> {
        if self.reference.is_some() {
            return Err(HarnessError::AlreadyRegistered {
                id: provider.id.clone(),
            });
        }
        validate_provider(&provider.id, &provider.capability)?;
        self.reference = Some(provider);
        Ok(())
    }

    /// Registers a real Harness Adapter.
    pub fn register_harness_adapter(
        &mut self,
        adapter: HarnessAdapter,
    ) -> Result<(), HarnessError> {
        if self.adapter.is_some() {
            return Err(HarnessError::AlreadyRegistered {
                id: adapter.id.clone(),
            });
        }
        validate_provider(&adapter.id, &adapter.capability)?;
        self.adapter = Some(adapter);
        Ok(())
    }

    /// Sets the common HTTP transport and creates the chat completion
    /// protocol handler with reasonable output bounds.
    pub fn set_transport(&mut self, transport: HttpTransport) {
        let protocol = crate::protocol::ChatCompletionProtocol::new(
            transport.clone(),
            1024 * 1024, // 1 MiB max output bytes
            1000,        // max output items
        );
        self.transport = Some(transport);
        self.protocol = Some(protocol);
    }

    /// Registers a real (extension) provider.
    pub fn register_real_provider(
        &mut self,
        provider: Box<dyn RealProvider>,
    ) -> Result<(), HarnessError> {
        // Check for duplicate ID among real providers
        for existing in &self.real_providers {
            if existing.id() == provider.id() {
                return Err(HarnessError::AlreadyRegistered {
                    id: provider.id().clone(),
                });
            }
        }
        // Check for duplicate ID with reference and adapter
        if let Some(r) = &self.reference {
            if r.id == *provider.id() {
                return Err(HarnessError::AlreadyRegistered {
                    id: provider.id().clone(),
                });
            }
        }
        if let Some(a) = &self.adapter {
            if a.id == *provider.id() {
                return Err(HarnessError::AlreadyRegistered {
                    id: provider.id().clone(),
                });
            }
        }
        validate_provider(provider.id(), provider.capability())?;
        self.real_providers.push(provider);
        Ok(())
    }

    /// Executes a task through the provider chain.
    ///
    /// Chain order:
    /// 1. Real providers (registered extension providers)
    /// 2. HarnessAdapter (canary)
    /// 3. ReferenceProvider (deterministic fallback)
    pub fn execute_task(&self, task: &TaskDescription) -> Result<TaskResult, HarnessError> {
        // 1. Try real providers first
        for provider in &self.real_providers {
            let protocol = self
                .protocol
                .as_ref()
                .ok_or(HarnessError::NoReferenceProvider)?;
            let request = provider.build_request(task);
            let config = ProviderExecuteConfig {
                deadline: task
                    .deadline
                    .map(|d| tokio::time::Instant::now() + std::time::Duration::from_secs(d as u64)),
                cancel_notify: None,
                max_output_bytes: protocol.max_output_bytes,
                max_output_items: protocol.max_output_items,
            };

            match self.rt.block_on(protocol.execute(&request, &config)) {
                Ok(events) => {
                    let output = events
                        .iter()
                        .filter_map(|e| match e {
                            crate::protocol::ProviderEvent::ContentDelta { text, .. } => {
                                Some(text.as_str())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    if !output.is_empty() {
                        return Ok(TaskResult {
                            output,
                            evidence: vec![Evidence {
                                provider: provider.id().clone(),
                                observation: format!(
                                    "real-provider:{}",
                                    provider.capability()
                                ),
                            }],
                            status: TaskStatus::Completed,
                        });
                    }
                    // Fall through if empty output
                }
                Err(crate::protocol::ProviderError::TransportUnavailable { .. })
                | Err(crate::protocol::ProviderError::UpstreamUnavailable { .. }) => {
                    // Fall through to next provider
                    continue;
                }
                Err(e) => {
                    return Err(HarnessError::ExecutionFailed {
                        id: provider.id().clone(),
                        reason: format!("{e:?}"),
                    });
                }
            }
        }

        // 2. Try HarnessAdapter
        let reference = self
            .reference
            .as_ref()
            .ok_or(HarnessError::NoReferenceProvider)?;
        if let Some(adapter) = &self.adapter {
            match adapter.execute(task) {
                Ok(result) => return Ok(result),
                Err(HarnessError::ProviderUnavailable { .. }) => {
                    // Real canary is unavailable: fall back to Reference.
                }
                Err(err) => return Err(err),
            }
        }

        // 3. Fall back to ReferenceProvider
        reference.execute(task)
    }

    /// Lists registered providers, optionally filtered by capability.
    pub fn list_providers(&self, capability: Option<&str>) -> Vec<ProviderInfo> {
        let mut out = Vec::new();
        if let Some(reference) = &self.reference {
            if capability.is_none_or(|cap| reference.capability == cap) {
                out.push(reference.info());
            }
        }
        if let Some(adapter) = &self.adapter {
            if capability.is_none_or(|cap| adapter.capability == cap) {
                out.push(adapter.info());
            }
        }
        for provider in &self.real_providers {
            let cap = provider.capability();
            if capability.is_none_or(|c| cap == c) {
                out.push(ProviderInfo {
                    id: provider.id().clone(),
                    capability: cap.to_string(),
                    generation: provider.generation(),
                    status: ProviderStatus::Ready,
                });
            }
        }
        out
    }

    /// Returns the registration info for a specific provider id.
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
}

/// Validates a provider registration identity.
fn validate_provider(_id: &ProviderId, capability: &str) -> Result<(), HarnessError> {
    if capability.trim().is_empty() {
        return Err(HarnessError::InvalidProvider {
            reason: "capability must not be empty".to_string(),
        });
    }
    Ok(())
}
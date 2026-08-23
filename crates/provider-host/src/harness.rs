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

use dxbot_core::types::ProviderId;

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

/// The provider host: owns the Reference Provider and the real Harness Adapter
/// and executes bounded tasks through the provider chain.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderHost {
    reference: Option<ReferenceProvider>,
    adapter: Option<HarnessAdapter>,
}

impl ProviderHost {
    /// Creates an empty host. Register the Reference Provider and at least one
    /// real Harness Adapter before the canary test.
    pub fn new() -> Self {
        Self::default()
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

    /// Executes a task through the provider chain.
    ///
    /// The real Harness Adapter is preferred (the canary). When it is not
    /// registered or is temporarily unavailable, execution falls back to the
    /// always-available Reference Provider.
    pub fn execute_task(&self, task: &TaskDescription) -> Result<TaskResult, HarnessError> {
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
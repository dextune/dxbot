//! Bounded typed diagnostics separate from durable audit correctness.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audit::redact_diagnostic_value;

const MAX_COMPONENT_ID_BYTES: usize = 96;
const MAX_ATTRIBUTE_VALUE_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticComponent {
    RuntimeHost,
    ProviderHost,
    Storage,
    Sandbox,
    Security,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticFamily {
    RuntimeLifecycle,
    RuntimeExtensionGraph,
    StorageIntegrity,
    StorageRecovery,
    ProviderLifecycle,
    ProviderProtocol,
    SandboxOwnership,
    SandboxArtifact,
    SandboxReadiness,
    SecurityAuthorization,
    ResourceExhaustion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticReason {
    GraphInvalid,
    StartFailed,
    StopFailed,
    SnapshotCaptured,
    SnapshotNoChange,
    SnapshotOverLimit,
    SnapshotSinkFailed,
    ProviderMalformedFrame,
    ProviderCancelled,
    SandboxUnowned,
    SandboxStaleGeneration,
    SandboxDigestMismatch,
    SandboxCleanupFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafeAttributeKey {
    ExtensionId,
    ProviderId,
    ResourceKind,
    LifecycleState,
    Expected,
    Observed,
    DetailClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeAttribute {
    pub key: SafeAttributeKey,
    pub value: String,
}

impl SafeAttribute {
    pub fn new(key: SafeAttributeKey, value: &str) -> Self {
        Self {
            key,
            value: bounded_redacted(value, MAX_ATTRIBUTE_VALUE_BYTES),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub event_id: u64,
    pub observed_at: i64,
    pub component: DiagnosticComponent,
    pub component_id: String,
    pub family: DiagnosticFamily,
    pub reason: DiagnosticReason,
    pub severity: DiagnosticSeverity,
    pub safe_attributes: Vec<SafeAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticError {
    InvalidCapacity,
    SinkUnavailable,
    EventTooLarge,
    Poisoned,
}

#[derive(Debug)]
struct DiagnosticState {
    events: VecDeque<DiagnosticEvent>,
    retained_bytes: usize,
    next_event_id: u64,
    dropped_events: u64,
    available: bool,
}

#[derive(Debug, Clone)]
pub struct DiagnosticSink {
    state: Arc<Mutex<DiagnosticState>>,
    max_events: usize,
    max_retained_bytes: usize,
    max_event_bytes: usize,
}

impl DiagnosticSink {
    pub fn new(
        max_events: usize,
        max_retained_bytes: usize,
        max_event_bytes: usize,
    ) -> Result<Self, DiagnosticError> {
        if max_events == 0
            || max_retained_bytes == 0
            || max_event_bytes == 0
            || max_event_bytes > max_retained_bytes
        {
            return Err(DiagnosticError::InvalidCapacity);
        }

        Ok(Self {
            state: Arc::new(Mutex::new(DiagnosticState {
                events: VecDeque::new(),
                retained_bytes: 0,
                next_event_id: 1,
                dropped_events: 0,
                available: true,
            })),
            max_events,
            max_retained_bytes,
            max_event_bytes,
        })
    }

    pub fn emit(
        &self,
        component: DiagnosticComponent,
        component_id: &str,
        family: DiagnosticFamily,
        reason: DiagnosticReason,
        severity: DiagnosticSeverity,
        safe_attributes: &[SafeAttribute],
    ) -> Result<u64, DiagnosticError> {
        let mut state = self.state.lock().map_err(|_| DiagnosticError::Poisoned)?;
        if !state.available {
            return Err(DiagnosticError::SinkUnavailable);
        }

        let event_id = state.next_event_id;
        let event = DiagnosticEvent {
            event_id,
            observed_at: now_unix_secs(),
            component,
            component_id: bounded_redacted(component_id, MAX_COMPONENT_ID_BYTES),
            family,
            reason,
            severity,
            safe_attributes: safe_attributes.to_vec(),
        };
        let event_bytes = event_size(&event);
        if event_bytes > self.max_event_bytes || event_bytes > self.max_retained_bytes {
            return Err(DiagnosticError::EventTooLarge);
        }

        while state.events.len() >= self.max_events
            || state.retained_bytes.saturating_add(event_bytes) > self.max_retained_bytes
        {
            let Some(dropped) = state.events.pop_front() else {
                break;
            };
            state.retained_bytes = state.retained_bytes.saturating_sub(event_size(&dropped));
            state.dropped_events = state.dropped_events.saturating_add(1);
        }

        state.next_event_id = state.next_event_id.saturating_add(1);
        state.retained_bytes = state.retained_bytes.saturating_add(event_bytes);
        state.events.push_back(event);
        Ok(event_id)
    }

    pub fn events(&self) -> Result<Vec<DiagnosticEvent>, DiagnosticError> {
        let state = self.state.lock().map_err(|_| DiagnosticError::Poisoned)?;
        Ok(state.events.iter().cloned().collect())
    }

    pub fn dropped_events(&self) -> Result<u64, DiagnosticError> {
        let state = self.state.lock().map_err(|_| DiagnosticError::Poisoned)?;
        Ok(state.dropped_events)
    }

    pub fn latest_watermark(&self) -> Result<u64, DiagnosticError> {
        let state = self.state.lock().map_err(|_| DiagnosticError::Poisoned)?;
        Ok(state.next_event_id.saturating_sub(1))
    }

    pub fn set_available(&self, available: bool) -> Result<(), DiagnosticError> {
        let mut state = self.state.lock().map_err(|_| DiagnosticError::Poisoned)?;
        state.available = available;
        Ok(())
    }
}

fn bounded_redacted(value: &str, max_bytes: usize) -> String {
    let redacted = redact_diagnostic_value(value);
    if redacted.len() <= max_bytes {
        return redacted;
    }

    let mut end = max_bytes;
    while end > 0 && !redacted.is_char_boundary(end) {
        end -= 1;
    }
    redacted[..end].to_owned()
}

fn event_size(event: &DiagnosticEvent) -> usize {
    event
        .component_id
        .len()
        .saturating_add(
            event
                .safe_attributes
                .iter()
                .map(|attribute| attribute.value.len().saturating_add(8))
                .sum::<usize>(),
        )
        .saturating_add(64)
}

fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_redacted_and_bounded() {
        let sink = DiagnosticSink::new(2, 512, 256);
        assert!(sink.is_ok());
        let Ok(sink) = sink else {
            return;
        };

        let secret = SafeAttribute::new(SafeAttributeKey::Observed, "token=secret-value");
        assert!(
            sink.emit(
                DiagnosticComponent::ProviderHost,
                "provider-a",
                DiagnosticFamily::ProviderProtocol,
                DiagnosticReason::ProviderMalformedFrame,
                DiagnosticSeverity::Error,
                &[secret],
            )
            .is_ok()
        );
        assert!(
            sink.emit(
                DiagnosticComponent::RuntimeHost,
                "runtime",
                DiagnosticFamily::RuntimeLifecycle,
                DiagnosticReason::StartFailed,
                DiagnosticSeverity::Warning,
                &[],
            )
            .is_ok()
        );
        assert!(
            sink.emit(
                DiagnosticComponent::Storage,
                "store",
                DiagnosticFamily::StorageRecovery,
                DiagnosticReason::SnapshotCaptured,
                DiagnosticSeverity::Info,
                &[],
            )
            .is_ok()
        );

        let events = sink.events();
        assert!(events.is_ok());
        let Ok(events) = events else {
            return;
        };
        assert_eq!(events.len(), 2);
        assert_eq!(sink.dropped_events(), Ok(1));
        assert!(!format!("{events:?}").contains("secret-value"));
    }

    #[test]
    fn sink_failure_is_explicit_and_non_blocking_for_callers() {
        let sink = DiagnosticSink::new(4, 1024, 256);
        assert!(sink.is_ok());
        let Ok(sink) = sink else {
            return;
        };
        assert!(sink.set_available(false).is_ok());
        assert_eq!(
            sink.emit(
                DiagnosticComponent::RuntimeHost,
                "runtime",
                DiagnosticFamily::RuntimeLifecycle,
                DiagnosticReason::StopFailed,
                DiagnosticSeverity::Error,
                &[],
            ),
            Err(DiagnosticError::SinkUnavailable)
        );
    }
}

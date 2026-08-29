//! Crash-reconstructable projection from canonical producer AuditIntents into
//! the durable Runtime Audit outbox. Projection failure stops new dispatch but
//! never erases canonical intent.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use application::{ApplicationMutator, ExecutionAuditPhase};
use runtime_audit::{AuditIntent, AuditOutbox};
use runtime_security::SecurityState;

#[derive(Debug)]
pub(crate) struct AuditProjection {
    outbox: Arc<AuditOutbox>,
    healthy: AtomicBool,
    last_error: Mutex<Option<String>>,
}

impl AuditProjection {
    pub(crate) fn new(outbox: Arc<AuditOutbox>) -> Arc<Self> {
        Arc::new(Self {
            outbox,
            healthy: AtomicBool::new(true),
            last_error: Mutex::new(None),
        })
    }

    pub(crate) fn drain(
        &self,
        application: &ApplicationMutator,
        security: &Arc<Mutex<SecurityState>>,
    ) -> Result<usize, String> {
        let application_intents =
            application
                .execution_audit_intents(100_000)
                .map_err(|error| {
                    self.fail(format!("cannot read Application AuditIntents: {error:?}"))
                })?;
        let security_intents = security
            .lock()
            .map_err(|_| self.fail("Security state lock poisoned".to_owned()))?
            .audit_intents();
        let mut projected = 0usize;
        for intent in application_intents {
            let mapped = AuditIntent {
                key: format!("application:{}", intent.key),
                source: "application".to_owned(),
                operation_id: None,
                principal_ref: None,
                action: phase_name(intent.phase).to_owned(),
                target_ref: format!("execution:{}", intent.execution_id.0),
                detail: format!(
                    "task={} process={} provider={}@{} {}",
                    intent.task_id.0,
                    intent.process_id.0,
                    intent.provider_id.0,
                    intent.provider_generation,
                    intent.detail
                ),
                created_at: intent.created_at,
            };
            self.outbox.append_if_absent(&mapped).map_err(|error| {
                self.fail(format!("cannot project Application AuditIntent: {error}"))
            })?;
            projected = projected.saturating_add(1);
        }
        for intent in security_intents {
            // The canonical Security intent set is keyed by
            // (operation_id, action, target): a single operation can commit
            // more than one required AuditIntent (for example a parked gate's
            // authority binding and its subsequent decision publication). The
            // outbox projection key must therefore preserve the same
            // uniqueness, otherwise two distinct canonical intents for one
            // operation collapse onto a single outbox key and restart
            // reprojection fails closed with a spurious key conflict. An
            // unambiguous length-prefixed encoding keeps distinct intents
            // separate while remaining byte-identical across restarts so
            // `append_if_absent` stays idempotent.
            let key = format!(
                "security:{}",
                length_prefixed_key(&[
                    intent.operation_id.0.as_str(),
                    intent.action.as_str(),
                    intent.target.as_str(),
                ])
            );
            let mapped = AuditIntent {
                key,
                source: "security".to_owned(),
                operation_id: Some(intent.operation_id),
                principal_ref: Some(intent.principal_ref),
                action: intent.action,
                target_ref: intent.target,
                detail: "required security AuditIntent committed".to_owned(),
                created_at: intent.created_at,
            };
            self.outbox.append_if_absent(&mapped).map_err(|error| {
                self.fail(format!("cannot project Security AuditIntent: {error}"))
            })?;
            projected = projected.saturating_add(1);
        }
        self.healthy.store(true, Ordering::Release);
        if let Ok(mut error) = self.last_error.lock() {
            *error = None;
        }
        Ok(projected)
    }

    pub(crate) fn is_healthy(&self) -> bool {
        self.healthy.load(Ordering::Acquire)
    }

    pub(crate) fn record_count(&self) -> Result<usize, String> {
        self.outbox
            .record_count()
            .map_err(|error| error.to_string())
    }

    fn fail(&self, message: String) -> String {
        self.healthy.store(false, Ordering::Release);
        if let Ok(mut error) = self.last_error.lock() {
            *error = Some(message.clone());
        }
        message
    }
}

fn phase_name(phase: ExecutionAuditPhase) -> &'static str {
    match phase {
        ExecutionAuditPhase::Admitted => "execution-admitted",
        ExecutionAuditPhase::Running => "execution-running",
        ExecutionAuditPhase::Dispatched => "provider-dispatched",
        ExecutionAuditPhase::Succeeded => "execution-succeeded",
        ExecutionAuditPhase::Failed => "execution-failed",
        ExecutionAuditPhase::Cancelled => "execution-cancelled",
        ExecutionAuditPhase::RecoveryRequired => "execution-recovery-required",
    }
}

/// Unambiguous, restart-stable encoding for a composite outbox projection key.
/// Each part is prefixed with its byte length so no combination of parts can be
/// confused with another (mirrors the canonical Security intent key encoding).
fn length_prefixed_key(parts: &[&str]) -> String {
    let mut key = String::new();
    for part in parts {
        key.push_str(&part.len().to_string());
        key.push(':');
        key.push_str(part);
    }
    key
}

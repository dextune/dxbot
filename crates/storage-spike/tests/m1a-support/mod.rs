use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use storage_spike::{OperationEffect, OperationRequest};

static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(0);

pub fn database_path(name: &str) -> PathBuf {
    let sequence = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "dxbot-storage-spike-{}-{name}-{sequence}.db",
        std::process::id()
    ))
}

pub const fn request<'a>(
    principal_ref: &'a str,
    key_digest: &'a str,
    command_id: &'a str,
    request_digest: &'a str,
    new_operation_id: &'a str,
) -> OperationRequest<'a> {
    OperationRequest {
        principal_ref,
        idempotency_key_principal_ref: principal_ref,
        idempotency_key_digest: key_digest,
        command_id,
        request_digest,
        new_operation_id,
        key_expires_at: 100,
        now: 10,
    }
}

pub const fn effect() -> OperationEffect<'static> {
    effect_for("bot-1", "state-v1")
}

pub const fn effect_for<'a>(aggregate_id: &'a str, state_value: &'a str) -> OperationEffect<'a> {
    OperationEffect {
        aggregate_id,
        state_value,
        event_payload: "event-v1",
        outbox_payload: "outbox-v1",
        audit_payload: "audit-v1",
        resolved_binding_digest: "binding-v1",
        result_ref: "result-1",
    }
}

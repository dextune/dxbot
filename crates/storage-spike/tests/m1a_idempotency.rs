use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use storage_spike::{
    OperationArtifactCounts, OperationEffect, OperationRequest, ReceiptDisposition, ReferenceStore,
    SpikeError, SubmitOutcome,
};

static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(0);

fn test_database_path(name: &str) -> PathBuf {
    let sequence = NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "dxbot-storage-spike-{}-{name}-{sequence}.db",
        std::process::id()
    ))
}

const fn request<'a>(
    principal_ref: &'a str,
    key_digest: &'a str,
    command_id: &'a str,
    request_digest: &'a str,
    new_operation_id: &'a str,
) -> OperationRequest<'a> {
    OperationRequest {
        principal_ref,
        idempotency_key_digest: key_digest,
        command_id,
        request_digest,
        new_operation_id,
        key_expires_at: 100,
        now: 10,
    }
}

const fn effect() -> OperationEffect<'static> {
    OperationEffect {
        aggregate_id: "bot-1",
        state_value: "state-v1",
        event_payload: "event-v1",
        outbox_payload: "outbox-v1",
        audit_payload: "audit-v1",
        resolved_binding_digest: "binding-v1",
        result_ref: "result-1",
    }
}

#[test]
fn atomic_submission_writes_all_required_artifacts() -> Result<(), SpikeError> {
    let path = test_database_path("atomic-submission");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let operation = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );

    assert_eq!(
        store.submit(&operation, &effect())?,
        SubmitOutcome::Created {
            operation_id: "operation-a".to_owned()
        }
    );
    assert_eq!(
        store.operation_artifact_counts(&operation, "operation-a")?,
        OperationArtifactCounts {
            aggregate_state: 1,
            event: 1,
            outbox: 1,
            receipt: 1,
            command_binding: 1,
            principal_binding: 1,
            audit_intent: 1,
        }
    );
    Ok(())
}

#[test]
fn idempotency_same_key_same_digest_returns_existing_operation() -> Result<(), SpikeError> {
    let path = test_database_path("same-key-same-digest");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let first = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    let retry = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-b",
    );

    assert!(matches!(
        store.submit(&first, &effect())?,
        SubmitOutcome::Created { .. }
    ));
    assert_eq!(
        store.submit(&retry, &effect())?,
        SubmitOutcome::Existing {
            operation_id: "operation-a".to_owned(),
            disposition: ReceiptDisposition::Committed,
            compacted: false,
        }
    );
    assert_eq!(store.operation_count()?, 1);
    Ok(())
}

#[test]
fn idempotency_same_key_different_digest_conflicts_without_creation() -> Result<(), SpikeError> {
    let path = test_database_path("same-key-different-digest");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let first = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    let conflicting = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-b",
        "operation-b",
    );

    store.submit(&first, &effect())?;
    assert!(matches!(
        store.submit(&conflicting, &effect()),
        Err(SpikeError::IdempotencyConflict)
    ));
    assert_eq!(store.operation_count()?, 1);
    Ok(())
}

#[test]
fn idempotency_command_reuse_across_principals_conflicts() -> Result<(), SpikeError> {
    let path = test_database_path("same-command-different-principal");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let first = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    let conflicting = request(
        "principal-b",
        "key-b",
        "command-a",
        "request-a",
        "operation-b",
    );

    store.submit(&first, &effect())?;
    assert!(matches!(
        store.submit(&conflicting, &effect()),
        Err(SpikeError::IdempotencyConflict)
    ));
    assert_eq!(store.operation_count()?, 1);
    Ok(())
}

#[test]
fn idempotency_principal_key_reuse_with_new_command_conflicts() -> Result<(), SpikeError> {
    let path = test_database_path("same-key-different-command");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let first = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    let conflicting = request(
        "principal-a",
        "key-a",
        "command-b",
        "request-a",
        "operation-b",
    );

    store.submit(&first, &effect())?;
    assert!(matches!(
        store.submit(&conflicting, &effect()),
        Err(SpikeError::IdempotencyConflict)
    ));
    assert_eq!(store.operation_count()?, 1);
    Ok(())
}

#[test]
fn idempotency_expired_key_never_creates_operation() -> Result<(), SpikeError> {
    let path = test_database_path("expired-key");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let mut expired = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    expired.now = 101;

    assert_eq!(
        store.submit(&expired, &effect())?,
        SubmitOutcome::IdempotencyExpired
    );
    assert_eq!(store.operation_count()?, 0);
    Ok(())
}

#[test]
fn stale_writer_generation_is_fenced() -> Result<(), SpikeError> {
    let path = test_database_path("writer-fence");
    let mut stale_store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let _active_store = ReferenceStore::open_file(&path, "instance-1", 2)?;
    let operation = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );

    assert!(matches!(
        stale_store.submit(&operation, &effect()),
        Err(SpikeError::StaleWriter)
    ));
    Ok(())
}

#[test]
fn schema_version_is_migrated_from_empty_file() -> Result<(), SpikeError> {
    let path = test_database_path("schema-version");
    let store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    assert_eq!(store.schema_version()?, 1);
    Ok(())
}

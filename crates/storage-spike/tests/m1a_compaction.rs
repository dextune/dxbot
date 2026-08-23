#[path = "m1a-support/mod.rs"]
mod m1a_fixture;

use storage_spike::{
    OperationArtifactCounts, ReceiptDisposition, ReferenceStore, SpikeError, SubmitOutcome,
};

use m1a_fixture::{database_path, effect, request};

#[test]
fn compacted_receipt_keeps_two_binding_tombstones() -> Result<(), SpikeError> {
    let path = database_path("receipt-compaction");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let operation = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    store.submit(&operation, &effect())?;

    assert!(store.compact_terminal_receipt("operation-a")?);
    assert_eq!(
        store.operation_artifact_counts(&operation, "operation-a")?,
        OperationArtifactCounts {
            aggregate_state: 1,
            event: 1,
            outbox: 1,
            receipt: 0,
            command_binding: 1,
            principal_binding: 1,
            audit_intent: 1,
        }
    );

    let retry = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-b",
    );
    assert_eq!(
        store.submit(&retry, &effect())?,
        SubmitOutcome::Existing {
            operation_id: "operation-a".to_owned(),
            disposition: ReceiptDisposition::Committed,
            compacted: true,
        }
    );
    assert_eq!(store.operation_count()?, 1);
    Ok(())
}

#[test]
fn expired_stored_tombstone_is_never_reinterpreted_as_absent() -> Result<(), SpikeError> {
    let path = database_path("expired-tombstone");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let operation = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    store.submit(&operation, &effect())?;
    assert!(store.compact_terminal_receipt("operation-a")?);

    let mut expired_retry = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-b",
    );
    expired_retry.now = 101;
    expired_retry.key_expires_at = 200;

    assert_eq!(
        store.submit(&expired_retry, &effect())?,
        SubmitOutcome::IdempotencyExpired
    );
    assert_eq!(store.operation_count()?, 1);
    Ok(())
}

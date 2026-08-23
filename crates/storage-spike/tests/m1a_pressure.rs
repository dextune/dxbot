#[path = "m1a-support/mod.rs"]
mod m1a_fixture;

use storage_spike::{
    OperationArtifactCounts, OperationEffect, ReceiptDisposition, ReferenceStore, SpikeError,
    SubmitOutcome,
};

use m1a_fixture::{database_path, effect_for, request};

#[test]
fn runtime_profile_applies_wal_and_temp_bounds() -> Result<(), SpikeError> {
    let path = database_path("storage-profile");
    let store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let profile = store.storage_runtime_profile()?;

    assert_eq!(profile.journal_mode, "wal");
    assert_eq!(profile.synchronous, 2);
    assert_eq!(profile.wal_autocheckpoint_pages, 64);
    assert_eq!(profile.journal_size_limit_bytes, 262_144);
    assert!(profile.temp_store_memory);
    Ok(())
}

#[test]
fn disk_full_preserves_committed_operation_and_rejects_new_work() -> Result<(), SpikeError> {
    let path = database_path("disk-full");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let committed = request(
        "principal-a",
        "key-committed",
        "command-committed",
        "request-committed",
        "operation-committed",
    );
    assert!(matches!(
        store.submit(&committed, &effect_for("bot-1", "state-v1"))?,
        SubmitOutcome::Created { .. }
    ));

    let required_payload_bytes = store.constrain_database_for_disk_full_fixture()?;
    let payload = "x".repeat(required_payload_bytes);
    let rejected = request(
        "principal-a",
        "key-rejected",
        "command-rejected",
        "request-rejected",
        "operation-rejected",
    );
    let rejected_effect = OperationEffect {
        aggregate_id: "bot-pressure",
        state_value: &payload,
        event_payload: &payload,
        outbox_payload: &payload,
        audit_payload: &payload,
        resolved_binding_digest: "binding-pressure",
        result_ref: "result-pressure",
    };

    assert!(matches!(
        store.submit(&rejected, &rejected_effect),
        Err(SpikeError::StorageFull)
    ));
    drop(store);

    let mut reopened = ReferenceStore::open_file(&path, "instance-1", 1)?;
    let committed_retry = request(
        "principal-a",
        "key-committed",
        "command-committed",
        "request-committed",
        "operation-retry",
    );
    assert_eq!(
        reopened.submit(&committed_retry, &effect_for("bot-1", "state-v1"))?,
        SubmitOutcome::Existing {
            operation_id: "operation-committed".to_owned(),
            disposition: ReceiptDisposition::Committed,
            compacted: false,
        }
    );
    assert_eq!(reopened.operation_count()?, 1);
    assert_eq!(
        reopened.operation_artifact_counts(&rejected, "operation-rejected")?,
        OperationArtifactCounts {
            aggregate_state: 0,
            event: 0,
            outbox: 0,
            receipt: 0,
            command_binding: 0,
            principal_binding: 0,
            audit_intent: 0,
        }
    );
    Ok(())
}

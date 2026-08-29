#[path = "m1a-support/mod.rs"]
mod m1a_fixture;

use std::error::Error;
use std::process::Command;

use storage_spike::{OperationArtifactCounts, ReceiptDisposition, ReferenceStore, SubmitOutcome};

use m1a_fixture::{database_path, effect_for, request};

#[test]
fn crash_before_commit_leaves_no_partial_operation() -> Result<(), Box<dyn Error>> {
    let path = database_path("crash-before-commit");
    let status = Command::new(env!("CARGO_BIN_EXE_storage-crash-probe"))
        .arg(&path)
        .arg("before-commit")
        .status()?;
    assert!(!status.success());

    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    assert_eq!(store.operation_count()?, 0);

    let operation = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-a",
    );
    assert_eq!(
        store.submit(&operation, &effect_for("bot-1", "state-v1"))?,
        SubmitOutcome::Created {
            operation_id: "operation-a".to_owned()
        }
    );
    Ok(())
}

#[test]
fn crash_after_commit_recovers_existing_operation() -> Result<(), Box<dyn Error>> {
    let path = database_path("crash-after-commit");
    let status = Command::new(env!("CARGO_BIN_EXE_storage-crash-probe"))
        .arg(&path)
        .arg("after-commit")
        .status()?;
    assert!(!status.success());

    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    assert_eq!(store.operation_count()?, 1);

    let retry = request(
        "principal-a",
        "key-a",
        "command-a",
        "request-a",
        "operation-b",
    );
    assert_eq!(
        store.submit(&retry, &effect_for("bot-1", "state-v1"))?,
        SubmitOutcome::Existing {
            operation_id: "operation-a".to_owned(),
            disposition: ReceiptDisposition::Committed,
            compacted: false,
        }
    );
    assert_eq!(
        store.operation_artifact_counts(&retry, "operation-a")?,
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

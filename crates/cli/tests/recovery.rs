//! Acceptance coverage for `AT-CLI-RECOVERY-001`: fresh-invocation digest
//! comparison, durable-entry validation, binding lookup, replay projection,
//! corruption refusal, and retention-safe pruning.
#![allow(clippy::unwrap_used)]

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cli::journal::{LocalJournal, PrunePolicy};
use cli::recovery::{BindingKey, RecoveryAction, RecoveryError, RecoveryManager};
use dxbot_core::types::*;
use dxbot_core::{ReceiptDisposition, ReceiptRecord};

struct TempDir(PathBuf);
impl TempDir {
    fn as_path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_base() -> TempDir {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "dxbot-recovery-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

fn instance(name: &str) -> InstanceId {
    InstanceId(name.to_owned())
}

fn prepared_record(
    instance_id: &InstanceId,
    command_id: &str,
    key_digest: &str,
    request_digest: &str,
) -> JournalRecord {
    JournalRecord {
        state: JournalState::Prepared,
        instance_id: instance_id.clone(),
        command_id: CommandId(command_id.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("test-principal".to_owned()),
            key_digest: key_digest.to_owned(),
            expires_at: 0,
        },
        request_digest: RequestDigest(request_digest.to_owned()),
        sequence: 0,
        previous_digest: String::new(),
        record_digest: String::new(),
    }
}

fn result(
    instance_id: &str,
    command_id: &str,
    request_digest: &str,
) -> OperationResult {
    let operation_id = OperationId(format!("op-{command_id}"));
    OperationResult {
        operation_id: operation_id.clone(),
        command_id: CommandId(command_id.to_owned()),
        instance_id: InstanceId(instance_id.to_owned()),
        receipt: ReceiptRecord {
            operation_id: operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: operation_id.0,
            resolved_binding_digest: request_digest.to_owned(),
            owner_kind: "instance".to_owned(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: "at-least-once".to_owned(),
        },
        status: "committed".to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}

fn existing_binding(_key: &BindingKey) -> Result<Option<OperationResult>, RecoveryError> {
    Ok(Some(result("i3", "cmd-existing", "req-existing")))
}

fn mismatched_binding(_key: &BindingKey) -> Result<Option<OperationResult>, RecoveryError> {
    Ok(Some(result("wrong-instance", "wrong-command", "wrong-digest")))
}

fn set_mtime(path: &Path, time: SystemTime) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

fn prepare_on_disk(
    base: &Path,
    instance_id: &InstanceId,
    command_id: &str,
    request_digest: &str,
) {
    let mut journal = LocalJournal::open(instance_id.clone(), base).unwrap();
    journal
        .append_prepared(&prepared_record(
            instance_id,
            command_id,
            "key",
            request_digest,
        ))
        .unwrap();
}

fn dispatch_on_disk(
    base: &Path,
    instance_id: &InstanceId,
    command_id: &str,
    request_digest: &str,
) -> IdempotencyKey {
    let record = prepared_record(instance_id, command_id, "key", request_digest);
    let key = record.idempotency_key.clone();
    let mut journal = LocalJournal::open(instance_id.clone(), base).unwrap();
    journal.append_prepared(&record).unwrap();
    journal
        .dispatch(&CommandId(command_id.to_owned()))
        .unwrap();
    key
}

#[test]
fn recovery_prepared_reuse_compares_fresh_invocation_digest() {
    let base = temp_base();
    let inst = instance("i1");
    prepare_on_disk(base.as_path(), &inst, "cmd-same", "req-a");

    let recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst, base.as_path()).unwrap(),
    )));
    let entry = recovery.scan_journal().unwrap().remove(0);

    assert!(matches!(
        recovery
            .reuse_prepared(&entry, &RequestDigest("req-a".to_owned()))
            .unwrap(),
        RecoveryAction::Continue { .. }
    ));

    // Different fresh semantics must not reuse an old Prepared record merely
    // because the stored projection equals the stored record.
    assert!(matches!(
        recovery
            .reuse_prepared(&entry, &RequestDigest("req-b".to_owned()))
            .unwrap(),
        RecoveryAction::Abandon
    ));
}

#[test]
fn recovery_stale_different_prepared_becomes_prune_eligible() {
    let base = temp_base();
    let inst = instance("i2");
    prepare_on_disk(base.as_path(), &inst, "cmd-stale", "req-a");

    let path = base
        .as_path()
        .join(&inst.0)
        .join("operations")
        .join("cmd-stale.jsonl");
    set_mtime(
        &path,
        SystemTime::now() - Duration::from_secs(7200),
    );

    let recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst, base.as_path()).unwrap(),
    )));
    let entry = recovery.scan_journal().unwrap().remove(0);
    assert!(matches!(
        recovery
            .reuse_prepared(&entry, &RequestDigest("req-b".to_owned()))
            .unwrap(),
        RecoveryAction::Prune
    ));
}

#[test]
fn recovery_dispatching_validates_returned_binding_identity() {
    let base = temp_base();
    let inst = instance("i3");
    dispatch_on_disk(
        base.as_path(),
        &inst,
        "cmd-existing",
        "req-existing",
    );
    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), base.as_path()).unwrap(),
    ));
    let entry = RecoveryManager::new(Arc::clone(&journal))
        .scan_journal()
        .unwrap()
        .remove(0);

    let recovery = RecoveryManager::with_binding_lookup(Arc::clone(&journal), existing_binding);
    assert!(matches!(
        recovery.lookup_dispatching(&entry).unwrap(),
        RecoveryAction::Resolved { .. }
    ));

    let recovery = RecoveryManager::with_binding_lookup(journal, mismatched_binding);
    assert!(matches!(
        recovery.lookup_dispatching(&entry),
        Err(RecoveryError::BindingConflict(_))
    ));
}

#[test]
fn recovery_dispatching_absent_binding_allows_same_local_binding_replay() {
    let base = temp_base();
    let inst = instance("i4");
    dispatch_on_disk(base.as_path(), &inst, "cmd-replay", "req-replay");

    let recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst, base.as_path()).unwrap(),
    )));
    let entry = recovery.scan_journal().unwrap().remove(0);
    match recovery.lookup_dispatching(&entry).unwrap() {
        RecoveryAction::Replay { ids } => {
            assert_eq!(ids.command_id.0, "cmd-replay");
            assert_eq!(ids.idempotency_key.key_digest, "key");
        }
        other => panic!("expected Replay, got {other:?}"),
    }
}

#[test]
fn recovery_replay_rejects_wrong_idempotency_key() {
    let base = temp_base();
    let inst = instance("i5");
    let key = dispatch_on_disk(base.as_path(), &inst, "cmd-key", "req-key");
    let recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst, base.as_path()).unwrap(),
    )));
    let command = CommandId("cmd-key".to_owned());

    let mut wrong = key.clone();
    wrong.key_digest = "wrong".to_owned();
    assert!(matches!(
        recovery.replay_operation(&command, &wrong),
        Err(RecoveryError::BindingConflict(_))
    ));

    let projected = recovery.replay_operation(&command, &key).unwrap();
    assert!(projected.operation_may_continue);
    assert_eq!(projected.receipt.resolved_binding_digest, "req-key");
}

#[test]
fn recovery_corrupt_journal_refuses_auto_replay() {
    let base = temp_base();
    let inst = instance("i6");
    let key = dispatch_on_disk(base.as_path(), &inst, "cmd-corrupt", "req-corrupt");
    let path = base
        .as_path()
        .join(&inst.0)
        .join("operations")
        .join("cmd-corrupt.jsonl");

    let content = fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let separator = "\"record_digest\":\"";
    let position = lines[0].find(separator).unwrap() + separator.len();
    lines[0].replace_range(position..position + 1, "0");
    fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();

    let recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst, base.as_path()).unwrap(),
    )));
    assert!(matches!(
        recovery.scan_journal(),
        Err(RecoveryError::ChainMismatch(_))
    ));
    assert!(recovery
        .replay_operation(&CommandId("cmd-corrupt".to_owned()), &key)
        .is_err());
    assert!(path.exists());
}

#[test]
fn recovery_pruning_respects_retention_before_capacity() {
    let base = temp_base();
    let inst = instance("i7");
    let operations = base.as_path().join(&inst.0).join("operations");

    for index in 0..4 {
        prepare_on_disk(
            base.as_path(),
            &inst,
            &format!("prep-{index}"),
            "req",
        );
    }
    let now = SystemTime::now();
    for index in 0..3 {
        set_mtime(
            &operations.join(format!("prep-{index}.jsonl")),
            now - Duration::from_secs(7200),
        );
    }

    let mut recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), base.as_path()).unwrap(),
    )));

    let no_pressure = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1000,
    };
    assert_eq!(recovery.prune_stale_prepared(&no_pressure).unwrap(), 0);

    let pressure = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1,
    };
    assert_eq!(recovery.prune_stale_prepared(&pressure).unwrap(), 3);
    assert_eq!(recovery.scan_journal().unwrap().len(), 1);

    for index in 5..7 {
        prepare_on_disk(
            base.as_path(),
            &inst,
            &format!("prep-{index}"),
            "req",
        );
    }
    let fresh_pressure = PrunePolicy {
        min_retention_seconds: 3600,
        max_prepared_count: 1,
    };
    assert_eq!(recovery.prune_stale_prepared(&fresh_pressure).unwrap(), 0);
    assert_eq!(recovery.scan_journal().unwrap().len(), 3);
}

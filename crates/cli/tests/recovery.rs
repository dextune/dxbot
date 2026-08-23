//! Acceptance coverage for `AT-CLI-RECOVERY-001`: bounded journal scan,
//! prepared reuse, dispatching binding lookup, replay projection, and stale
//! prepared pruning — including the `operation_may_continue` user contract and
//! the corrupt-journal no-auto-replay rule.
#![allow(clippy::unwrap_used)]

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cli::journal::{LocalJournal, PrunePolicy};
use cli::recovery::{
    BindingKey, RecoveryAction, RecoveryEntry, RecoveryError, RecoveryManager,
};
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
    InstanceId(name.to_string())
}

fn prepared_record(
    instance_id: &InstanceId,
    command_id: &str,
    key_digest: &str,
    req_digest: &str,
) -> JournalRecord {
    JournalRecord {
        state: JournalState::Prepared,
        instance_id: instance_id.clone(),
        command_id: CommandId(command_id.to_string()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("test-principal".to_string()),
            key_digest: key_digest.to_string(),
            expires_at: 0,
        },
        request_digest: RequestDigest(req_digest.to_string()),
        sequence: 1,
        previous_digest: String::new(),
        record_digest: String::new(),
    }
}

/// A fully synthetic Prepared entry (no disk state required for the same-digest
/// reuse decision, which never touches the journal).
fn prepared_entry(instance_id: &InstanceId, command_id: &str, req_digest: &str) -> RecoveryEntry {
    let rec = prepared_record(instance_id, command_id, "key", req_digest);
    RecoveryEntry {
        command_id: rec.command_id.clone(),
        state: JournalState::Prepared,
        semantic_digest: rec.request_digest.0.clone(),
        records: vec![rec],
    }
}

/// A fully synthetic Dispatching entry (state + ids derived from its records).
fn dispatching_entry(
    instance_id: &InstanceId,
    command_id: &str,
) -> RecoveryEntry {
    let rec = prepared_record(instance_id, command_id, "key", "req");
    RecoveryEntry {
        command_id: rec.command_id.clone(),
        state: JournalState::Dispatching,
        semantic_digest: rec.request_digest.0.clone(),
        records: vec![rec],
    }
}

fn committed_result(command_id: &str) -> OperationResult {
    let operation_id = OperationId(format!("op-{command_id}"));
    OperationResult {
        operation_id: operation_id.clone(),
        command_id: CommandId(command_id.to_string()),
        instance_id: InstanceId("i".to_string()),
        receipt: ReceiptRecord {
            operation_id: operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: operation_id.0.clone(),
            resolved_binding_digest: String::new(),
            owner_kind: String::new(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: String::new(),
        },
        status: "committed".to_string(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}

/// An injected server binding lookup that finds an existing committed result.
fn existing_binding_lookup(_key: &BindingKey) -> Result<Option<OperationResult>, RecoveryError> {
    Ok(Some(committed_result("cmd-existing")))
}

fn set_mtime(path: &Path, time: SystemTime) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

/// A prepared entry with a same semantic digest is reused (Continue) with the
/// original IDs.
#[test]
fn recovery_prepared_reuse_with_same_digest_returns_continue() {
    let base = temp_base();
    let inst = instance("i1");

    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), base.as_path()).unwrap(),
    ));
    let recovery = RecoveryManager::new(journal);

    let entry = prepared_entry(&inst, "cmd-same", "req-A");
    match recovery.reuse_prepared(&entry).unwrap() {
        RecoveryAction::Continue { ids } => {
            assert_eq!(ids.command_id, CommandId("cmd-same".to_string()));
            assert_eq!(ids.idempotency_key.key_digest, "key");
            assert!(!ids.operation_id.0.is_empty());
        }
        other => panic!("expected Continue, got {other:?}"),
    }
}

/// A prepared entry carrying a stale/different digest is Pruned when the record
/// is lock-free, integrity-valid and retention-eligible.
#[test]
fn recovery_prepared_stale_digest_returns_prune_when_eligible() {
    let base = temp_base();
    let base_path = base.as_path().to_path_buf();
    let inst = instance("i2");

    // One provably-unsent Prepared command, writer exits (releases the lock).
    {
        let mut writer = LocalJournal::open(inst.clone(), &base_path).unwrap();
        writer
            .append_prepared(&prepared_record(&inst, "cmd-stale", "key", "req-A"))
            .unwrap();
    }
    // Age it past the default retention bound.
    let ops = base_path.join(inst.0.clone()).join("operations");
    set_mtime(
        &ops.join("cmd-stale.jsonl"),
        SystemTime::now() - Duration::from_secs(3600),
    );

    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), &base_path).unwrap(),
    ));
    let recovery = RecoveryManager::new(journal);

    // A later invocation has a different semantic digest.
    let entries = recovery.scan_journal().unwrap();
    let mut entry = entries
        .into_iter()
        .find(|e| e.command_id.0 == "cmd-stale")
        .unwrap();
    entry.semantic_digest = "req-DIFFERENT".to_string();

    match recovery.reuse_prepared(&entry).unwrap() {
        RecoveryAction::Prune => {}
        other => panic!("expected Prune, got {other:?}"),
    }
}

/// A Dispatching entry whose server binding lookup finds a committed result is
/// Resolved with that result (no replay).
#[test]
fn recovery_dispatching_lookup_finds_existing_result() {
    let base = temp_base();
    let inst = instance("i3");

    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), base.as_path()).unwrap(),
    ));
    let recovery = RecoveryManager::with_binding_lookup(journal, existing_binding_lookup);

    let entry = dispatching_entry(&inst, "cmd-existing");
    match recovery.lookup_dispatching(&entry).unwrap() {
        RecoveryAction::Resolved { result } => {
            assert_eq!(result.command_id, entry.command_id);
            assert_eq!(result.status, "committed");
            assert!(result.operation_may_continue);
            assert!(!result.operation_id.0.is_empty(), "must carry an OperationRef");
        }
        other => panic!("expected Resolved, got {other:?}"),
    }
}

/// A Dispatching entry whose binding lookup is absent allows replay with the
/// same IDs.
#[test]
fn recovery_dispatching_not_found_allows_replay() {
    let base = temp_base();
    let inst = instance("i4");

    // Default binding lookup reports an absent binding.
    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), base.as_path()).unwrap(),
    ));
    let recovery = RecoveryManager::new(journal);

    let entry = dispatching_entry(&inst, "cmd-replay");
    match recovery.lookup_dispatching(&entry).unwrap() {
        RecoveryAction::Replay { ids } => {
            assert_eq!(ids.command_id, CommandId("cmd-replay".to_string()));
            assert_eq!(ids.idempotency_key.key_digest, "key");
            assert!(!ids.operation_id.0.is_empty());
        }
        other => panic!("expected Replay, got {other:?}"),
    }
}

/// Replaying a Dispatching entry yields `operation_may_continue = true` and an
/// OperationRef, so a local observation end is never presented as a Runtime
/// cancel.
#[test]
fn recovery_operation_may_continue_flag_is_set() {
    let base = temp_base();
    let inst = instance("i5");

    // A real Dispatching command on disk so replay_operation can read it.
    let cmd = CommandId("cmd-continue".to_string());
    let key_digest = "key-continue";
    let req_digest = "req-continue";
    let prep = prepared_record(&inst, &cmd.0, key_digest, req_digest);
    {
        let mut writer = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
        writer.append_prepared(&prep).unwrap();
        writer.dispatch(&cmd).unwrap();
    }

    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), base.as_path()).unwrap(),
    ));
    let recovery = RecoveryManager::new(journal);

    let result = recovery
        .replay_operation(&cmd, &prep.idempotency_key)
        .unwrap();
    assert!(
        result.operation_may_continue,
        "a Dispatching result must be operation_may_continue=true"
    );
    assert!(!result.operation_id.0.is_empty(), "must carry an OperationRef");
    assert_eq!(result.command_id, cmd);
}

/// A corrupt (chain-mismatched) journal refuses auto replay or auto delete.
#[test]
fn recovery_corrupt_journal_refuses_auto_replay() {
    let base = temp_base();
    let inst = instance("i6");

    let cmd = CommandId("cmd-corrupt".to_string());
    let prep = prepared_record(&inst, &cmd.0, "key-corrupt", "req-corrupt");
    let mut writer = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    writer.append_prepared(&prep).unwrap();
    writer.dispatch(&cmd).unwrap();
    let ops = writer.dir().to_path_buf();
    drop(writer);

    // Tamper with the first record's digest so chain validation fails closed.
    let path = ops.join("cmd-corrupt.jsonl");
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    let first = lines[0];
    let sep = "\"record_digest\":\"";
    let pos = first.find(sep).unwrap() + sep.len();
    let mut chars: Vec<char> = first.chars().collect();
    chars[pos] = if chars[pos] == 'a' { 'b' } else { 'a' };
    let mut rewritten = chars.into_iter().collect::<String>();
    for line in lines.iter().skip(1) {
        rewritten.push('\n');
        rewritten.push_str(line);
    }
    rewritten.push('\n');
    fs::write(&path, rewritten).unwrap();

    let base_path = base.as_path().to_path_buf();
    let journal = Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), &base_path).unwrap(),
    ));
    let recovery = RecoveryManager::new(journal);

    // The bounded startup scan fails rather than surfacing a corrupt entry.
    assert!(
        matches!(
            recovery.scan_journal(),
            Err(RecoveryError::ChainMismatch(_))
        ),
        "corrupt chain must fail the bounded scan, never auto-replay"
    );

    // Replay is refused too.
    assert!(recovery.replay_operation(&cmd, &prep.idempotency_key).is_err());

    // And nothing was auto-deleted.
    assert!(
        path.exists(),
        "corrupt journal must not be auto-deleted"
    );
}

/// Pruning stale Prepared entries under a `PrunePolicy` respects retention and
/// quota.
#[test]
fn recovery_prune_stale_prepared_respects_retention() {
    let base = temp_base();
    let base_path = base.as_path().to_path_buf();
    let inst = instance("i7");

    // Several Prepared commands, each writer exits (lock released).
    for i in 0..4 {
        let name = format!("prep-{i}");
        let mut w = LocalJournal::open(inst.clone(), &base_path).unwrap();
        w.append_prepared(&prepared_record(&inst, &name, "key", "req")).unwrap();
    }
    let ops = base_path.join(inst.0.clone()).join("operations");
    let now = SystemTime::now();
    for i in 0..3 {
        set_mtime(
            &ops.join(format!("prep-{i}.jsonl")),
            now - Duration::from_secs(3600),
        );
    }
    set_mtime(&ops.join("prep-3.jsonl"), now);

    let mut recovery = RecoveryManager::new(Arc::new(Mutex::new(
        LocalJournal::open(inst.clone(), &base_path).unwrap(),
    )));

    // Retention: only the three stale ones are pruned; fresh `prep-3` stays.
    let policy = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1000,
    };
    let pruned = recovery.prune_stale_prepared(&policy).unwrap();
    assert_eq!(pruned, 3);
    assert!(ops.join("prep-3.jsonl").exists());
    for i in 0..3 {
        assert!(!ops.join(format!("prep-{i}.jsonl")).exists());
    }
    assert_eq!(recovery.scan_journal().unwrap().len(), 1);

    // Quota: with a cap of 1 the two oldest of the three survivors are evicted
    // regardless of retention age.
    for i in 5..7 {
        let name = format!("prep-{i}");
        let mut w = LocalJournal::open(inst.clone(), &base_path).unwrap();
        w.append_prepared(&prepared_record(&inst, &name, "key", "req")).unwrap();
    }
    let quota = PrunePolicy {
        min_retention_seconds: 3600,
        max_prepared_count: 1,
    };
    let pruned = recovery.prune_stale_prepared(&quota).unwrap();
    assert_eq!(pruned, 2);
    let remaining = recovery.scan_journal().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].command_id.0, "prep-6");
}
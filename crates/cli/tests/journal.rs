//! Acceptance coverage for `AT-JOURNAL-001`: single-writer ownership, hash
//! chain integrity, state transitions, takeover, and retention-safe capacity.
#![allow(clippy::unwrap_used)]

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cli::journal::{JournalError, LocalJournal, PrunePolicy};
use dxbot_core::types::*;
use dxbot_core::{ReceiptDisposition, ReceiptRecord};

const ZERO_HASH_HEX: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

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
        "dxbot-journal-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

fn instance(name: &str) -> InstanceId {
    InstanceId(name.to_owned())
}

fn prepared_record(instance_id: &InstanceId, command_id: &str) -> JournalRecord {
    JournalRecord {
        state: JournalState::Prepared,
        instance_id: instance_id.clone(),
        command_id: CommandId(command_id.to_owned()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("test-principal".to_owned()),
            key_digest: format!("key-{command_id}"),
            expires_at: 0,
        },
        request_digest: RequestDigest(format!("req-{command_id}")),
        sequence: 0,
        previous_digest: String::new(),
        record_digest: String::new(),
    }
}

fn make_result(instance_id: &InstanceId, command_id: &CommandId) -> OperationResult {
    let operation_id = OperationId(format!("op-{}", command_id.0));
    OperationResult {
        operation_id: operation_id.clone(),
        command_id: command_id.clone(),
        instance_id: instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: operation_id.0,
            disposition: ReceiptDisposition::Accepted,
            result_ref: String::new(),
            resolved_binding_digest: String::new(),
            owner_kind: String::new(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: String::new(),
        },
        status: "accepted".to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}

fn set_mtime(path: &Path, time: SystemTime) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

#[test]
fn journal_single_writer_exclusive_create() {
    let base = temp_base();
    let inst = instance("i1");
    let record = prepared_record(&inst, "command-a");

    let mut first = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    first.append_prepared(&record).unwrap();

    let mut second = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    assert!(matches!(
        second.append_prepared(&record),
        Err(JournalError::LockBusy(_))
    ));

    drop(first);
    let mut third = LocalJournal::open(inst, base.as_path()).unwrap();
    assert!(matches!(
        third.append_prepared(&record),
        Err(JournalError::AlreadyExists(_))
    ));
}

#[test]
fn journal_hash_chain_and_transition_order_are_validated() {
    let base = temp_base();
    let inst = instance("i2");
    let cmd = CommandId("chain-command".to_owned());
    let mut journal = LocalJournal::open(inst.clone(), base.as_path()).unwrap();

    journal
        .append_prepared(&prepared_record(&inst, &cmd.0))
        .unwrap();
    assert!(matches!(
        journal.terminal(&cmd),
        Err(JournalError::InvalidTransition { .. })
    ));

    journal.dispatch(&cmd).unwrap();
    journal.observe(&cmd, &make_result(&inst, &cmd)).unwrap();
    journal.terminal(&cmd).unwrap();

    let records = journal.lookup(&cmd).unwrap().unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(records[0].previous_digest, ZERO_HASH_HEX);
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record.sequence, i64::try_from(index + 1).unwrap());
        if index > 0 {
            assert_eq!(record.previous_digest, records[index - 1].record_digest);
        }
    }

    let path = journal.dir().join("chain-command.jsonl");
    let content = fs::read_to_string(&path).unwrap();
    let mut lines: Vec<String> = content.lines().map(str::to_owned).collect();
    let separator = "\"record_digest\":\"";
    let position = lines[0].find(separator).unwrap() + separator.len();
    lines[0].replace_range(position..position + 1, "0");
    fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();

    assert!(matches!(
        journal.lookup(&cmd),
        Err(JournalError::ChainIntegrity(_))
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn journal_takeover_reclaims_only_a_dead_owner() {
    let base = temp_base();
    let inst = instance("i3");
    let cmd = CommandId("takeover-command".to_owned());

    {
        let mut writer = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
        writer
            .append_prepared(&prepared_record(&inst, &cmd.0))
            .unwrap();
    }

    let operations = base.as_path().join(&inst.0).join("operations");
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg("exit 0")
        .spawn()
        .unwrap();
    child.wait().unwrap();
    fs::write(
        operations.join(".takeover-command.lock"),
        child.id().to_string(),
    )
    .unwrap();

    let mut successor = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    successor.takeover(&cmd).unwrap();
    successor.dispatch(&cmd).unwrap();

    let mut contender = LocalJournal::open(inst, base.as_path()).unwrap();
    assert!(matches!(
        contender.takeover(&cmd),
        Err(JournalError::LockBusy(_))
    ));
}

#[test]
fn journal_capacity_never_bypasses_minimum_retention() {
    let base = temp_base();
    let inst = instance("i4");
    let operations = base.as_path().join(&inst.0).join("operations");

    for index in 0..4 {
        let name = format!("prep-{index}");
        let mut writer = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
        writer
            .append_prepared(&prepared_record(&inst, &name))
            .unwrap();
    }

    let now = SystemTime::now();
    for index in 0..3 {
        set_mtime(
            &operations.join(format!("prep-{index}.jsonl")),
            now - Duration::from_secs(7200),
        );
    }
    set_mtime(&operations.join("prep-3.jsonl"), now);

    let mut journal = LocalJournal::open(inst.clone(), base.as_path()).unwrap();

    // Minimum retention creates eligibility; it is not a TTL by itself.
    let no_pressure = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1000,
    };
    assert_eq!(journal.prune_prepared(&no_pressure).unwrap(), 0);
    assert_eq!(journal.scan().unwrap().len(), 4);

    // Under capacity pressure only the three old eligible records are removed.
    let pressure = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1,
    };
    assert_eq!(journal.prune_prepared(&pressure).unwrap(), 3);
    let remaining = journal.scan().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].command_id.0, "prep-3");

    // Fresh records can temporarily exceed the capacity target; safety wins
    // over eagerly deleting provably-unsent recovery state.
    for index in 5..7 {
        let name = format!("prep-{index}");
        let mut writer = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
        writer
            .append_prepared(&prepared_record(&inst, &name))
            .unwrap();
    }
    let fresh_pressure = PrunePolicy {
        min_retention_seconds: 3600,
        max_prepared_count: 1,
    };
    assert_eq!(journal.prune_prepared(&fresh_pressure).unwrap(), 0);
    assert_eq!(journal.scan().unwrap().len(), 3);
}

#[test]
fn journal_multi_process_uses_real_journal_path() {
    let base = temp_base();
    let helper = env!("CARGO_BIN_EXE_journal-helper");
    let command = "multi-process-command";

    let mut first = std::process::Command::new(helper)
        .arg(base.as_path())
        .arg(command)
        .arg("300")
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(50));

    let second = std::process::Command::new(helper)
        .arg(base.as_path())
        .arg(command)
        .status()
        .unwrap();
    assert_eq!(second.code(), Some(2));
    assert!(first.wait().unwrap().success());

    let third = std::process::Command::new(helper)
        .arg(base.as_path())
        .arg(command)
        .status()
        .unwrap();
    assert_eq!(third.code(), Some(2));

    let created = base
        .as_path()
        .join("multi-process")
        .join("operations")
        .join(format!("{command}.jsonl"));
    assert!(created.is_file());
}

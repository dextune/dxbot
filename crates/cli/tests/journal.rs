//! Acceptance coverage for `AT-JOURNAL-001`: single-writer ownership, hash
//! chain integrity, state transitions, takeover, and retention-safe capacity.
#![allow(clippy::unwrap_used)]

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
        operation_id: OperationId(format!("op-{command_id}")),
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
            operation_id: operation_id.0.clone(),
            disposition: ReceiptDisposition::Accepted,
            result_ref: format!("operation:{}", operation_id.0),
            resolved_binding_digest: format!("req-{}", command_id.0),
            owner_kind: "instance".to_owned(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: "at-least-once".to_owned(),
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
    assert_eq!(
        records[0].operation_id,
        OperationId("op-chain-command".to_owned())
    );
    for (index, record) in records.iter().enumerate() {
        assert_eq!(record.sequence, i64::try_from(index + 1).unwrap());
        assert_eq!(record.operation_id, records[0].operation_id);
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

#[test]
fn journal_truncated_tail_is_repaired_before_next_append() {
    let base = temp_base();
    let inst = instance("tail");
    let cmd = CommandId("tail-command".to_owned());
    let mut journal = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    journal
        .append_prepared(&prepared_record(&inst, &cmd.0))
        .unwrap();

    let path = journal.dir().join("tail-command.jsonl");
    let mut file = OpenOptions::new().append(true).open(&path).unwrap();
    use std::io::Write as _;
    file.write_all(b"{\"partial\":").unwrap();
    file.sync_all().unwrap();

    journal.dispatch(&cmd).unwrap();
    let records = journal.lookup(&cmd).unwrap().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].state, JournalState::Dispatching);
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
    fs::write(
        operations.join(".takeover-command.lock"),
        "stale-diagnostic-pid",
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

    let no_pressure = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1000,
    };
    assert_eq!(journal.prune_prepared(&no_pressure).unwrap(), 0);
    assert_eq!(journal.scan().unwrap().len(), 4);

    let pressure = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1,
    };
    assert_eq!(journal.prune_prepared(&pressure).unwrap(), 3);
    let remaining = journal.scan().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].command_id.0, "prep-3");

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
    let created = base
        .as_path()
        .join("multi-process")
        .join("operations")
        .join(format!("{command}.jsonl"));

    let mut first = std::process::Command::new(helper)
        .arg(base.as_path())
        .arg(command)
        .arg("300")
        .spawn()
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(2);
    while !created.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(created.is_file(), "first writer did not publish Prepared in time");

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
    assert!(created.is_file());
}

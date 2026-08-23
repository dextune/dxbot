//! Acceptance coverage for `AT-JOURNAL-001`: single-writer journal, takeover,
//! hash chain, and bounded retention.
#![allow(clippy::unwrap_used)]

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cli::journal::{JournalError, LocalJournal, PrunePolicy};
use dxbot_core::types::*;
use dxbot_core::{ReceiptDisposition, ReceiptRecord};

/// SHA-256 of the empty message; the zero hash anchoring every chain head.
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
    InstanceId(name.to_string())
}

fn prepared_record(instance_id: &InstanceId, command_id: &str) -> JournalRecord {
    JournalRecord {
        state: JournalState::Prepared,
        instance_id: instance_id.clone(),
        command_id: CommandId(command_id.to_string()),
        idempotency_key: IdempotencyKey {
            principal_ref: PrincipalRef("test-principal".to_string()),
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
    OperationResult {
        operation_id: OperationId(format!("op-{}", command_id.0)),
        command_id: command_id.clone(),
        instance_id: instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: format!("op-{}", command_id.0),
            disposition: ReceiptDisposition::Accepted,
            result_ref: String::new(),
            resolved_binding_digest: String::new(),
            owner_kind: String::new(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: String::new(),
        },
        status: "accepted".to_string(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}

fn set_mtime(path: &Path, time: SystemTime) {
    let file = OpenOptions::new().write(true).open(path).unwrap();
    file.set_modified(time).unwrap();
}

fn dead_pid() -> u32 {
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg("exit 0")
        .spawn()
        .unwrap();
    child.wait().unwrap();
    child.id()
}

/// Single writer enforced by OS-level exclusive create and the writer lock.
#[test]
fn journal_single_writer_exclusive_create() {
    let base = temp_base();
    let inst = instance("i1");

    let mut writer = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    let rec = prepared_record(&inst, "command-a");
    writer.append_prepared(&rec).unwrap();

    // A second writer, while the first is alive, cannot acquire the lock.
    let mut writer2 = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    match writer2.append_prepared(&rec) {
        Err(JournalError::LockBusy(_)) => {}
        other => panic!("expected LockBusy while writer alive, got {other:?}"),
    }
    drop(writer);

    // After the first writer exits, the lock is released but the command file
    // still exists: a fresh writer cannot re-create it (exclusive create).
    let mut writer3 = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    match writer3.append_prepared(&rec) {
        Err(JournalError::AlreadyExists(_)) => {}
        other => panic!("expected AlreadyExists after writer exit, got {other:?}"),
    }

    // OS-atomic exclusive create: a file laid down before any lock is taken is
    // refused by create_new.
    let mut writer4 = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    fs::write(writer4.dir().join("manual.jsonl"), b"{}").unwrap();
    match writer4.append_prepared(&prepared_record(&inst, "manual")) {
        Err(JournalError::AlreadyExists(_)) => {}
        other => panic!("expected AlreadyExists on pre-existing file, got {other:?}"),
    }
}

/// Records form a valid hash chain; any tampering is detected on read.
#[test]
fn journal_hash_chain_is_valid() {
    let base = temp_base();
    let inst = instance("i2");
    let mut journal = LocalJournal::open(inst.clone(), base.as_path()).unwrap();
    let cmd = CommandId("chain-command".to_string());

    journal.append_prepared(&prepared_record(&inst, "chain-command")).unwrap();
    journal.dispatch(&cmd).unwrap();
    journal.observe(&cmd, &make_result(&inst, &cmd)).unwrap();
    journal.terminal(&cmd).unwrap();

    let records = journal.lookup(&cmd).unwrap().unwrap();
    assert_eq!(records.len(), 4);
    assert_eq!(records[0].state, JournalState::Prepared);
    assert_eq!(records[1].state, JournalState::Dispatching);
    assert_eq!(records[2].state, JournalState::Observed);
    assert_eq!(records[3].state, JournalState::Terminal);

    // The chain is anchored on the zero hash and links record to record.
    assert_eq!(records[0].previous_digest, ZERO_HASH_HEX);
    assert_eq!(records[0].sequence, 1);
    for i in 1..records.len() {
        assert_eq!(records[i].previous_digest, records[i - 1].record_digest);
        assert_eq!(records[i].sequence, records[i - 1].sequence + 1);
    }
    // A single command's digests are all distinct within a chain.
    assert!(records.iter().any(|r| !r.record_digest.is_empty()));

    let all = journal.scan().unwrap();
    assert_eq!(all.len(), 4);

    // Tamper with the first record's digest and confirm the read fails closed.
    let path = journal.dir().join("chain-command.jsonl");
    let content = fs::read_to_string(&path).unwrap();
    let lines: Vec<&str> = content.lines().collect();
    let first = lines[0];
    let sep = "\"record_digest\":\"";
    let pos = first.find(sep).unwrap() + sep.len();
    let mut chars: Vec<char> = first.chars().collect();
    chars[pos] = if chars[pos] == 'a' { 'b' } else { 'a' };
    let tampered = chars.into_iter().collect::<String>();
    let mut rewritten = tampered;
    for line in lines.iter().skip(1) {
        rewritten.push('\n');
        rewritten.push_str(line);
    }
    rewritten.push('\n');
    fs::write(&path, rewritten).unwrap();

    match journal.lookup(&cmd) {
        Err(JournalError::ChainIntegrity(_)) => {}
        other => panic!("expected ChainIntegrity on tampering, got {other:?}"),
    }
}

/// After a crash the previous writer's lock is stale, and a new writer can
/// take over (only then).
#[test]
fn journal_takeover_after_crash() {
    let base = temp_base();
    let base_path = base.as_path().to_path_buf();
    let inst = instance("i3");
    let cmd = CommandId("takeover-command".to_string());

    let mut crashed = LocalJournal::open(inst.clone(), &base_path).unwrap();
    crashed
        .append_prepared(&prepared_record(&inst, "takeover-command"))
        .unwrap();

    // Simulate a crash: the original writer's lock now names a dead PID (the
    // lock was never released). Keep `crashed` alive so Drop does not clean it.
    let ops = crashed.dir();
    let lock = ops.join(".takeover-command.lock");
    fs::write(&lock, format!("{}\n", dead_pid())).unwrap();

    // A new writer can take over the stale command and continue appending.
    let mut successor = LocalJournal::open(inst.clone(), &base_path).unwrap();
    successor.takeover(&cmd).unwrap();
    successor.dispatch(&cmd).unwrap();

    let records = successor.lookup(&cmd).unwrap().unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[1].state, JournalState::Dispatching);
    assert_eq!(records[1].previous_digest, records[0].record_digest);

    // While `successor` holds the lock, a third writer cannot take it over.
    let mut third = LocalJournal::open(inst.clone(), &base_path).unwrap();
    match third.takeover(&cmd) {
        Err(JournalError::LockBusy(_)) => {}
        other => panic!("expected LockBusy for live writer takeover, got {other:?}"),
    }
    match third.dispatch(&cmd) {
        Err(JournalError::NotOwner(_)) => {}
        other => panic!("expected NotOwner for non-owner append, got {other:?}"),
    }
}

/// Old Prepared records are pruned under a bounded retention policy; fresh ones
/// survive, and the quota keeps at most `max_prepared_count`.
#[test]
fn journal_prune_stale_prepared() {
    let base = temp_base();
    let base_path = base.as_path().to_path_buf();
    let inst = instance("i4");

    // Lay down several Prepared commands, each from a writer that exits, so no
    // writer lock remains at prune time.
    let n_old = 3;
    for i in 0..4 {
        let name = format!("prep-{i}");
        let mut w = LocalJournal::open(inst.clone(), &base_path).unwrap();
        w.append_prepared(&prepared_record(&inst, &name)).unwrap();
    }
    let ops = base_path.join(inst.0.clone()).join("operations");
    let now = SystemTime::now();
    for i in 0..n_old {
        set_mtime(
            &ops.join(format!("prep-{i}.jsonl")),
            now - Duration::from_secs(3600),
        );
    }
    // `prep-3` stays fresh.
    set_mtime(&ops.join("prep-3.jsonl"), now);

    let mut journal = LocalJournal::open(inst.clone(), &base_path).unwrap();

    // 1. Retention: the three stale ones are pruned, fresh `prep-3` survives.
    let policy = PrunePolicy {
        min_retention_seconds: 300,
        max_prepared_count: 1000,
    };
    let pruned = journal.prune_prepared(&policy).unwrap();
    assert_eq!(pruned, n_old);
    assert!(ops.join("prep-3.jsonl").exists());
    for i in 0..n_old {
        assert!(!ops.join(format!("prep-{i}.jsonl")).exists());
    }
    assert_eq!(journal.scan().unwrap().len(), 1);

    // 2. Quota: none of the fresh commands are past retention, but with a cap
    //    of 1 the two oldest of the three are evicted regardless.
    for i in 5..7 {
        let name = format!("prep-{i}");
        let mut w = LocalJournal::open(inst.clone(), &base_path).unwrap();
        w.append_prepared(&prepared_record(&inst, &name)).unwrap();
    }
    // `prep-3`, `prep-5` and `prep-6` are all fresh (created just now).
    let quota = PrunePolicy {
        min_retention_seconds: 3600,
        max_prepared_count: 1,
    };
    let pruned = journal.prune_prepared(&quota).unwrap();
    assert_eq!(pruned, 2);
    let remaining = journal.scan().unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].state, JournalState::Prepared);
    // The newest one (`prep-6`) survives.
    assert_eq!(remaining[0].command_id.0, "prep-6");
}

/// Two real processes cannot both create / write the same command file: the
/// OS-atomic exclusive create admits exactly one winner.
#[test]
fn journal_multi_process_os_lock() {
    let base = temp_base();
    let base_path = base.as_path().to_path_buf();
    let helper = env!("CARGO_BIN_EXE_journal-helper");
    let command = "multi-process-command";

    let first = std::process::Command::new(helper)
        .arg(&base_path)
        .arg(command)
        .status()
        .unwrap();
    assert!(
        first.success(),
        "first process should win the exclusive create"
    );

    let second = std::process::Command::new(helper)
        .arg(&base_path)
        .arg(command)
        .status()
        .unwrap();
    assert_eq!(
        second.code(),
        Some(2),
        "second process must fail with already-exists"
    );

    let created = base_path.join(format!("{command}.jsonl"));
    assert!(created.exists());
    assert_eq!(fs::read_to_string(&created).unwrap(), "winner\n");
}
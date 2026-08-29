#![allow(clippy::expect_used)]

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use dxbot_core::types::{OperationId, PrincipalRef};
use runtime_audit::{AuditIntent, AuditOutbox, Error, RedactionLevel};

static SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn path(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "dxbot-audit-outbox-{}-{name}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    root.join("audit-outbox.json")
}

fn intent(key: &str, detail: &str) -> AuditIntent {
    AuditIntent {
        key: key.to_owned(),
        source: "application".to_owned(),
        operation_id: Some(OperationId("operation-1".to_owned())),
        principal_ref: Some(PrincipalRef("principal-1".to_owned())),
        action: "execution-succeeded".to_owned(),
        target_ref: "execution:1".to_owned(),
        detail: detail.to_owned(),
        created_at: 1,
    }
}

#[test]
fn outbox_survives_restart_and_exact_replay_is_idempotent() {
    let path = path("restart");
    let outbox = AuditOutbox::open(path.clone()).expect("open");
    let first = outbox
        .append_if_absent(&intent("execution:1:succeeded", "result committed"))
        .expect("append");
    let replay = outbox
        .append_if_absent(&intent("execution:1:succeeded", "result committed"))
        .expect("replay");
    assert_eq!(first, replay);
    assert_eq!(outbox.record_count().expect("count"), 1);
    drop(outbox);

    let reopened = AuditOutbox::open(path.clone()).expect("reopen");
    let records = reopened.records().expect("records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].sequence, 0);
    assert_eq!(records[0].previous_digest, "0".repeat(64));
    assert_eq!(records[0].record_digest.len(), 64);
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}

#[test]
fn outbox_redacts_before_durable_write_and_conflicting_key_fails() {
    let path = path("redaction");
    let outbox = AuditOutbox::open(path.clone()).expect("open");
    let record = outbox
        .append_if_absent(&intent(
            "execution:1:failed",
            "credential=secret-canary host=provider",
        ))
        .expect("append");
    assert_ne!(record.redaction_level, RedactionLevel::None);
    assert!(!record.intent.detail.contains("secret-canary"));
    assert!(
        !fs::read_to_string(&path)
            .expect("stored")
            .contains("secret-canary")
    );
    assert!(matches!(
        outbox.append_if_absent(&intent("execution:1:failed", "different result")),
        Err(Error::InvalidInput(_))
    ));
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}

#[test]
fn tampered_digest_chain_fails_closed_on_restart() {
    let path = path("tamper");
    let outbox = AuditOutbox::open(path.clone()).expect("open");
    outbox
        .append_if_absent(&intent("execution:1:running", "started"))
        .expect("append");
    drop(outbox);
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
    value["records"][0]["record_digest"] = serde_json::Value::String("0".repeat(64));
    fs::write(&path, serde_json::to_vec(&value).expect("encode")).expect("tamper");
    assert!(matches!(
        AuditOutbox::open(path.clone()),
        Err(Error::CorruptLog)
    ));
    let _ = fs::remove_dir_all(path.parent().expect("parent"));
}

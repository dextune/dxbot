//! Integration tests for the Runtime-Host-owned offline backup/restore
//! boundary and the Application v1→v2 startup migration.
//!
//! These exercise the operational R9 invariants directly against real files:
//! owner-only artifacts, regular-file/no-symlink validation, SHA-256 manifest
//! digests, the dry-run vs authorized apply boundary, fail-closed digest
//! corruption handling, byte-for-byte nonterminal state preservation through a
//! backup/restore cycle, and pre-migration backup + rollback rehearsal.

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use runtime_host::offline_storage::{OfflineStorage, OfflineStorageError, RestoreMode};
use runtime_host::state_migration::{MigrationOutcome, migrate_application_state};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let seq = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let base =
        std::env::temp_dir().join(format!("dxbot-offline-{}-{name}-{seq}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).expect("temp base");
    base
}

fn write_owner_only(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("write artifact");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("chmod artifact");
}

/// A representative nonterminal Application snapshot at version 2 with a task in
/// a nonterminal recovery state and a nonterminal process.
fn nonterminal_application_snapshot_v2() -> Vec<u8> {
    let value = serde_json::json!({
        "version": 2,
        "bots": [],
        "conversations": [],
        "messages": [],
        "threads": [],
        "tasks": [{
            "id": "task-nonterminal",
            "owner": "bot:main",
            "revision": 3,
            "execution_generation": 1,
            "status": "recovery-required",
            "intent": null,
            "result": null
        }],
        "executions": [],
        "processes": [{
            "id": "process-nonterminal",
            "definition_id": "def-a",
            "definition_version": "1",
            "revision": 2,
            "scope_ref": "project:alpha",
            "initiator_ref": "bot:main",
            "lifecycle": "running",
            "current_step_ref": null,
            "waiting_condition_ref": null,
            "child_refs": [],
            "progress": 10,
            "terminal_reason": null
        }],
        "projects": [],
        "channels": [],
        "memories": [],
        "side_effects": [],
        "execution_audit_intents": [],
        "receipts": [],
        "results": [],
        "command_bindings": [],
        "command_request_digests": [],
        "idempotency_bindings": [],
        "memberships": [],
        "delegations": []
    });
    serde_json::to_vec(&value).expect("encode snapshot")
}

fn seed_runtime_root(runtime_root: &std::path::Path) -> Vec<u8> {
    fs::create_dir_all(runtime_root).expect("runtime root");
    fs::set_permissions(runtime_root, fs::Permissions::from_mode(0o700)).expect("chmod root");
    let application = nonterminal_application_snapshot_v2();
    write_owner_only(&runtime_root.join("application-state.json"), &application);
    write_owner_only(
        &runtime_root.join("security-state.json"),
        br#"{"schema_version":1,"state":{}}"#,
    );
    // Optional coordination + audit artifacts included when present.
    write_owner_only(
        &runtime_root.join(".security-application-uow.json"),
        br#"{"schema_version":1}"#,
    );
    write_owner_only(
        &runtime_root.join("audit-outbox.json"),
        br#"{"schema_version":1,"records":[]}"#,
    );
    application
}

#[test]
fn capture_produces_owner_only_artifacts_and_sha256_manifest() {
    let base = temp_root("capture-manifest");
    let runtime_root = base.join("runtime");
    seed_runtime_root(&runtime_root);
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let backup_dir = base.join("backup");

    let manifest = storage.capture(&backup_dir, 1000).expect("capture");

    // Manifest self-verifies and records a SHA-256 (64 hex chars) per artifact.
    manifest.verify_self().expect("manifest self-verify");
    assert!(
        manifest
            .entries
            .iter()
            .any(|entry| entry.file_name == "application-state.json"),
        "application state must be in the set"
    );
    for entry in &manifest.entries {
        assert_eq!(
            entry.sha256.len(),
            64,
            "sha256 hex length for {}",
            entry.file_name
        );
    }
    assert_eq!(manifest.manifest_digest.len(), 64);

    // Backup directory is owner-only; every backed-up file is owner-only.
    let dir_mode = fs::metadata(&backup_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(dir_mode, 0o700, "backup directory owner-only");
    for entry in &manifest.entries {
        let mode = fs::metadata(backup_dir.join(&entry.file_name))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "artifact {} owner-only", entry.file_name);
    }
}

#[test]
fn dry_run_verifies_without_mutating_runtime_root() {
    let base = temp_root("dry-run");
    let runtime_root = base.join("runtime");
    seed_runtime_root(&runtime_root);
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let backup_dir = base.join("backup");
    storage.capture(&backup_dir, 1000).expect("capture");

    // Corrupt the live application state after backup, then dry-run restore.
    write_owner_only(
        &runtime_root.join("application-state.json"),
        b"{\"version\":2,\"tampered\":true}",
    );
    let live_after_corrupt = fs::read(runtime_root.join("application-state.json")).unwrap();

    let outcome = storage
        .restore(&backup_dir, RestoreMode::DryRun)
        .expect("dry-run restore");
    assert_eq!(outcome.mode, RestoreMode::DryRun);
    assert!(outcome.applied.is_empty(), "dry-run must not write");
    assert!(
        outcome
            .verified
            .contains(&"application-state.json".to_owned())
    );

    // Live root is untouched by dry-run.
    assert_eq!(
        fs::read(runtime_root.join("application-state.json")).unwrap(),
        live_after_corrupt
    );
}

#[test]
fn authorized_apply_preserves_nonterminal_state_byte_for_byte() {
    let base = temp_root("apply-byte-for-byte");
    let runtime_root = base.join("runtime");
    let original = seed_runtime_root(&runtime_root);
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let backup_dir = base.join("backup");
    storage.capture(&backup_dir, 1000).expect("capture");

    // Simulate loss/corruption of the live nonterminal state.
    write_owner_only(&runtime_root.join("application-state.json"), b"{}");

    let outcome = storage
        .restore(&backup_dir, RestoreMode::Apply)
        .expect("authorized apply restore");
    assert_eq!(outcome.mode, RestoreMode::Apply);
    assert!(
        outcome
            .applied
            .contains(&"application-state.json".to_owned())
    );

    // Nonterminal state is restored byte-for-byte.
    let restored = fs::read(runtime_root.join("application-state.json")).unwrap();
    assert_eq!(restored, original, "byte-for-byte nonterminal preservation");
    // Restored file is owner-only.
    let mode = fs::metadata(runtime_root.join("application-state.json"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn digest_corruption_fails_closed_and_does_not_apply() {
    let base = temp_root("digest-corruption");
    let runtime_root = base.join("runtime");
    seed_runtime_root(&runtime_root);
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let backup_dir = base.join("backup");
    storage.capture(&backup_dir, 1000).expect("capture");

    // Tamper with a backed-up artifact's bytes without updating the manifest.
    write_owner_only(
        &backup_dir.join("application-state.json"),
        b"{\"version\":2,\"tampered\":true}",
    );
    let live_before = fs::read(runtime_root.join("application-state.json")).unwrap();

    let error = storage
        .restore(&backup_dir, RestoreMode::Apply)
        .expect_err("digest corruption must fail closed");
    assert!(
        matches!(error, OfflineStorageError::DigestMismatch { .. }),
        "{error}"
    );
    // Fail-closed: live root is not partially applied.
    assert_eq!(
        fs::read(runtime_root.join("application-state.json")).unwrap(),
        live_before
    );
}

#[test]
fn manifest_tampering_fails_closed() {
    let base = temp_root("manifest-tamper");
    let runtime_root = base.join("runtime");
    seed_runtime_root(&runtime_root);
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let backup_dir = base.join("backup");
    storage.capture(&backup_dir, 1000).expect("capture");

    // Corrupt the manifest self-digest.
    let manifest_path = backup_dir.join("backup-manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    value["manifest_digest"] = serde_json::Value::from("0".repeat(64));
    write_owner_only(&manifest_path, &serde_json::to_vec(&value).unwrap());

    let error = storage
        .restore(&backup_dir, RestoreMode::DryRun)
        .expect_err("manifest tampering must fail closed");
    assert!(
        matches!(error, OfflineStorageError::ManifestCorrupt(_)),
        "{error}"
    );
}

#[test]
fn symlink_artifact_is_rejected() {
    let base = temp_root("symlink-reject");
    let runtime_root = base.join("runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    fs::set_permissions(&runtime_root, fs::Permissions::from_mode(0o700)).unwrap();

    // Required application state present as a real file.
    write_owner_only(
        &runtime_root.join("application-state.json"),
        &nonterminal_application_snapshot_v2(),
    );
    // security-state.json is a symlink -> must be rejected, not followed.
    let target = base.join("outside-secret.json");
    write_owner_only(&target, br#"{"schema_version":1,"state":{}}"#);
    std::os::unix::fs::symlink(&target, runtime_root.join("security-state.json")).unwrap();

    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let error = storage
        .capture(&base.join("backup"), 1000)
        .expect_err("symlink artifact must be rejected");
    assert!(
        matches!(error, OfflineStorageError::NotRegularFile(_)),
        "{error}"
    );
}

#[test]
fn missing_required_artifact_fails_closed() {
    let base = temp_root("missing-required");
    let runtime_root = base.join("runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    fs::set_permissions(&runtime_root, fs::Permissions::from_mode(0o700)).unwrap();
    // Only the optional audit artifact present; required state absent.
    write_owner_only(
        &runtime_root.join("audit-outbox.json"),
        br#"{"schema_version":1,"records":[]}"#,
    );
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    let error = storage
        .capture(&base.join("backup"), 1000)
        .expect_err("missing required artifact must fail closed");
    assert!(
        matches!(error, OfflineStorageError::MissingRequiredArtifact(_)),
        "{error}"
    );
}

// ---- Migration ----

/// A legacy v1 Application snapshot: no additive v2 array fields.
fn application_snapshot_v1() -> Vec<u8> {
    let value = serde_json::json!({
        "version": 1,
        "bots": [],
        "conversations": [],
        "messages": [],
        "threads": [],
        "tasks": [{
            "id": "task-nonterminal",
            "owner": "bot:main",
            "revision": 3,
            "execution_generation": 1,
            "status": "recovery-required",
            "intent": null,
            "result": null
        }],
        "projects": [],
        "channels": [],
        "memories": [],
        "side_effects": [],
        "receipts": [],
        "results": [],
        "command_bindings": [],
        "command_request_digests": [],
        "idempotency_bindings": [],
        "memberships": [],
        "delegations": []
    });
    serde_json::to_vec(&value).expect("encode v1")
}

#[test]
fn migration_absent_when_no_snapshot() {
    let base = temp_root("migrate-absent");
    let runtime_root = base.join("runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    let outcome = migrate_application_state(&runtime_root, &base.join("backups")).expect("migrate");
    assert_eq!(outcome, MigrationOutcome::Absent);
}

#[test]
fn migration_leaves_v2_snapshot_untouched() {
    let base = temp_root("migrate-v2-noop");
    let runtime_root = base.join("runtime");
    seed_runtime_root(&runtime_root);
    let before = fs::read(runtime_root.join("application-state.json")).unwrap();
    let outcome = migrate_application_state(&runtime_root, &base.join("backups")).expect("migrate");
    assert_eq!(outcome, MigrationOutcome::AlreadyCurrent);
    assert_eq!(
        fs::read(runtime_root.join("application-state.json")).unwrap(),
        before,
        "already-current snapshot must be byte-for-byte untouched"
    );
}

#[test]
fn migration_v1_to_v2_backs_up_and_rewrites_loadable_v2() {
    let base = temp_root("migrate-v1");
    let runtime_root = base.join("runtime");
    fs::create_dir_all(&runtime_root).unwrap();
    fs::set_permissions(&runtime_root, fs::Permissions::from_mode(0o700)).unwrap();
    write_owner_only(
        &runtime_root.join("application-state.json"),
        &application_snapshot_v1(),
    );
    // Required companion + optional artifacts for a consistent backup set.
    write_owner_only(
        &runtime_root.join("security-state.json"),
        br#"{"schema_version":1,"state":{}}"#,
    );

    let v1_original = fs::read(runtime_root.join("application-state.json")).unwrap();
    let backups_root = base.join("backups");
    let outcome = migrate_application_state(&runtime_root, &backups_root).expect("migrate v1");

    let backup_dir = match outcome {
        MigrationOutcome::Migrated { backup_dir } => backup_dir,
        other => panic!("expected Migrated, got {other:?}"),
    };

    // The migrated on-disk snapshot is now version 2 and carries additive
    // fields as empty arrays (matching v2 serde defaults).
    let migrated: serde_json::Value =
        serde_json::from_slice(&fs::read(runtime_root.join("application-state.json")).unwrap())
            .unwrap();
    assert_eq!(migrated["version"], 2);
    assert!(migrated["executions"].is_array());
    assert!(migrated["processes"].is_array());
    assert!(migrated["execution_audit_intents"].is_array());
    // Nonterminal domain data preserved through the migration.
    assert_eq!(migrated["tasks"][0]["status"], "recovery-required");

    // Migrated file remains owner-only.
    let mode = fs::metadata(runtime_root.join("application-state.json"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);

    // The canonical Application loader accepts the migrated snapshot.
    let (_store, restored) =
        application::ApplicationStateStore::open(runtime_root.join("application-state.json"))
            .expect("migrated snapshot loads under canonical v2 loader");
    assert!(
        restored
            .tasks
            .values()
            .any(|task| task.status == application::TaskStatus::RecoveryRequired)
    );

    // Rollback rehearsal: the pre-migration backup restores the v1 bytes
    // byte-for-byte via an authorized apply.
    let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");
    storage
        .restore(&backup_dir, RestoreMode::Apply)
        .expect("rollback apply");
    assert_eq!(
        fs::read(runtime_root.join("application-state.json")).unwrap(),
        v1_original,
        "rollback restores pre-migration v1 bytes byte-for-byte"
    );
}

//! Integration tests for `LocalRuntimeHost` first-init bootstrap consumption:
//! owner principal registration from the manifest and restart identity
//! continuity.

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use runtime_bootstrap::InstanceManifest;
use runtime_host::LocalRuntimeHost;
use runtime_security::SecurityStateStore;

const SECURITY_STATE_FILE: &str = "security-state.json";

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct Layout {
    runtime_root: PathBuf,
    discovery_root: PathBuf,
}

fn temp_layout(name: &str) -> Layout {
    let seq = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let base = std::env::temp_dir().join(format!("dxbot-host-{}-{name}-{seq}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    Layout {
        runtime_root: base.join("runtime"),
        discovery_root: base.join("discovery"),
    }
}

#[test]
fn host_registers_owner_principal_from_manifest() {
    let layout = temp_layout("owner-principal");

    let mut host = LocalRuntimeHost::start(
        layout.runtime_root.clone(),
        layout.discovery_root.clone(),
        None,
    )
    .expect("host start");
    let instance_id = host.instance_id().clone();
    assert!(host.audit_healthy());
    assert_eq!(host.audit_record_count().expect("audit count"), 0);
    let pid_path = layout.runtime_root.join(".runtime-host.pid");
    assert_eq!(
        fs::read_to_string(&pid_path).expect("pid artifact").trim(),
        std::process::id().to_string()
    );
    let endpoint_path = PathBuf::from(
        host.endpoint_uri()
            .strip_prefix("unix://")
            .expect("unix endpoint URI"),
    );
    let report = host.shutdown().expect("graceful shutdown");
    assert_eq!(
        report.steps,
        [
            "admission-stopped",
            "activity-drained",
            "provider-drained",
            "provider-runtime-stopped",
            "audit-checkpointed",
            "endpoint-unpublished",
            "discovery-unpublished",
            "pid-removed",
        ]
    );
    assert!(!endpoint_path.exists(), "endpoint is unpublished last");
    assert!(
        !pid_path.exists(),
        "normal shutdown removes owned PID artifact"
    );
    assert_eq!(
        host.shutdown().expect("idempotent shutdown").steps,
        report.steps
    );
    drop(host);

    // The first-init manifest decided the owner principal; the host must have
    // registered exactly that principal into the canonical Security registry.
    let manifest = InstanceManifest::load(&layout.runtime_root).expect("load manifest");
    assert_eq!(manifest.instance_id, instance_id);

    let (_store, security) =
        SecurityStateStore::open(layout.runtime_root.join(SECURITY_STATE_FILE))
            .expect("open security state");
    let owner = security
        .principals
        .resolve_principal(&manifest.owner.principal_ref)
        .expect("owner principal registered");
    assert_eq!(owner.ref_, manifest.owner.principal_ref);
    assert!(
        security
            .authority
            .check_global_authority(&manifest.owner.principal_ref, "operator")
            .expect("owner authority lookup"),
        "first-init owner must have canonical operator authority"
    );
}

#[test]
fn host_restart_preserves_instance_identity_and_advances_generation() {
    let layout = temp_layout("restart-continuity");

    let (first_id, first_gen) = {
        let host = LocalRuntimeHost::start(
            layout.runtime_root.clone(),
            layout.discovery_root.clone(),
            None,
        )
        .expect("first host start");
        (host.instance_id().clone(), host.host_generation())
    };
    assert_eq!(first_gen, 1);

    let (second_id, second_gen) = {
        let host = LocalRuntimeHost::start(
            layout.runtime_root.clone(),
            layout.discovery_root.clone(),
            None,
        )
        .expect("second host start");
        (host.instance_id().clone(), host.host_generation())
    };

    // Identity is continuous across restart; generation fences forward.
    assert_eq!(second_id, first_id);
    assert_eq!(second_gen, 2);

    // Owner principal registration remains idempotent: still exactly resolvable.
    let manifest = InstanceManifest::load(&layout.runtime_root).expect("load manifest");
    assert_eq!(manifest.instance_id, first_id);
    let (_store, security) =
        SecurityStateStore::open(layout.runtime_root.join(SECURITY_STATE_FILE))
            .expect("open security state");
    assert!(
        security
            .principals
            .resolve_principal(&manifest.owner.principal_ref)
            .is_ok()
    );
}

#[test]
fn host_start_rolls_back_all_publication_on_discovery_failpoint() {
    use std::os::unix::fs::PermissionsExt;

    let layout = temp_layout("discovery-failpoint");
    fs::create_dir_all(&layout.runtime_root).expect("runtime root");
    fs::set_permissions(&layout.runtime_root, fs::Permissions::from_mode(0o700))
        .expect("runtime permissions");
    // Inject a discovery-publish failpoint: the discovery root is a regular
    // file, so the terminal discovery extension cannot persist its state and
    // its `start` fails. This is the last publication step, so the composition
    // must roll back the PID artifact and the bound control endpoint in reverse
    // order, leaving zero published artifacts.
    fs::write(&layout.discovery_root, b"not-a-directory").expect("discovery file");

    let error = LocalRuntimeHost::start(
        layout.runtime_root.clone(),
        layout.discovery_root.clone(),
        None,
    )
    .expect_err("discovery failpoint must fail startup closed");
    assert!(
        error.to_string().contains("runtime composition failed"),
        "startup failure must surface as a composition rollback: {error}"
    );

    // Zero published PID artifact.
    let pid_path = layout.runtime_root.join(".runtime-host.pid");
    assert!(
        !pid_path.exists(),
        "failed start must leave no published PID artifact"
    );
    // Zero published control endpoint socket. Bootstrap binds the endpoint
    // under `<runtime_root>/endpoints/`; assert no live socket remains there.
    let mut live_sockets = 0_u32;
    let endpoints_dir = layout.runtime_root.join("endpoints");
    if let Ok(read_dir) = fs::read_dir(&endpoints_dir) {
        for entry in read_dir.flatten() {
            if let Ok(meta) = entry.path().symlink_metadata() {
                use std::os::unix::fs::FileTypeExt;
                if meta.file_type().is_socket() {
                    live_sockets += 1;
                }
            }
        }
    }
    assert_eq!(
        live_sockets, 0,
        "failed start must unpublish the control endpoint socket"
    );
}

#[test]
fn host_start_fails_closed_for_corrupt_audit_outbox() {
    use std::os::unix::fs::PermissionsExt;

    let layout = temp_layout("corrupt-audit");
    fs::create_dir_all(&layout.runtime_root).expect("runtime root");
    fs::set_permissions(&layout.runtime_root, fs::Permissions::from_mode(0o700))
        .expect("runtime permissions");
    let audit_path = layout.runtime_root.join("audit-outbox.json");
    fs::write(
        &audit_path,
        br#"{"schema_version":1,"records":[{"tampered":true}]}"#,
    )
    .expect("corrupt audit fixture");
    fs::set_permissions(&audit_path, fs::Permissions::from_mode(0o600)).expect("audit permissions");

    let error = LocalRuntimeHost::start(
        layout.runtime_root.clone(),
        layout.discovery_root.clone(),
        None,
    )
    .expect_err("corrupt audit must prevent Runtime readiness");
    let message = error.to_string();
    assert!(
        message.contains("cannot restore Runtime Audit outbox"),
        "{message}"
    );
    assert!(message.contains("audit log is corrupt"), "{message}");
    assert!(
        !layout.runtime_root.join(".runtime-host.pid").exists(),
        "failed startup must not publish a live PID artifact"
    );
}

#[test]
fn offline_storage_lock_excludes_live_runtime_host() {
    let layout = temp_layout("offline-lock-fence");
    let host = LocalRuntimeHost::start(
        layout.runtime_root.clone(),
        layout.discovery_root.clone(),
        None,
    )
    .expect("host start");

    let error = runtime_host::OfflineStorage::open_offline(layout.runtime_root.clone())
        .expect_err("live Runtime must fence offline storage");
    assert!(
        error
            .to_string()
            .contains("offline storage lock unavailable")
    );
    drop(host);

    let offline = runtime_host::OfflineStorage::open_offline(layout.runtime_root.clone())
        .expect("offline lock after host shutdown");
    assert_eq!(offline.runtime_root(), layout.runtime_root.as_path());
}

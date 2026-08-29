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

    let host = LocalRuntimeHost::start(
        layout.runtime_root.clone(),
        layout.discovery_root.clone(),
        None,
    )
    .expect("host start");
    let instance_id = host.instance_id().clone();
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

//! Integration tests for atomic first-Instance bootstrap and endpoint
#![allow(clippy::expect_used)]
//! verification.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use dxbot_core::types::InstanceId;
use runtime_bootstrap::bootstrap::{BootstrapEndpoint, Error};
use runtime_bootstrap::{
    INSTANCE_MANIFEST_FILE, INSTANCE_MANIFEST_VERSION, InstanceManifest, RuntimeBootstrap,
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn temp_root(name: &str) -> PathBuf {
    let seq = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "dxbot-bootstrap-{}-{name}-{seq}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    path
}

#[test]
fn bootstrap_first_instance_is_atomic() -> Result<(), Error> {
    let root = temp_root("atomic");
    let a_root = root.clone();
    let b_root = root.clone();

    let a = std::thread::spawn(move || -> Result<BootstrapEndpoint, Error> {
        let mut bootstrap = RuntimeBootstrap::new();
        let id = bootstrap.bootstrap_first_instance(&a_root)?;
        bootstrap.verify_endpoint(&id)
    });
    let b = std::thread::spawn(move || -> Result<BootstrapEndpoint, Error> {
        let mut bootstrap = RuntimeBootstrap::new();
        let id = bootstrap.bootstrap_first_instance(&b_root)?;
        bootstrap.verify_endpoint(&id)
    });

    let results = [
        a.join().expect("thread a panicked"),
        b.join().expect("thread b panicked"),
    ];

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    for result in &results {
        if let Err(error) = result {
            assert_eq!(error, &Error::AlreadyBootstrapped);
        }
    }

    let descriptor = results
        .into_iter()
        .find_map(Result::ok)
        .expect("a winner must exist");
    assert!(descriptor.verified);
    Ok(())
}

#[test]
fn bootstrap_commit_marker_is_published_after_required_state() -> Result<(), Error> {
    let root = temp_root("commit-last");
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || -> Result<(), Error> {
        let mut bootstrap = RuntimeBootstrap::new();
        bootstrap.bootstrap_first_instance(&worker_root)?;
        Ok(())
    });

    let commit = root.join("instance.id");
    for _ in 0..10_000 {
        if commit.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(commit.exists(), "bootstrap did not publish instance.id");

    // Observing the commit marker is sufficient evidence that every required
    // auxiliary artifact was already published.
    let instance_id = fs::read_to_string(&commit).map_err(err_map)?;
    assert_eq!(
        fs::read_to_string(root.join("host-generation")).map_err(err_map)?,
        "1"
    );
    assert!(
        root.join("endpoints")
            .join(format!("{}.sock", instance_id.trim()))
            .is_file()
    );

    worker.join().expect("bootstrap worker panicked")?;
    Ok(())
}

#[test]
fn bootstrap_verified_endpoint_has_correct_descriptor() -> Result<(), Error> {
    let root = temp_root("descriptor");
    let mut bootstrap = RuntimeBootstrap::new();

    let instance_id = bootstrap.bootstrap_first_instance(&root)?;
    let descriptor = bootstrap.verify_endpoint(&instance_id)?;

    assert_eq!(descriptor.instance_id, instance_id);
    assert_eq!(descriptor.host_generation, 1);
    assert!(descriptor.verified);
    assert!(descriptor.endpoint_path.exists());
    assert!(
        descriptor
            .endpoint_path
            .to_string_lossy()
            .ends_with(".sock")
    );

    let mut second = RuntimeBootstrap::new();
    assert_eq!(
        second.bootstrap_first_instance(&root),
        Err(Error::AlreadyBootstrapped)
    );
    Ok(())
}

#[test]
fn bootstrap_crash_during_init_leaves_clean_state() -> Result<(), Error> {
    let root = temp_root("crash");

    fs::create_dir_all(&root).map_err(err_map)?;
    fs::write(root.join(".dxbot-bootstrap.lock"), "crashed").map_err(err_map)?;
    fs::write(
        root.join("instance.id").with_extension("tmp"),
        "partial-instance",
    )
    .map_err(err_map)?;
    fs::write(root.join("host-generation.tmp"), "partial").map_err(err_map)?;

    let mut bootstrap = RuntimeBootstrap::new();
    let instance_id = bootstrap.bootstrap_first_instance(&root)?;
    let descriptor = bootstrap.verify_endpoint(&instance_id)?;

    assert!(descriptor.verified);
    assert_eq!(descriptor.host_generation, 1);

    for entry in fs::read_dir(&root).map_err(err_map)? {
        let name = entry
            .map_err(err_map)?
            .file_name()
            .to_string_lossy()
            .into_owned();
        assert!(
            !name.ends_with(".tmp"),
            "crash artifact left behind: {name}"
        );
    }
    let lock_path = root.join(".dxbot-bootstrap.lock");
    assert!(
        lock_path.is_file(),
        "bootstrap lock inode must remain durable"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(lock_path)
                .map_err(err_map)?
                .permissions()
                .mode()
                & 0o077,
            0,
            "bootstrap lock must be owner-only"
        );
    }
    Ok(())
}

fn err_map(error: std::io::Error) -> Error {
    Error::Io(error.to_string())
}

// ── BF-CLI-026: first-init default artifacts (manifest, owner, policy) ──

#[test]
fn manifest_artifacts_are_complete_at_commit_observation() -> Result<(), Error> {
    let root = temp_root("manifest-commit");
    let worker_root = root.clone();
    let worker = std::thread::spawn(move || -> Result<(), Error> {
        let mut bootstrap = RuntimeBootstrap::new();
        bootstrap.bootstrap_first_instance(&worker_root)?;
        Ok(())
    });

    let commit = root.join("instance.id");
    for _ in 0..10_000 {
        if commit.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(commit.exists(), "bootstrap did not publish instance.id");

    // Observing instance.id must imply the full first-init artifact set is
    // already durable: host-generation=1, endpoint path, and a complete,
    // owner-only initialization manifest.
    let committed = fs::read_to_string(&commit).map_err(err_map)?;
    let committed = committed.trim().to_owned();

    let manifest = InstanceManifest::load(&root)?;
    assert_eq!(manifest.manifest_version, INSTANCE_MANIFEST_VERSION);
    assert_eq!(manifest.instance_id.0, committed);
    assert_eq!(manifest.host_generation, 1);
    assert!(manifest.data_schema_version >= 1);
    assert_eq!(manifest.policy_generations.brain, 1);
    assert_eq!(manifest.policy_generations.permission, 1);
    assert_eq!(manifest.policy_generations.resource, 1);
    assert_eq!(manifest.policy_generations.provider, 1);
    assert!(!manifest.owner.principal_ref.0.is_empty());
    assert!(manifest.owner.principal_ref.0.contains(&committed));
    assert!(!manifest.owner.authority_binding_id.is_empty());
    assert_eq!(manifest.owner.authority_generation, 1);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(root.join(INSTANCE_MANIFEST_FILE))
            .map_err(err_map)?
            .permissions()
            .mode();
        assert_eq!(mode & 0o077, 0, "manifest must be owner-only");
    }

    worker.join().expect("bootstrap worker panicked")?;
    Ok(())
}

#[test]
fn manifest_identity_survives_restart_attach() -> Result<(), Error> {
    let root = temp_root("manifest-restart");

    let mut first = RuntimeBootstrap::new();
    let instance_id = first.bootstrap_first_instance(&root)?;
    let first_manifest = first.load_manifest()?;

    // A fresh process attaches to the committed identity without changing it.
    let mut attached = RuntimeBootstrap::new();
    let attached_id = attached.attach_existing(&root)?;
    assert_eq!(attached_id, instance_id);
    let attached_manifest = attached.load_manifest()?;

    assert_eq!(first_manifest, attached_manifest);
    assert_eq!(attached_manifest.instance_id, instance_id);
    Ok(())
}

#[test]
fn corrupt_manifest_fails_closed_on_attach() -> Result<(), Error> {
    let root = temp_root("manifest-corrupt");

    let mut bootstrap = RuntimeBootstrap::new();
    let _ = bootstrap.bootstrap_first_instance(&root)?;

    // Truncate the manifest to invalid JSON: attach must fail closed rather
    // than adopting an unverified identity.
    fs::write(root.join(INSTANCE_MANIFEST_FILE), b"{ not json").map_err(err_map)?;

    let mut reattach = RuntimeBootstrap::new();
    match reattach.attach_existing(&root) {
        Err(Error::CorruptState(_)) => Ok(()),
        other => panic!("expected CorruptState on corrupt manifest, got {other:?}"),
    }
}

#[test]
fn missing_manifest_fails_closed_on_attach() -> Result<(), Error> {
    let root = temp_root("manifest-missing");

    let mut bootstrap = RuntimeBootstrap::new();
    let _ = bootstrap.bootstrap_first_instance(&root)?;
    fs::remove_file(root.join(INSTANCE_MANIFEST_FILE)).map_err(err_map)?;

    let mut reattach = RuntimeBootstrap::new();
    match reattach.attach_existing(&root) {
        Err(Error::CorruptState(_)) => Ok(()),
        other => panic!("expected CorruptState on missing manifest, got {other:?}"),
    }
}

#[test]
fn concurrent_bootstrap_publishes_single_consistent_manifest() -> Result<(), Error> {
    let root = temp_root("manifest-concurrent");
    let a_root = root.clone();
    let b_root = root.clone();

    let a = std::thread::spawn(move || -> Result<InstanceId, Error> {
        let mut bootstrap = RuntimeBootstrap::new();
        bootstrap.bootstrap_first_instance(&a_root)
    });
    let b = std::thread::spawn(move || -> Result<InstanceId, Error> {
        let mut bootstrap = RuntimeBootstrap::new();
        bootstrap.bootstrap_first_instance(&b_root)
    });

    let results = [
        a.join().expect("thread a panicked"),
        b.join().expect("thread b panicked"),
    ];
    let winner = results
        .iter()
        .find_map(|result| result.as_ref().ok())
        .expect("a winner must exist")
        .clone();

    // Exactly one committed identity, and the single durable manifest must
    // describe that winner. The loser cannot leave a divergent manifest.
    let committed = fs::read_to_string(root.join("instance.id")).map_err(err_map)?;
    assert_eq!(committed.trim(), winner.0);

    let manifest = InstanceManifest::load(&root)?;
    assert_eq!(manifest.instance_id, winner);
    assert_eq!(manifest.host_generation, 1);
    Ok(())
}

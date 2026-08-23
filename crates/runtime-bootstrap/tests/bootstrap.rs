//! Integration tests for atomic first-Instance bootstrap and endpoint
//! verification.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use runtime_bootstrap::bootstrap::{BootstrapEndpoint, Error};
use runtime_bootstrap::RuntimeBootstrap;

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

/// Build a unique, clean temp state root for a single test.
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

    // Each contender bootstraps and, on success, verifies within the same
    // bootstrap handle so the state root is the one it committed.
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

    let succeeded = results.iter().filter(|r| r.is_ok()).count();
    let failed = results.iter().filter(|r| r.is_err()).count();

    // Only one of the two concurrent bootstraps may succeed.
    assert_eq!(succeeded, 1, "expected exactly one successful bootstrap");
    assert_eq!(failed, 1, "expected exactly one refused bootstrap");

    for result in &results {
        if let Err(err) = result {
            assert_eq!(err, &Error::AlreadyBootstrapped);
        }
    }

    // The single committed instance is the fully verified endpoint.
    let descriptor = results.into_iter().find(|r| r.is_ok()).expect("a winner must exist").ok();
    let descriptor = descriptor.expect("winner is a verified descriptor");
    assert!(descriptor.verified);
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
    assert!(descriptor.endpoint_path.to_string_lossy().ends_with(".sock"));

    // A second bootstrap on the same root is refused.
    let mut second = RuntimeBootstrap::new();
    assert_eq!(second.bootstrap_first_instance(&root), Err(Error::AlreadyBootstrapped));
    Ok(())
}

#[test]
fn bootstrap_crash_during_init_leaves_clean_state() -> Result<(), Error> {
    let root = temp_root("crash");

    // Simulate a process that created the guard marker and some partial
    // artifacts, then crashed before committing the first Instance. The state
    // root already exists from the crashed process.
    fs::create_dir_all(&root).map_err(err_map)?;
    fs::write(root.join(".dxbot-bootstrap.lock"), "crashed").map_err(err_map)?;
    fs::write(root.join("instance.id" ).with_extension("tmp"), "partial-instance").map_err(err_map)?;
    fs::write(root.join("host-generation.tmp"), "partial").map_err(err_map)?;

    // A fresh bootstrap must recover into a clean, committed state.
    let mut bootstrap = RuntimeBootstrap::new();
    let instance_id = bootstrap.bootstrap_first_instance(&root)?;
    let descriptor = bootstrap.verify_endpoint(&instance_id)?;

    assert!(descriptor.verified);
    assert_eq!(descriptor.host_generation, 1);

    // No partial artifacts remain.
    let names = fs::read_dir(&root).map_err(err_map)?;
    for entry in names {
        let name = entry.map_err(err_map)?.file_name().to_string_lossy().into_owned();
        assert!(
            !name.ends_with(".tmp"),
            "crash artifact left behind: {name}"
        );
    }
    Ok(())
}

fn err_map(err: std::io::Error) -> Error {
    Error::Io(err.to_string())
}
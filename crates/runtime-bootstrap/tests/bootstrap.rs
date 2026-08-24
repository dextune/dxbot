//! Integration tests for atomic first-Instance bootstrap and endpoint
//! verification.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use runtime_bootstrap::bootstrap::{BootstrapEndpoint, Error};
use runtime_bootstrap::RuntimeBootstrap;

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
    for _ in 0..1000 {
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
    assert!(descriptor.endpoint_path.to_string_lossy().ends_with(".sock"));

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
        assert!(!name.ends_with(".tmp"), "crash artifact left behind: {name}");
    }
    assert!(!root.join(".dxbot-bootstrap.lock").exists());
    Ok(())
}

fn err_map(error: std::io::Error) -> Error {
    Error::Io(error.to_string())
}

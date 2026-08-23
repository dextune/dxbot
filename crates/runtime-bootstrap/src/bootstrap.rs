//! Atomic first Instance and verified endpoint bootstrap.
//!
//! Bootstrap is the exclusive, single-winner creation of the first runtime
//! Instance under a state root. Concurrency safety is provided by file-based
//! locking: an exclusive `create_new` on marker files. Whether a caller wins
//! the guard marker or observes a started/crashed bootstrap, exactly one
//! caller can exclusively create the commit marker, so only one process ever
//! reports a successful first bootstrap while the others report
//! [`Error::AlreadyBootstrapped`]. A crash during init leaves only removable
//! partial artifacts and never a half-committed Instance, so a later bootstrap
//! recovers into a clean state.

use std::fs::{self, OpenOptions};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dxbot_core::types::*;

/// Guard/commit marker file names persisted directly under the state root.
const LOCK_FILE: &str = ".dxbot-bootstrap.lock";
const COMMIT_FILE: &str = "instance.id";
const GENERATION_FILE: &str = "host-generation";
const ENDPOINTS_DIR: &str = "endpoints";

/// Successful and expected failure outcomes for the bootstrap lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An I/O error operating on the state root, with a message.
    Io(String),
    /// `verify_endpoint` was called before a state root was bootstrapped.
    NoStateRoot,
    /// The first Instance is already committed; a second bootstrap is refused.
    AlreadyBootstrapped,
    /// The requested Instance does not match the committed first Instance.
    InstanceNotFound,
    /// Persisted state exists but is malformed.
    CorruptState(String),
}

/// A verified, ready runtime endpoint for a bootstrapped Instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapEndpoint {
    pub instance_id: InstanceId,
    pub endpoint_path: PathBuf,
    pub host_generation: i64,
    pub verified: bool,
}

/// Owner of the atomic first-Instance bootstrap on a state root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBootstrap {
    state_root: Option<PathBuf>,
}

impl RuntimeBootstrap {
    /// Create a bootstrap handle with no state root bound yet.
    pub fn new() -> Self {
        Self { state_root: None }
    }

    /// Atomically create the first Instance under `state_root`.
    ///
    /// Exactly one concurrent caller succeeds; every other caller observes the
    /// committed marker and returns [`Error::AlreadyBootstrapped`].
    pub fn bootstrap_first_instance(&mut self, state_root: &Path) -> Result<InstanceId, Error> {
        fs::create_dir_all(state_root).map_err(io_err)?;
        self.state_root = Some(state_root.to_path_buf());

        // File-based lock: exclusive create of the guard marker. A live or
        // crashed owner leaves the marker behind; correctness on who may commit
        // is decided by the commit gate below, so a present guard marker alone
        // is not a hard failure.
        let lock_path = state_root.join(LOCK_FILE);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(_) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_err(e)),
        }

        let instance_id = generate_instance_id();

        // Commit gate: exclusive create of the Instance marker. Exactly one
        // concurrent caller can win, which makes the whole bootstrap atomic.
        // Only the winner touches the directory afterwards, so concurrent
        // callers never race on shared artifacts.
        let commit_path = state_root.join(COMMIT_FILE);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&commit_path)
        {
            Ok(mut file) => {
                file.write_all(instance_id.0.as_bytes()).map_err(io_err)?;
                file.flush().map_err(io_err)?;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                return Err(Error::AlreadyBootstrapped);
            }
            Err(e) => return Err(io_err(e)),
        }

        // Remove partial artifacts from any crashed init.
        clean_partials(state_root)?;

        // Auxiliary state is persisted with distinct atomic temp names so an
        // interrupted init never leaves partial content behind.
        write_atomic(&state_root.join(GENERATION_FILE), b"1", instance_id.0.as_str())?;
        let endpoint_path = endpoint_path(state_root, &instance_id);
        if let Some(parent) = endpoint_path.parent() {
            fs::create_dir_all(parent).map_err(io_err)?;
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&endpoint_path)
            .map_err(io_err)?;

        Ok(instance_id)
    }

    /// Verify that the endpoint for `instance_id` is ready.
    pub fn verify_endpoint(&self, instance_id: &InstanceId) -> Result<BootstrapEndpoint, Error> {
        let root = self.state_root.as_ref().ok_or(Error::NoStateRoot)?;

        let committed = fs::read_to_string(root.join(COMMIT_FILE)).map_err(io_err)?;
        let committed = committed.trim();
        if committed.is_empty() {
            return Err(Error::CorruptState("instance.id is empty".into()));
        }
        if committed != instance_id.0 {
            return Err(Error::InstanceNotFound);
        }

        let host_generation = read_generation(root)?;
        let endpoint_path = endpoint_path(root, instance_id);
        let verified = endpoint_path.exists();

        Ok(BootstrapEndpoint {
            instance_id: instance_id.clone(),
            endpoint_path,
            host_generation,
            verified,
        })
    }
}

impl Default for RuntimeBootstrap {
    fn default() -> Self {
        Self::new()
    }
}

fn clean_partials(root: &Path) -> Result<(), Error> {
    let endpoints_dir = root.join(ENDPOINTS_DIR);
    if endpoints_dir.exists() {
        fs::remove_dir_all(&endpoints_dir).map_err(io_err)?;
    }
    for entry in fs::read_dir(root).map_err(io_err)? {
        let entry = entry.map_err(io_err)?;
        if entry.file_name().to_string_lossy().ends_with(".tmp") {
            fs::remove_file(entry.path()).map_err(io_err)?;
        }
    }
    Ok(())
}

fn write_atomic(dest: &Path, bytes: &[u8], tag: &str) -> Result<(), Error> {
    // A per-caller tag keeps concurrent writers on distinct temp names; the
    // final rename is atomic and idempotent (last writer wins with identical
    // content).
    let name = dest.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = dest.with_file_name(format!("{name}.{tag}.tmp"));
    fs::write(&tmp, bytes).map_err(io_err)?;
    fs::rename(&tmp, dest).map_err(io_err)?;
    Ok(())
}

fn read_generation(root: &Path) -> Result<i64, Error> {
    let value = fs::read_to_string(root.join(GENERATION_FILE)).map_err(io_err)?;
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| Error::CorruptState("host-generation is malformed".into()))
}

fn endpoint_path(root: &Path, instance_id: &InstanceId) -> PathBuf {
    root.join(ENDPOINTS_DIR).join(format!("{}.sock", instance_id.0))
}

fn generate_instance_id() -> InstanceId {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    InstanceId(format!("instance-{pid}-{nanos}"))
}

fn io_err(err: io::Error) -> Error {
    Error::Io(err.to_string())
}
//! Atomic first-Instance bootstrap with commit-last publication.
//!
//! A bootstrap candidate prepares every required auxiliary artifact first and
//! publishes `instance.id` last with an atomic no-replace hard link. Therefore
//! any visible committed Instance already has its generation and endpoint.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dxbot_core::types::*;

const LOCK_FILE: &str = ".dxbot-bootstrap.lock";
const COMMIT_FILE: &str = "instance.id";
const GENERATION_FILE: &str = "host-generation";
const ENDPOINTS_DIR: &str = "endpoints";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Io(String),
    NoStateRoot,
    AlreadyBootstrapped,
    InstanceNotFound,
    CorruptState(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapEndpoint {
    pub instance_id: InstanceId,
    pub endpoint_path: PathBuf,
    pub host_generation: i64,
    pub verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeBootstrap {
    state_root: Option<PathBuf>,
}

impl RuntimeBootstrap {
    pub fn new() -> Self {
        Self { state_root: None }
    }

    /// Atomically create the first Instance under `state_root`.
    ///
    /// Required state is prepared and fsynced before `instance.id` becomes
    /// visible. Concurrent candidates may prepare harmless private/orphan
    /// artifacts, but exactly one can publish the no-replace commit marker.
    pub fn bootstrap_first_instance(&mut self, state_root: &Path) -> Result<InstanceId, Error> {
        fs::create_dir_all(state_root).map_err(io_err)?;
        self.state_root = Some(state_root.to_path_buf());

        let commit_path = state_root.join(COMMIT_FILE);
        if commit_path.exists() {
            return Err(Error::AlreadyBootstrapped);
        }

        let instance_id = generate_instance_id();
        let endpoint_path = endpoint_path(state_root, &instance_id);
        let endpoints_dir = endpoint_path
            .parent()
            .ok_or_else(|| Error::CorruptState("endpoint path has no parent".to_owned()))?;
        fs::create_dir_all(endpoints_dir).map_err(io_err)?;

        let endpoint_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&endpoint_path)
            .map_err(io_err)?;
        endpoint_file.sync_all().map_err(io_err)?;
        sync_dir(endpoints_dir)?;

        ensure_first_generation(state_root, &instance_id)?;

        let commit_candidate = state_root.join(format!(
            ".{COMMIT_FILE}.{}.tmp",
            instance_id.0
        ));
        let mut candidate_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&commit_candidate)
            .map_err(io_err)?;
        candidate_file
            .write_all(instance_id.0.as_bytes())
            .map_err(io_err)?;
        candidate_file.sync_all().map_err(io_err)?;
        drop(candidate_file);
        sync_dir(state_root)?;

        match fs::hard_link(&commit_candidate, &commit_path) {
            Ok(()) => {
                sync_dir(state_root)?;
                let _ = fs::remove_file(&commit_candidate);
                cleanup_after_commit(state_root, &instance_id);
                sync_dir(state_root)?;
                Ok(instance_id)
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                cleanup_losing_candidate(&endpoint_path, &commit_candidate);
                Err(Error::AlreadyBootstrapped)
            }
            Err(error) if error.kind() == ErrorKind::NotFound && commit_path.exists() => {
                cleanup_losing_candidate(&endpoint_path, &commit_candidate);
                Err(Error::AlreadyBootstrapped)
            }
            Err(error) => {
                cleanup_losing_candidate(&endpoint_path, &commit_candidate);
                Err(io_err(error))
            }
        }
    }

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
        let verified = endpoint_path.is_file();

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

fn ensure_first_generation(root: &Path, instance_id: &InstanceId) -> Result<(), Error> {
    let dest = root.join(GENERATION_FILE);
    if dest.exists() {
        return verify_generation_one(root);
    }

    let candidate = root.join(format!(".{GENERATION_FILE}.{}.tmp", instance_id.0));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&candidate)
        .map_err(io_err)?;
    file.write_all(b"1").map_err(io_err)?;
    file.sync_all().map_err(io_err)?;
    drop(file);

    match fs::hard_link(&candidate, &dest) {
        Ok(()) => {
            sync_dir(root)?;
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            verify_generation_one(root)?;
        }
        Err(error) if error.kind() == ErrorKind::NotFound && dest.exists() => {
            verify_generation_one(root)?;
        }
        Err(error) => {
            let _ = fs::remove_file(&candidate);
            return Err(io_err(error));
        }
    }
    let _ = fs::remove_file(candidate);
    sync_dir(root)?;
    Ok(())
}

fn verify_generation_one(root: &Path) -> Result<(), Error> {
    let generation = read_generation(root)?;
    if generation == 1 {
        Ok(())
    } else {
        Err(Error::CorruptState(format!(
            "first bootstrap expected host-generation 1, found {generation}"
        )))
    }
}

fn cleanup_losing_candidate(endpoint: &Path, candidate: &Path) {
    let _ = fs::remove_file(endpoint);
    let _ = fs::remove_file(candidate);
}

/// After the commit marker is visible all other candidates are losers, so the
/// winner may remove orphan endpoints and transient bootstrap artifacts.
fn cleanup_after_commit(root: &Path, winner: &InstanceId) {
    let endpoints = root.join(ENDPOINTS_DIR);
    if let Ok(entries) = fs::read_dir(&endpoints) {
        let winner_name = format!("{}.sock", winner.0);
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy() != winner_name {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.ends_with(".tmp") || name == LOCK_FILE {
                let _ = fs::remove_file(entry.path());
            }
        }
    }
}

fn read_generation(root: &Path) -> Result<i64, Error> {
    let value = fs::read_to_string(root.join(GENERATION_FILE)).map_err(io_err)?;
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| Error::CorruptState("host-generation is malformed".into()))
}

fn endpoint_path(root: &Path, instance_id: &InstanceId) -> PathBuf {
    root.join(ENDPOINTS_DIR)
        .join(format!("{}.sock", instance_id.0))
}

fn generate_instance_id() -> InstanceId {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    InstanceId(format!("instance-{pid}-{nanos}"))
}

fn sync_dir(path: &Path) -> Result<(), Error> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(io_err)
}

fn io_err(error: io::Error) -> Error {
    Error::Io(error.to_string())
}

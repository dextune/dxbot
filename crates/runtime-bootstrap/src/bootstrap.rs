//! Atomic first-Instance bootstrap with commit-last publication.
//!
//! A bootstrap candidate prepares every required auxiliary artifact first and
//! publishes `instance.id` last with an atomic no-replace hard link. Therefore
//! any visible committed Instance already has its generation and endpoint path.
//! Runtime host restart attaches to that committed identity and advances the
//! host generation atomically before replacing a stale endpoint under the host
//! single-writer lock.

use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};

use dxbot_core::types::*;
use fs2::FileExt;

use crate::manifest::InstanceManifest;

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
        #[cfg(unix)]
        fs::set_permissions(state_root, fs::Permissions::from_mode(0o700)).map_err(io_err)?;
        self.state_root = Some(state_root.to_path_buf());

        // The advisory lock covers stale-artifact recovery, all first-init
        // writes, and commit publication. A process crash releases the lock,
        // while the durable lock inode remains stable for future contenders.
        let bootstrap_lock = open_bootstrap_lock(&state_root.join(LOCK_FILE))?;
        bootstrap_lock.lock_exclusive().map_err(io_err)?;

        let commit_path = state_root.join(COMMIT_FILE);
        if commit_path.exists() {
            return Err(Error::AlreadyBootstrapped);
        }

        // A valid manifest without instance.id can only be left by a prior
        // lock holder that crashed before commit. Reclaim it while exclusively
        // locked; malformed or unsafe state fails closed instead of being
        // silently replaced.
        let manifest_path = state_root.join(crate::manifest::INSTANCE_MANIFEST_FILE);
        if manifest_path.exists() {
            InstanceManifest::load(state_root)?;
            fs::remove_file(&manifest_path).map_err(io_err)?;
            sync_dir(state_root)?;
        }

        let instance_id = generate_instance_id();
        let endpoint_path = endpoint_path(state_root, &instance_id);
        let endpoints_dir = endpoint_path
            .parent()
            .ok_or_else(|| Error::CorruptState("endpoint path has no parent".to_owned()))?;
        fs::create_dir_all(endpoints_dir).map_err(io_err)?;

        // Bootstrap publishes a durable placeholder path before the commit
        // marker. The Runtime Host later replaces it with the live Unix socket
        // only while holding the host single-writer lock.
        let endpoint_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&endpoint_path)
            .map_err(io_err)?;
        harden_owner_file(&endpoint_path)?;
        endpoint_file.sync_all().map_err(io_err)?;
        sync_dir(endpoints_dir)?;

        ensure_first_generation(state_root, &instance_id)?;

        let commit_candidate = state_root.join(format!(".{COMMIT_FILE}.{}.tmp", instance_id.0));
        let mut candidate_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&commit_candidate)
            .map_err(io_err)?;
        harden_owner_file(&commit_candidate)?;
        candidate_file
            .write_all(instance_id.0.as_bytes())
            .map_err(io_err)?;
        candidate_file.sync_all().map_err(io_err)?;
        drop(candidate_file);

        // The complete initialization record is durable before the observable
        // commit marker. It contains the owner identity/binding defaults,
        // policy generations, and DataSchemaVersion selected by bootstrap.
        if let Err(error) =
            InstanceManifest::first_init(&instance_id, state_root).prepare(state_root)
        {
            cleanup_losing_candidate(&endpoint_path, &commit_candidate);
            return Err(error);
        }
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
            Err(error) => {
                cleanup_losing_candidate(&endpoint_path, &commit_candidate);
                Err(io_err(error))
            }
        }
    }

    /// Attach to an already committed Runtime state root without creating or
    /// changing identity.
    pub fn attach_existing(&mut self, state_root: &Path) -> Result<InstanceId, Error> {
        self.state_root = Some(state_root.to_path_buf());
        let committed = fs::read_to_string(state_root.join(COMMIT_FILE)).map_err(io_err)?;
        let committed = committed.trim();
        if committed.is_empty() {
            return Err(Error::CorruptState("instance.id is empty".to_owned()));
        }
        let generation = read_generation(state_root)?;
        if generation <= 0 {
            return Err(Error::CorruptState(format!(
                "host-generation must be positive, found {generation}"
            )));
        }
        // Restart identity continuity: the durable manifest must exist and
        // describe exactly the committed Instance, or attach fails closed.
        let instance_id = InstanceId(committed.to_owned());
        InstanceManifest::load_for_instance(state_root, &instance_id)?;
        Ok(instance_id)
    }

    /// Load and fully validate the first-init initialization manifest for the
    /// attached/bootstrapped state root. Fails closed on corruption, wrong
    /// ownership, or identity mismatch.
    pub fn load_manifest(&self) -> Result<InstanceManifest, Error> {
        let root = self.state_root.as_ref().ok_or(Error::NoStateRoot)?;
        InstanceManifest::load(root)
    }

    /// Advance HostGeneration using write-fsync-rename-fsync. The caller must
    /// already hold the Runtime host single-writer lock.
    pub fn advance_host_generation(&self) -> Result<i64, Error> {
        let root = self.state_root.as_ref().ok_or(Error::NoStateRoot)?;
        let current = read_generation(root)?;
        let next = current
            .checked_add(1)
            .ok_or_else(|| Error::CorruptState("host-generation exhausted".to_owned()))?;
        let tmp = root.join(format!(".{GENERATION_FILE}.tmp"));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)
            .map_err(io_err)?;
        harden_owner_file(&tmp)?;
        file.write_all(next.to_string().as_bytes())
            .map_err(io_err)?;
        file.sync_all().map_err(io_err)?;
        drop(file);
        fs::rename(&tmp, root.join(GENERATION_FILE)).map_err(io_err)?;
        sync_dir(root)?;
        Ok(next)
    }

    pub fn endpoint_path(&self, instance_id: &InstanceId) -> Result<PathBuf, Error> {
        let root = self.state_root.as_ref().ok_or(Error::NoStateRoot)?;
        Ok(endpoint_path(root, instance_id))
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
        let verified = verify_endpoint_file_type(&endpoint_path)?;

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
    harden_owner_file(&candidate)?;
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

fn open_bootstrap_lock(path: &Path) -> Result<File, Error> {
    // Multiple first-boot candidates may reach lock creation concurrently. The
    // lock file is a shared, durable inode: whoever creates it first wins the
    // create, and every other candidate must reopen the same inode so that the
    // advisory `flock` below actually serializes them. A concurrent
    // `create_new` losing the race returns `AlreadyExists`, which is a normal
    // outcome here and must not leak as a raw I/O error.
    loop {
        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                    return Err(Error::CorruptState(format!(
                        "bootstrap lock is not a direct regular file: {}",
                        path.display()
                    )));
                }
                let file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(path)
                    .map_err(io_err)?;
                #[cfg(unix)]
                fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_err)?;
                return Ok(file);
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create_new(true)
                    .open(path)
                {
                    Ok(file) => {
                        #[cfg(unix)]
                        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                            .map_err(io_err)?;
                        return Ok(file);
                    }
                    // Lost the create race with a concurrent candidate; the inode
                    // now exists, so loop and reopen it.
                    Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(io_err(error)),
                }
            }
            Err(error) => return Err(io_err(error)),
        }
    }
}

/// After the commit marker is visible all other candidates are losers, so the
/// winner may remove orphan endpoints and transient bootstrap artifacts. The
/// durable lock file is deliberately retained: unlinking a locked inode would
/// allow a second process to create and lock a different inode concurrently.
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
            if name.ends_with(".tmp") {
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

#[cfg(unix)]
fn harden_owner_file(path: &Path) -> Result<(), Error> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_err)
}

#[cfg(not(unix))]
fn harden_owner_file(_path: &Path) -> Result<(), Error> {
    Ok(())
}
fn generate_instance_id() -> InstanceId {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    InstanceId(format!("instance-{pid}-{nanos}"))
}

fn verify_endpoint_file_type(path: &Path) -> Result<bool, Error> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_err(error)),
    };
    if metadata.file_type().is_symlink() {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        Ok(metadata.file_type().is_file() || metadata.file_type().is_socket())
    }
    #[cfg(not(unix))]
    {
        Ok(metadata.file_type().is_file())
    }
}

fn sync_dir(path: &Path) -> Result<(), Error> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(io_err)
}

fn io_err(error: io::Error) -> Error {
    Error::Io(error.to_string())
}

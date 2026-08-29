//! Runtime-owned publication format for verified local control endpoints.
//!
//! The Runtime Host is the sole writer. CLI discovery consumes the same type
//! read-only, preventing a duplicated descriptor schema or a CLI-owned Runtime
//! bootstrap path.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use dxbot_core::types::InstanceId;
use serde::{Deserialize, Serialize};

use crate::bootstrap::Error;

pub const DISCOVERY_STATE_FILE: &str = "discovery.json";
const MAX_DISCOVERY_STATE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryEndpoint {
    pub instance_id: InstanceId,
    pub profile: Option<String>,
    pub endpoint: String,
    pub host_generation: i64,
    pub provider_id: String,
    #[serde(default)]
    pub provider_generation: i64,
    pub provider_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryState {
    pub instance_endpoints: HashMap<InstanceId, DiscoveryEndpoint>,
}

impl DiscoveryState {
    pub fn empty() -> Self {
        Self {
            instance_endpoints: HashMap::new(),
        }
    }

    pub fn load_state(base_path: &Path) -> Result<Self, Error> {
        let path = base_path.join(DISCOVERY_STATE_FILE);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::empty()),
            Err(error) => return Err(Error::Io(format!("{}: {error}", path.display()))),
        };
        validate_state_metadata(&path, &metadata)?;
        if metadata.len() > MAX_DISCOVERY_STATE_BYTES {
            return Err(Error::CorruptState(format!(
                "discovery state exceeds {} bytes: {}",
                MAX_DISCOVERY_STATE_BYTES,
                path.display()
            )));
        }

        let mut data = Vec::with_capacity(metadata.len() as usize);
        File::open(&path)
            .and_then(|file| {
                file.take(MAX_DISCOVERY_STATE_BYTES + 1)
                    .read_to_end(&mut data)
                    .map(|_| ())
            })
            .map_err(|error| Error::Io(format!("{}: {error}", path.display())))?;
        if data.len() as u64 > MAX_DISCOVERY_STATE_BYTES {
            return Err(Error::CorruptState(format!(
                "discovery state grew beyond {} bytes during read: {}",
                MAX_DISCOVERY_STATE_BYTES,
                path.display()
            )));
        }
        serde_json::from_slice(&data)
            .map_err(|error| Error::CorruptState(format!("{}: {error}", path.display())))
    }

    /// Atomically publish the complete verified endpoint set. The file and
    /// parent directory are owner-only and fsynced before publication returns.
    pub fn save_state(&self, base_path: &Path) -> Result<(), Error> {
        fs::create_dir_all(base_path)
            .map_err(|error| Error::Io(format!("{}: {error}", base_path.display())))?;
        harden_directory(base_path)?;
        let path = base_path.join(DISCOVERY_STATE_FILE);
        let tmp = unique_temp_path(base_path);
        let bytes = serde_json::to_vec(self)
            .map_err(|error| Error::CorruptState(format!("serialize discovery state: {error}")))?;
        if bytes.len() as u64 > MAX_DISCOVERY_STATE_BYTES {
            return Err(Error::CorruptState(format!(
                "serialized discovery state exceeds {MAX_DISCOVERY_STATE_BYTES} bytes"
            )));
        }

        let publish_result = (|| -> Result<(), Error> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .map_err(|error| Error::Io(format!("{}: {error}", tmp.display())))?;
            harden_file(&tmp)?;
            file.write_all(&bytes)
                .map_err(|error| Error::Io(format!("{}: {error}", tmp.display())))?;
            file.sync_all()
                .map_err(|error| Error::Io(format!("{}: {error}", tmp.display())))?;
            drop(file);
            fs::rename(&tmp, &path).map_err(|error| {
                Error::Io(format!("{} -> {}: {error}", tmp.display(), path.display()))
            })?;
            File::open(base_path)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| Error::Io(format!("{}: {error}", base_path.display())))?;
            validate_state_file(&path)
        })();

        if publish_result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        publish_result
    }
}

pub fn discovery_state_owner_uid(base_path: &Path) -> Result<Option<u32>, Error> {
    let path = base_path.join(DISCOVERY_STATE_FILE);
    match fs::symlink_metadata(&path) {
        Ok(metadata) => {
            validate_state_metadata(&path, &metadata)?;
            #[cfg(unix)]
            {
                Ok(Some(metadata.uid()))
            }
            #[cfg(not(unix))]
            {
                let _ = metadata;
                Ok(None)
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Error::Io(format!("{}: {error}", path.display()))),
    }
}

fn validate_state_file(path: &Path) -> Result<(), Error> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::Io(format!("{}: {error}", path.display())))?;
    validate_state_metadata(path, &metadata)
}

fn validate_state_metadata(path: &Path, metadata: &fs::Metadata) -> Result<(), Error> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(Error::CorruptState(format!(
            "discovery state is not a direct regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error::CorruptState(format!(
            "discovery state permissions are not owner-only: {}",
            path.display()
        )));
    }
    Ok(())
}

fn unique_temp_path(base_path: &Path) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    base_path.join(format!(
        ".{DISCOVERY_STATE_FILE}.{}.{}.tmp",
        std::process::id(),
        nanos
    ))
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> Result<(), Error> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| Error::Io(format!("{}: {error}", path.display())))
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<(), Error> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| Error::Io(format!("{}: {error}", path.display())))
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<(), Error> {
    Ok(())
}

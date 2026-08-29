//! Crash-safe cross-owner coordination marker for Application + Security UoW.
//!
//! The marker stores only the operation identity/request and the intended
//! Security delta. It is not a second canonical state store. Recovery resolves
//! the Application binding: committed => idempotently apply Security delta;
//! absent => discard the prepared marker.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dxbot_core::types::OperationRequest;
use runtime_security::SecurityDelta;
use serde::{Deserialize, Serialize};

const COORDINATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityCoordinationRecord {
    pub request: OperationRequest,
    pub delta: SecurityDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Envelope {
    schema_version: u32,
    record: SecurityCoordinationRecord,
}

#[derive(Debug, Clone)]
pub struct SecurityCoordinationStore {
    path: PathBuf,
}

impl SecurityCoordinationStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Option<SecurityCoordinationRecord>, io::Error> {
        match fs::read(&self.path) {
            Ok(bytes) => {
                let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("cannot decode security coordination marker: {error}"),
                    )
                })?;
                if envelope.schema_version != COORDINATION_SCHEMA_VERSION {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "unsupported security coordination schema {}",
                            envelope.schema_version
                        ),
                    ));
                }
                Ok(Some(envelope.record))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn prepare(&self, record: &SecurityCoordinationRecord) -> Result<(), io::Error> {
        if self.load()?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "unrecovered security coordination marker exists",
            ));
        }
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(&Envelope {
            schema_version: COORDINATION_SCHEMA_VERSION,
            record: record.clone(),
        })
        .map_err(|error| io::Error::other(format!("cannot encode coordination marker: {error}")))?;
        let temp = temp_path(&self.path);
        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                file.set_permissions(fs::Permissions::from_mode(0o600))?;
            }
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)?;
            File::open(parent)?.sync_all()?;
            Ok::<(), io::Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    pub fn clear(&self) -> Result<(), io::Error> {
        match fs::remove_file(&self.path) {
            Ok(()) => {
                let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
                File::open(parent)?.sync_all()
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("security-coordination.json");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce))
}

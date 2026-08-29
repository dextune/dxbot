//! Durable store of parked high-risk operation requests awaiting approval.
//!
//! A high-risk operation is *parked* before any Application mutation: the exact
//! original [`OperationRequest`] (identity + payload + request digest) is written
//! here, keyed by the gating [`ApprovalId`], and the security-owned gate/approval
//! records the binding. On approved wakeup the coordinator reloads the parked
//! request and re-evaluates it against current CAS, applying it exactly once.
//!
//! This store holds the replayable *request*, not Domain rows and not a second
//! canonical Approval/gate. It is the control-plane's continuation queue; the
//! Approval decision and parked-gate lifecycle remain canonical in
//! runtime-security. Records are removed only after a terminal continuation
//! (commit/conflict) or a denial, so a crash mid-continuation always recovers.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use dxbot_core::types::OperationRequest;
use serde::{Deserialize, Serialize};

const PARKING_SCHEMA_VERSION: u32 = 1;

/// One parked operation awaiting its approval decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedOperationRecord {
    /// ApprovalId whose terminal decision gates this parked request.
    pub approval_id: String,
    /// The exact original request to replay on approval.
    pub request: OperationRequest,
    /// Policy generation the parking classification was decided under.
    pub policy_generation: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Envelope {
    schema_version: u32,
    records: BTreeMap<String, ParkedOperationRecord>,
}

/// Durable, atomically-rewritten map of parked operations keyed by ApprovalId.
#[derive(Debug, Clone)]
pub struct ParkedOperationStore {
    path: PathBuf,
}

impl ParkedOperationStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load_all(&self) -> Result<BTreeMap<String, ParkedOperationRecord>, io::Error> {
        match fs::symlink_metadata(&self.path) {
            Ok(metadata)
                if metadata.file_type().is_symlink() || !metadata.file_type().is_file() =>
            {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "parked operation store is not a direct regular file: {}",
                        self.path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(error) => return Err(error),
        }
        let bytes = fs::read(&self.path)?;
        let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("cannot decode parked operation store: {error}"),
            )
        })?;
        if envelope.schema_version != PARKING_SCHEMA_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported parked operation schema {}",
                    envelope.schema_version
                ),
            ));
        }
        Ok(envelope.records)
    }

    pub fn get(&self, approval_id: &str) -> Result<Option<ParkedOperationRecord>, io::Error> {
        Ok(self.load_all()?.remove(approval_id))
    }

    /// Insert a parked record. Idempotent for a byte-identical replay; a
    /// conflicting re-park under the same ApprovalId fails closed.
    pub fn insert(&self, record: &ParkedOperationRecord) -> Result<(), io::Error> {
        let mut records = self.load_all()?;
        match records.get(&record.approval_id) {
            Some(existing) if existing == record => return Ok(()),
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    format!(
                        "parked operation already exists for approval {}",
                        record.approval_id
                    ),
                ));
            }
            None => {}
        }
        records.insert(record.approval_id.clone(), record.clone());
        self.write_all(&records)
    }

    /// Remove a parked record (terminal continuation or denial). Removing an
    /// absent record is a safe no-op so recovery is idempotent.
    pub fn remove(&self, approval_id: &str) -> Result<(), io::Error> {
        let mut records = self.load_all()?;
        if records.remove(approval_id).is_none() {
            return Ok(());
        }
        self.write_all(&records)
    }

    fn write_all(
        &self,
        records: &BTreeMap<String, ParkedOperationRecord>,
    ) -> Result<(), io::Error> {
        let parent = self.path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(&Envelope {
            schema_version: PARKING_SCHEMA_VERSION,
            records: records.clone(),
        })
        .map_err(|error| {
            io::Error::other(format!("cannot encode parked operation store: {error}"))
        })?;
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
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("parked-operations.json");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), nonce))
}

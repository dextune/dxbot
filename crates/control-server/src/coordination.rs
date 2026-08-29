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
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.file_type().is_file() => {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "security coordination marker is not a direct regular file: {}",
                        self.path.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        }
        let bytes = fs::read(&self.path)?;
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
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.file_type().is_file() => {
                Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "refusing to remove unexpected coordination path: {}",
                        self.path.display()
                    ),
                ))
            }
            Ok(_) => {
                fs::remove_file(&self.path)?;
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use dxbot_core::types::{
        CanonicalTarget, CommandId, CommandPayload, IdempotencyKey, InstanceId, OperationId,
        PrincipalRef, RequestDigest,
    };
    use runtime_security::SecurityAuditIntent;
    use serde_json::json;

    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dxbot-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    fn record() -> SecurityCoordinationRecord {
        let principal = PrincipalRef("local:i:uid:1000".to_owned());
        let operation_id = OperationId("operation-a".to_owned());
        SecurityCoordinationRecord {
            request: OperationRequest {
                command_id: CommandId("command-a".to_owned()),
                idempotency_key: IdempotencyKey {
                    principal_ref: principal.clone(),
                    key_digest: "key-a".to_owned(),
                    expires_at: 99,
                },
                request_digest: RequestDigest("digest-a".to_owned()),
                new_operation_id: operation_id.clone(),
                payload: CommandPayload {
                    command_key: "project-member-set".to_owned(),
                    principal_ref: principal.clone(),
                    instance_id: InstanceId("i".to_owned()),
                    canonical_target: CanonicalTarget::Instance(InstanceId("i".to_owned())),
                    cas: None,
                    content: None,
                    semantic_options: json!({}),
                },
            },
            delta: SecurityDelta::audit_only(SecurityAuditIntent {
                operation_id,
                principal_ref: principal,
                action: "project-member-set".to_owned(),
                target: "project:a".to_owned(),
                created_at: 1,
            }),
        }
    }

    #[test]
    fn marker_roundtrip_is_single_slot_and_clearable() {
        let root = temp_root("coordination-roundtrip");
        fs::create_dir_all(&root).expect("temp root");
        let store = SecurityCoordinationStore::new(root.join("marker.json"));
        let record = record();

        assert!(store.load().expect("empty load").is_none());
        store.prepare(&record).expect("prepare");
        assert_eq!(store.load().expect("load"), Some(record.clone()));
        assert_eq!(
            store.prepare(&record).expect_err("second prepare fails").kind(),
            io::ErrorKind::AlreadyExists
        );
        store.clear().expect("clear");
        assert!(store.load().expect("empty after clear").is_none());
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn marker_refuses_symlink_paths() {
        use std::os::unix::fs::symlink;

        let root = temp_root("coordination-symlink");
        fs::create_dir_all(&root).expect("temp root");
        let target = root.join("target.json");
        fs::write(&target, b"not-a-marker").expect("target");
        let marker = root.join("marker.json");
        symlink(&target, &marker).expect("symlink");
        let store = SecurityCoordinationStore::new(marker);
        assert_eq!(
            store.load().expect_err("symlink rejected").kind(),
            io::ErrorKind::PermissionDenied
        );
        assert_eq!(
            store.clear().expect_err("symlink clear rejected").kind(),
            io::ErrorKind::PermissionDenied
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}

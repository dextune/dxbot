//! Durable, bounded audit projection reconstructed from canonical producer intents.
//!
//! Producers retain canonical committed intent in their own Unit of Work. This
//! outbox owns only the sequenced, redacted, integrity-checked observation path.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use dxbot_core::types::{OperationId, PrincipalRef};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::audit::{Error, RedactionLevel, Result, redact_for_storage};

const SCHEMA_VERSION: u32 = 1;
const MAX_RECORDS: usize = 100_000;
const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
const OWNER_DIRECTORY_MODE: u32 = 0o700;
const OWNER_FILE_MODE: u32 = 0o600;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditIntent {
    pub key: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_id: Option<OperationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_ref: Option<PrincipalRef>,
    pub action: String,
    pub target_ref: String,
    pub detail: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableAuditRecord {
    pub sequence: u64,
    pub intent: AuditIntent,
    pub redaction_level: RedactionLevel,
    pub previous_digest: String,
    pub record_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutboxSnapshot {
    schema_version: u32,
    records: Vec<DurableAuditRecord>,
}

#[derive(Debug)]
pub struct AuditOutbox {
    path: PathBuf,
    records: Mutex<Vec<DurableAuditRecord>>,
}

impl AuditOutbox {
    pub fn open(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
            harden_directory(parent)?;
        }
        let records = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                validate_file(&path, &metadata)?;
                let bytes = fs::read(&path).map_err(io_error)?;
                if bytes.len() > MAX_SNAPSHOT_BYTES {
                    return Err(Error::CapacityExceeded);
                }
                let snapshot: OutboxSnapshot =
                    serde_json::from_slice(&bytes).map_err(|_| Error::CorruptLog)?;
                if snapshot.schema_version != SCHEMA_VERSION || snapshot.records.len() > MAX_RECORDS
                {
                    return Err(Error::CorruptLog);
                }
                validate_chain(&snapshot.records)?;
                snapshot.records
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(io_error(error)),
        };
        Ok(Self {
            path,
            records: Mutex::new(records),
        })
    }

    pub fn append_if_absent(&self, intent: &AuditIntent) -> Result<DurableAuditRecord> {
        validate_intent(intent)?;
        let mut records = self.lock()?;
        if let Some(existing) = records
            .iter()
            .find(|record| record.intent.key == intent.key)
        {
            let redacted = redact_intent(intent);
            if existing.intent == redacted.0 && existing.redaction_level == redacted.1 {
                return Ok(existing.clone());
            }
            return Err(Error::InvalidInput(format!(
                "audit intent key conflicts with committed record: {}",
                intent.key
            )));
        }
        if records.len() >= MAX_RECORDS {
            return Err(Error::CapacityExceeded);
        }
        let (intent, redaction_level) = redact_intent(intent);
        let sequence = u64::try_from(records.len()).map_err(|_| Error::CapacityExceeded)?;
        let previous_digest = records
            .last()
            .map(|record| record.record_digest.clone())
            .unwrap_or_else(|| "0".repeat(64));
        let mut record = DurableAuditRecord {
            sequence,
            intent,
            redaction_level,
            previous_digest,
            record_digest: String::new(),
        };
        record.record_digest = digest_record(&record)?;
        let mut candidate = records.clone();
        candidate.push(record.clone());
        persist(&self.path, &candidate)?;
        *records = candidate;
        Ok(record)
    }

    pub fn records(&self) -> Result<Vec<DurableAuditRecord>> {
        let records = self.lock()?;
        validate_chain(&records)?;
        Ok(records.clone())
    }

    pub fn record_count(&self) -> Result<usize> {
        self.records().map(|records| records.len())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> Result<MutexGuard<'_, Vec<DurableAuditRecord>>> {
        self.records.lock().map_err(|_| Error::CorruptLog)
    }
}

fn validate_intent(intent: &AuditIntent) -> Result<()> {
    if intent.key.trim().is_empty()
        || intent.key.len() > 512
        || intent.source.trim().is_empty()
        || intent.source.len() > 64
        || intent.action.trim().is_empty()
        || intent.action.len() > 128
        || intent.target_ref.len() > 1024
        || intent.detail.len() > 4096
        || intent.created_at < 0
    {
        return Err(Error::InvalidInput(
            "audit intent exceeds required field bounds".to_owned(),
        ));
    }
    Ok(())
}

fn redact_intent(intent: &AuditIntent) -> (AuditIntent, RedactionLevel) {
    let combined = format!("{} {} {}", intent.action, intent.target_ref, intent.detail);
    let (redacted, level) = redact_for_storage(&combined);
    let mut safe = intent.clone();
    match level {
        RedactionLevel::None => {}
        RedactionLevel::Partial | RedactionLevel::Full => {
            safe.action = "redacted-audit-event".to_owned();
            safe.target_ref = "[REDACTED]".to_owned();
            safe.detail = redacted;
        }
    }
    (safe, level)
}

fn validate_chain(records: &[DurableAuditRecord]) -> Result<()> {
    let mut previous = "0".repeat(64);
    for (index, record) in records.iter().enumerate() {
        if record.sequence != index as u64 || record.previous_digest != previous {
            return Err(Error::CorruptLog);
        }
        let digest = digest_record(record)?;
        if record.record_digest != digest {
            return Err(Error::CorruptLog);
        }
        previous = record.record_digest.clone();
    }
    Ok(())
}

fn digest_record(record: &DurableAuditRecord) -> Result<String> {
    #[derive(Serialize)]
    struct DigestInput<'a> {
        sequence: u64,
        intent: &'a AuditIntent,
        redaction_level: RedactionLevel,
        previous_digest: &'a str,
    }
    let bytes = serde_json::to_vec(&DigestInput {
        sequence: record.sequence,
        intent: &record.intent,
        redaction_level: record.redaction_level,
        previous_digest: &record.previous_digest,
    })
    .map_err(|error| Error::InvalidInput(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn persist(path: &Path, records: &[DurableAuditRecord]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::Io("audit outbox path has no parent".to_owned()))?;
    let bytes = serde_json::to_vec(&OutboxSnapshot {
        schema_version: SCHEMA_VERSION,
        records: records.to_vec(),
    })
    .map_err(|error| Error::InvalidInput(error.to_string()))?;
    if bytes.len() > MAX_SNAPSHOT_BYTES {
        return Err(Error::CapacityExceeded);
    }
    let temp = parent.join(format!(
        ".audit-outbox.{}.{}.tmp",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(io_error)?;
    harden_file(&temp)?;
    if let Err(error) = write_outbox_bytes(&mut file, &bytes, path).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(io_error(error));
    }
    drop(file);
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(io_error(error));
    }
    harden_file(path)?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(io_error)?;
    Ok(())
}

fn write_outbox_bytes(file: &mut File, bytes: &[u8], _target: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if test_storage_full_path()
        .lock()
        .is_ok_and(|path| path.as_ref() == Some(&_target.to_path_buf()))
    {
        file.write_all(&bytes[..bytes.len().min(32)])?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "injected Runtime Audit storage-full fault",
        ));
    }
    file.write_all(bytes)
}

#[cfg(test)]
fn test_storage_full_path() -> &'static Mutex<Option<PathBuf>> {
    static PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    PATH.get_or_init(|| Mutex::new(None))
}
fn validate_file(path: &Path, metadata: &fs::Metadata) -> Result<()> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(Error::Io(format!(
            "audit outbox is not a direct regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error::Io(format!(
            "audit outbox permissions are not owner-only: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_DIRECTORY_MODE)).map_err(io_error)
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_FILE_MODE)).map_err(io_error)
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<()> {
    Ok(())
}

fn io_error(error: std::io::Error) -> Error {
    Error::Io(error.to_string())
}

#[cfg(test)]
mod storage_full_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn intent(key: &str) -> AuditIntent {
        AuditIntent {
            key: key.to_owned(),
            source: "application".to_owned(),
            operation_id: None,
            principal_ref: None,
            action: "execution-running".to_owned(),
            target_ref: "execution:test".to_owned(),
            detail: "bounded".to_owned(),
            created_at: 1,
        }
    }

    #[test]
    fn storage_full_during_temp_write_preserves_prior_digest_chain() {
        let root = std::env::temp_dir().join(format!(
            "dxbot-audit-enospc-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let path = root.join("audit-outbox.json");
        let outbox = AuditOutbox::open(path.clone()).expect("open");
        outbox.append_if_absent(&intent("first")).expect("baseline");
        let baseline = fs::read(&path).expect("baseline bytes");
        *test_storage_full_path().lock().expect("fault lock") = Some(path.clone());
        let error = outbox
            .append_if_absent(&intent("second"))
            .expect_err("storage full");
        *test_storage_full_path().lock().expect("fault lock") = None;
        assert!(matches!(error, Error::Io(message) if message.contains("storage-full")));
        assert_eq!(outbox.record_count().expect("in-memory count"), 1);
        assert_eq!(fs::read(&path).expect("durable bytes"), baseline);
        drop(outbox);
        assert_eq!(
            AuditOutbox::open(path.clone())
                .expect("reopen")
                .record_count()
                .unwrap(),
            1
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}

//! Runtime-Host-owned Application snapshot v1→v2 migration.
//!
//! The canonical Application persistence loader (`application::ApplicationStateStore`)
//! is fixed at snapshot version 2 and fails closed on any other version. Older
//! deployments may still hold an on-disk `application-state.json` written at
//! version 1 (before the additive `executions` / `processes` /
//! `execution_audit_intents` fields existed). This owner performs the one-way
//! v1→v2 migration while the Runtime Host holds the exclusive lock and the
//! Runtime is offline, *before* the Application store is opened.
//!
//! Migration is fail-closed and crash-safe:
//! - a pre-migration backup of the whole consistent artifact set is captured
//!   first, so a rollback always has a byte-for-byte source
//! - the rewrite uses the durable temp + fsync + rename + directory-fsync
//!   discipline via [`OfflineStorage`]-compatible writes
//! - additive v2 array fields are materialized as empty arrays, which is
//!   exactly what the v2 loader's `serde(default)` would have produced, so an
//!   already-v2 snapshot (including one carrying additive fields) is left
//!   untouched and still loads
//!
//! This module never widens the canonical loader's accepted version set; it
//! only makes a v1 artifact become a valid v2 artifact on disk.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::offline_storage::{OfflineStorage, OfflineStorageError};

/// The canonical Application snapshot version this runtime understands.
const TARGET_SNAPSHOT_VERSION: u64 = 2;
/// Read ceiling for the Application snapshot during migration inspection.
/// Mirrors the canonical Application loader's 64 MiB ceiling.
const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024 * 1024;
/// Additive array fields introduced by (or defaulted in) snapshot version 2.
const V2_ADDITIVE_ARRAY_FIELDS: &[&str] = &["executions", "processes", "execution_audit_intents"];

/// Outcome of a migration attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationOutcome {
    /// No `application-state.json` present; nothing to migrate.
    Absent,
    /// The snapshot was already at the target version; left untouched.
    AlreadyCurrent,
    /// A v1 snapshot was migrated to v2 after a pre-migration backup.
    Migrated {
        /// Directory holding the pre-migration backup set for rollback.
        backup_dir: PathBuf,
    },
}

/// Errors from the migration owner.
#[derive(Debug)]
pub enum MigrationError {
    /// The snapshot was not a JSON object with a numeric `version`.
    Malformed(String),
    /// The on-disk version is newer than this runtime understands.
    UnsupportedVersion(u64),
    /// The pre-migration backup or a durable write failed.
    Storage(OfflineStorageError),
    /// Underlying filesystem I/O failure.
    Io(String),
}

impl std::fmt::Display for MigrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(detail) => {
                write!(formatter, "application snapshot is malformed: {detail}")
            }
            Self::UnsupportedVersion(version) => write!(
                formatter,
                "application snapshot version {version} is newer than supported {TARGET_SNAPSHOT_VERSION}"
            ),
            Self::Storage(error) => write!(formatter, "migration storage failure: {error}"),
            Self::Io(detail) => write!(formatter, "migration I/O: {detail}"),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<OfflineStorageError> for MigrationError {
    fn from(error: OfflineStorageError) -> Self {
        Self::Storage(error)
    }
}

/// Migrate the Application snapshot in `runtime_root` to the current version if
/// needed. Must be called while the Runtime Host owns the lock and no server is
/// bound (offline fence). `backups_root` is where the pre-migration backup set
/// is written; it is created owner-only on demand.
pub fn migrate_application_state(
    runtime_root: &Path,
    backups_root: &Path,
) -> Result<MigrationOutcome, MigrationError> {
    let storage = OfflineStorage::open_offline(runtime_root.to_path_buf())?;
    migrate_application_state_with_storage(runtime_root, backups_root, &storage)
}

/// Startup path used only after `LocalRuntimeHost` acquired the exclusive host
/// lock and before any endpoint is published.
pub(crate) fn migrate_application_state_under_owned_lock(
    runtime_root: &Path,
    backups_root: &Path,
) -> Result<MigrationOutcome, MigrationError> {
    let storage = OfflineStorage::under_owned_host_lock(runtime_root.to_path_buf());
    migrate_application_state_with_storage(runtime_root, backups_root, &storage)
}

fn migrate_application_state_with_storage(
    runtime_root: &Path,
    backups_root: &Path,
    storage: &OfflineStorage,
) -> Result<MigrationOutcome, MigrationError> {
    let snapshot_path = runtime_root.join("application-state.json");
    let metadata = match fs::symlink_metadata(&snapshot_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(MigrationOutcome::Absent);
        }
        Err(error) => return Err(MigrationError::Io(error.to_string())),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(MigrationError::Malformed(format!(
            "application snapshot is not a direct regular file: {}",
            snapshot_path.display()
        )));
    }
    if metadata.len() > MAX_SNAPSHOT_BYTES {
        return Err(MigrationError::Malformed(
            "application snapshot exceeds bounded read ceiling".to_owned(),
        ));
    }

    let bytes = fs::read(&snapshot_path).map_err(|error| MigrationError::Io(error.to_string()))?;
    let mut document: Value = serde_json::from_slice(&bytes)
        .map_err(|error| MigrationError::Malformed(error.to_string()))?;

    let version = document
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            MigrationError::Malformed("snapshot has no numeric version field".to_owned())
        })?;

    if version == TARGET_SNAPSHOT_VERSION {
        return Ok(MigrationOutcome::AlreadyCurrent);
    }
    if version > TARGET_SNAPSHOT_VERSION {
        return Err(MigrationError::UnsupportedVersion(version));
    }
    if version != 1 {
        return Err(MigrationError::UnsupportedVersion(version));
    }

    // Pre-migration backup of the whole consistent artifact set. Rollback can
    // restore byte-for-byte from this directory.
    let backup_dir = backups_root.join(format!("pre-migration-v1-{}", now_nanos()));
    storage.capture(&backup_dir, now_secs())?;

    // Transform v1 → v2: bump the version and materialize additive array fields
    // as empty arrays, exactly mirroring the v2 loader's `serde(default)`.
    let object = document.as_object_mut().ok_or_else(|| {
        MigrationError::Malformed("snapshot root is not a JSON object".to_owned())
    })?;
    object.insert("version".to_owned(), Value::from(TARGET_SNAPSHOT_VERSION));
    for field in V2_ADDITIVE_ARRAY_FIELDS {
        object
            .entry((*field).to_owned())
            .or_insert_with(|| Value::Array(Vec::new()));
    }

    let migrated_bytes = serde_json::to_vec(&document)
        .map_err(|error| MigrationError::Io(format!("cannot encode migrated snapshot: {error}")))?;
    durable_write(&snapshot_path, &migrated_bytes)?;

    Ok(MigrationOutcome::Migrated { backup_dir })
}

/// Durable in-place rewrite of a state artifact: temp + fsync + rename +
/// directory fsync, owner-only permissions throughout.
fn durable_write(path: &Path, bytes: &[u8]) -> Result<(), MigrationError> {
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let parent = path
        .parent()
        .ok_or_else(|| MigrationError::Io("snapshot path has no parent".to_owned()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| MigrationError::Io("snapshot path has no file name".to_owned()))?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let temp = parent.join(format!(
        ".{file_name}.migrate.{}.{nonce}.tmp",
        std::process::id()
    ));

    let write = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        #[cfg(unix)]
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)?;
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if let Err(error) = write {
        let _ = fs::remove_file(&temp);
        return Err(MigrationError::Io(error.to_string()));
    }
    Ok(())
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

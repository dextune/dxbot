//! Runtime-Host-owned offline backup/restore for canonical durable state.
//!
//! This is the Runtime Host owner for cold-path operational protection of the
//! actual production state artifacts (`application-state.json`,
//! `security-state.json`) and the coordination/audit artifacts that must move
//! with them for a consistent restore. It is **not** a second canonical store
//! and never runs while the Runtime is serving: capture and restore are only
//! valid while the caller holds the exclusive Runtime Host lock and the
//! Runtime is offline (no live server bound).
//!
//! Invariants enforced here:
//! - owner-only directories (`0o700`) and files (`0o600`)
//! - every read validates a direct regular file (no symlink, no special file)
//!   and is bounded by an explicit byte ceiling
//! - each artifact is content-addressed with a SHA-256 digest recorded in a
//!   bounded integrity-checked manifest; the manifest itself carries a digest of its own
//!   entries so a corrupted manifest fails closed
//! - durability uses the temp + fsync + rename + directory-fsync discipline
//! - restore has an explicit boundary: [`RestoreMode::DryRun`] only plans and
//!   verifies digests; [`RestoreMode::Apply`] is the sole authorized mutation
//!   path and still refuses to run on any digest mismatch (fail closed)

use std::fs::{self, File, OpenOptions};
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Manifest schema version for the offline backup set.
const BACKUP_MANIFEST_VERSION: u32 = 1;
/// Manifest file name inside a backup directory.
const MANIFEST_FILE: &str = "backup-manifest.json";
/// Owner-only directory mode.
const OWNER_DIRECTORY_MODE: u32 = 0o700;
/// Owner-only file mode.
const OWNER_FILE_MODE: u32 = 0o600;
/// Per-artifact read ceiling. Matches the largest canonical snapshot ceiling
/// (Application state at 64 MiB) so a legitimate artifact always fits while an
/// unbounded/hostile file is rejected before it is read into memory.
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
/// Manifest read ceiling. The manifest only holds bounded metadata rows.
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Logical identity of a durable artifact within the runtime root. The name is
/// the stable on-disk file name; membership in a backup set is decided by the
/// artifact being present at capture time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtifactSpec {
    /// On-disk file name relative to the runtime root.
    pub file_name: &'static str,
    /// Whether the artifact must exist for a capture to be considered complete.
    pub required: bool,
}

/// Canonical durable artifacts that a consistent offline backup must move as a
/// set. Application and Security state are required; the coordination and audit
/// artifacts are included when present so a restore reproduces cross-owner
/// coordination and the audit observation window byte-for-byte.
pub const BACKUP_ARTIFACTS: &[ArtifactSpec] = &[
    ArtifactSpec {
        file_name: "application-state.json",
        required: true,
    },
    ArtifactSpec {
        file_name: "security-state.json",
        required: true,
    },
    ArtifactSpec {
        file_name: ".security-application-uow.json",
        required: false,
    },
    ArtifactSpec {
        file_name: "audit-outbox.json",
        required: false,
    },
];

/// Errors from the offline backup/restore boundary.
#[derive(Debug)]
pub enum OfflineStorageError {
    /// An artifact required for a consistent set was missing at capture time.
    MissingRequiredArtifact(String),
    /// A path referred to something other than a direct regular file.
    NotRegularFile(String),
    /// A manifest entry named a file that is not a known backup artifact (or
    /// carried path separators / traversal components). Trusting such a name
    /// would allow a crafted manifest to read or write outside the intended
    /// artifact set, so it fails closed.
    UnknownArtifact(String),
    /// An artifact or manifest exceeded its bounded read ceiling.
    ArtifactTooLarge(String),
    /// A recorded SHA-256 digest did not match the observed bytes.
    DigestMismatch { artifact: String },
    /// The manifest self-digest did not match its recorded entries.
    ManifestCorrupt(String),
    /// The manifest schema version is not understood by this owner.
    UnsupportedManifestVersion(u32),
    /// Serialization of the manifest failed.
    Encode(String),
    /// Underlying filesystem I/O failure.
    Io(String),
}

impl std::fmt::Display for OfflineStorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredArtifact(name) => {
                write!(formatter, "required backup artifact is missing: {name}")
            }
            Self::NotRegularFile(path) => {
                write!(formatter, "path is not a direct regular file: {path}")
            }
            Self::UnknownArtifact(name) => {
                write!(
                    formatter,
                    "manifest names a file that is not a known backup artifact: {name}"
                )
            }
            Self::ArtifactTooLarge(name) => {
                write!(formatter, "artifact exceeds bounded read ceiling: {name}")
            }
            Self::DigestMismatch { artifact } => {
                write!(
                    formatter,
                    "SHA-256 digest mismatch for artifact: {artifact}"
                )
            }
            Self::ManifestCorrupt(detail) => {
                write!(formatter, "backup manifest is corrupt: {detail}")
            }
            Self::UnsupportedManifestVersion(version) => {
                write!(formatter, "unsupported backup manifest version {version}")
            }
            Self::Encode(detail) => write!(formatter, "cannot encode backup manifest: {detail}"),
            Self::Io(detail) => write!(formatter, "offline storage I/O: {detail}"),
        }
    }
}

impl std::error::Error for OfflineStorageError {}

impl From<std::io::Error> for OfflineStorageError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

/// One recorded artifact in a backup manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// On-disk file name relative to the runtime root.
    pub file_name: String,
    /// Byte length of the captured artifact.
    pub byte_len: u64,
    /// Lowercase hex SHA-256 of the artifact bytes.
    pub sha256: String,
}

/// Signed-shape manifest describing a consistent backup set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    /// Manifest schema version.
    pub version: u32,
    /// Wall-clock capture time in Unix seconds (advisory metadata only).
    pub captured_at: i64,
    /// Ordered artifact digests (sorted by `file_name`).
    pub entries: Vec<ManifestEntry>,
    /// SHA-256 over the ordered entries, guarding manifest integrity itself.
    pub manifest_digest: String,
}

impl BackupManifest {
    fn build(captured_at: i64, mut entries: Vec<ManifestEntry>) -> Self {
        entries.sort_by(|left, right| left.file_name.cmp(&right.file_name));
        let manifest_digest = digest_entries(&entries);
        Self {
            version: BACKUP_MANIFEST_VERSION,
            captured_at,
            entries,
            manifest_digest,
        }
    }

    /// Verify the manifest self-integrity. Fails closed on any mismatch.
    pub fn verify_self(&self) -> Result<(), OfflineStorageError> {
        if self.version != BACKUP_MANIFEST_VERSION {
            return Err(OfflineStorageError::UnsupportedManifestVersion(
                self.version,
            ));
        }
        if digest_entries(&self.entries) != self.manifest_digest {
            return Err(OfflineStorageError::ManifestCorrupt(
                "manifest digest does not match recorded entries".to_owned(),
            ));
        }
        Ok(())
    }
}

/// Restore boundary: planning/verification vs. authorized mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreMode {
    /// Verify every artifact digest against the manifest without writing to the
    /// runtime root. This is the default, side-effect-free planning path.
    DryRun,
    /// Authorized apply. Verifies every digest first, then atomically installs
    /// each artifact into the runtime root. Refuses to write anything if any
    /// digest fails to verify.
    Apply,
}

/// Outcome of a restore invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreOutcome {
    /// The mode the restore ran in.
    pub mode: RestoreMode,
    /// Artifacts whose digests were verified against the manifest.
    pub verified: Vec<String>,
    /// Artifacts written into the runtime root (empty in dry-run).
    pub applied: Vec<String>,
}

/// Runtime-Host-owned offline backup/restore facade. Construct only while the
/// caller holds the exclusive Runtime Host lock and the Runtime is offline.
#[derive(Debug)]
pub struct OfflineStorage {
    runtime_root: PathBuf,
    _host_lock: Option<File>,
}

impl OfflineStorage {
    /// Open the offline storage boundary and acquire the same exclusive lock
    /// used by `LocalRuntimeHost`. Backup/restore cannot race a serving Runtime.
    pub fn open_offline(runtime_root: PathBuf) -> Result<Self, OfflineStorageError> {
        fs::create_dir_all(&runtime_root)?;
        harden_directory(&runtime_root)?;
        let lock_path = runtime_root.join(".runtime-host.lock");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        harden_path(&lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            OfflineStorageError::Io(format!(
                "Runtime Host is active; offline storage lock unavailable: {error}"
            ))
        })?;
        Ok(Self {
            runtime_root,
            _host_lock: Some(lock),
        })
    }

    /// Bind while `LocalRuntimeHost` already owns the exclusive lock. This is
    /// crate-private so external callers cannot bypass the offline fence.
    pub(crate) fn under_owned_host_lock(runtime_root: PathBuf) -> Self {
        Self {
            runtime_root,
            _host_lock: None,
        }
    }

    /// Capture a consistent backup set into `backup_dir`. Required artifacts
    /// must be present; optional artifacts are captured only when present. The
    /// directory is created owner-only and every file is written with the
    /// durable temp + fsync + rename + directory-fsync discipline.
    pub fn capture(
        &self,
        backup_dir: &Path,
        captured_at: i64,
    ) -> Result<BackupManifest, OfflineStorageError> {
        fs::create_dir_all(backup_dir)?;
        harden_directory(backup_dir)?;

        let mut entries = Vec::with_capacity(BACKUP_ARTIFACTS.len());
        for spec in BACKUP_ARTIFACTS {
            let source = self.runtime_root.join(spec.file_name);
            let bytes = match read_bounded(&source, MAX_ARTIFACT_BYTES) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => {
                    if spec.required {
                        return Err(OfflineStorageError::MissingRequiredArtifact(
                            spec.file_name.to_owned(),
                        ));
                    }
                    continue;
                }
                Err(error) => return Err(error),
            };
            let sha256 = digest_bytes(&bytes);
            let target = backup_dir.join(spec.file_name);
            atomic_write(&target, &bytes)?;
            entries.push(ManifestEntry {
                file_name: spec.file_name.to_owned(),
                byte_len: bytes.len() as u64,
                sha256,
            });
        }

        let manifest = BackupManifest::build(captured_at, entries);
        let manifest_bytes = serde_json::to_vec(&manifest)
            .map_err(|error| OfflineStorageError::Encode(error.to_string()))?;
        atomic_write(&backup_dir.join(MANIFEST_FILE), &manifest_bytes)?;
        Ok(manifest)
    }

    /// Load and self-verify a manifest from a backup directory.
    pub fn load_manifest(&self, backup_dir: &Path) -> Result<BackupManifest, OfflineStorageError> {
        let path = backup_dir.join(MANIFEST_FILE);
        let bytes = read_bounded(&path, MAX_MANIFEST_BYTES)?.ok_or_else(|| {
            OfflineStorageError::ManifestCorrupt("manifest is missing".to_owned())
        })?;
        let manifest: BackupManifest = serde_json::from_slice(&bytes)
            .map_err(|error| OfflineStorageError::ManifestCorrupt(error.to_string()))?;
        manifest.verify_self()?;
        // The manifest self-digest only detects accidental corruption; it is
        // not an authenticated MAC, so a crafted manifest can carry a matching
        // digest. Independently constrain every entry name to the known backup
        // artifact set (all plain, separator-free file names). This closes any
        // path-traversal write/read primitive on the restore path.
        for entry in &manifest.entries {
            if !is_known_artifact_name(&entry.file_name) {
                return Err(OfflineStorageError::UnknownArtifact(
                    entry.file_name.clone(),
                ));
            }
        }
        Ok(manifest)
    }

    /// Restore a backup set. In [`RestoreMode::DryRun`] every artifact digest is
    /// verified against the (self-verified) manifest and nothing is written. In
    /// [`RestoreMode::Apply`] every digest is verified first and only then are
    /// artifacts atomically installed into the runtime root. Any digest
    /// mismatch fails closed with no partial apply.
    pub fn restore(
        &self,
        backup_dir: &Path,
        mode: RestoreMode,
    ) -> Result<RestoreOutcome, OfflineStorageError> {
        let manifest = self.load_manifest(backup_dir)?;

        // Phase 1: verify every artifact digest before any mutation.
        let mut verified = Vec::with_capacity(manifest.entries.len());
        let mut staged: Vec<(String, Vec<u8>)> = Vec::with_capacity(manifest.entries.len());
        for entry in &manifest.entries {
            let source = backup_dir.join(&entry.file_name);
            let bytes = read_bounded(&source, MAX_ARTIFACT_BYTES)?.ok_or_else(|| {
                OfflineStorageError::MissingRequiredArtifact(entry.file_name.clone())
            })?;
            if bytes.len() as u64 != entry.byte_len || digest_bytes(&bytes) != entry.sha256 {
                return Err(OfflineStorageError::DigestMismatch {
                    artifact: entry.file_name.clone(),
                });
            }
            verified.push(entry.file_name.clone());
            if mode == RestoreMode::Apply {
                staged.push((entry.file_name.clone(), bytes));
            }
        }

        // Phase 2: authorized apply only. Digests are already verified.
        let mut applied = Vec::new();
        if mode == RestoreMode::Apply {
            fs::create_dir_all(&self.runtime_root)?;
            harden_directory(&self.runtime_root)?;
            for (file_name, bytes) in staged {
                atomic_write(&self.runtime_root.join(&file_name), &bytes)?;
                applied.push(file_name);
            }
        }

        Ok(RestoreOutcome {
            mode,
            verified,
            applied,
        })
    }

    /// Runtime root this owner is bound to.
    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }
}

/// Read a file into memory only if it is a direct regular file within the
/// bounded ceiling. Returns `Ok(None)` when the path does not exist so optional
/// artifacts can be skipped without ambiguity.
fn read_bounded(path: &Path, max_bytes: u64) -> Result<Option<Vec<u8>>, OfflineStorageError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(OfflineStorageError::NotRegularFile(
            path.display().to_string(),
        ));
    }
    if metadata.len() > max_bytes {
        return Err(OfflineStorageError::ArtifactTooLarge(
            path.display().to_string(),
        ));
    }
    let bytes = fs::read(path)?;
    // Re-check the length actually read to defend against concurrent growth.
    if bytes.len() as u64 > max_bytes {
        return Err(OfflineStorageError::ArtifactTooLarge(
            path.display().to_string(),
        ));
    }
    Ok(Some(bytes))
}

/// Durable write: temp file (owner-only) + fsync + rename + directory fsync.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), OfflineStorageError> {
    let parent = path.parent().ok_or_else(|| {
        OfflineStorageError::Io(format!("path has no parent: {}", path.display()))
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            OfflineStorageError::Io(format!("path has no file name: {}", path.display()))
        })?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temp = parent.join(format!(
        ".{file_name}.{}.{sequence}.tmp",
        std::process::id()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    harden_open_file(&file)?;
    if let Err(error) = write_atomic_bytes(&mut file, bytes, path).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    drop(file);
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);

        return Err(error.into());
    }
    harden_path(path)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}

fn write_atomic_bytes(file: &mut File, bytes: &[u8], _target: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if test_storage_full_path()
        .lock()
        .is_ok_and(|path| path.as_ref() == Some(&_target.to_path_buf()))
    {
        file.write_all(&bytes[..bytes.len().min(32)])?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "injected offline storage-full fault",
        ));
    }
    file.write_all(bytes)
}

#[cfg(test)]
fn test_storage_full_path() -> &'static Mutex<Option<PathBuf>> {
    static PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    PATH.get_or_init(|| Mutex::new(None))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// True only when `name` matches a declared backup artifact. Because every
/// [`BACKUP_ARTIFACTS`] entry is a plain, separator-free file name, this also
/// rejects any traversal component (`..`), path separator, or absolute path a
/// crafted manifest might carry.
fn is_known_artifact_name(name: &str) -> bool {
    BACKUP_ARTIFACTS.iter().any(|spec| spec.file_name == name)
}

fn digest_entries(entries: &[ManifestEntry]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((entries.len() as u64).to_le_bytes());
    for entry in entries {
        hasher.update((entry.file_name.len() as u64).to_le_bytes());
        hasher.update(entry.file_name.as_bytes());
        hasher.update(entry.byte_len.to_le_bytes());
        hasher.update((entry.sha256.len() as u64).to_le_bytes());
        hasher.update(entry.sha256.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> Result<(), OfflineStorageError> {
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_DIRECTORY_MODE))?;
    Ok(())
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> Result<(), OfflineStorageError> {
    Ok(())
}

#[cfg(unix)]
fn harden_path(path: &Path) -> Result<(), OfflineStorageError> {
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_FILE_MODE))?;
    Ok(())
}

#[cfg(not(unix))]
fn harden_path(_path: &Path) -> Result<(), OfflineStorageError> {
    Ok(())
}

#[cfg(unix)]
fn harden_open_file(file: &File) -> Result<(), OfflineStorageError> {
    file.set_permissions(fs::Permissions::from_mode(OWNER_FILE_MODE))?;
    Ok(())
}

#[cfg(not(unix))]
fn harden_open_file(_file: &File) -> Result<(), OfflineStorageError> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!(
            "dxbot-offline-unit-{}-{name}-{seq}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        base
    }

    /// A crafted manifest whose self-digest is internally valid (recomputed via
    /// `BackupManifest::build`) but whose entry names attempt path traversal
    /// must be rejected. This proves the manifest self-digest is not treated as
    /// an authenticator and that restore cannot be steered outside the runtime
    /// root by a self-consistent hostile manifest.
    #[test]
    fn traversal_manifest_name_is_rejected_even_with_valid_self_digest() {
        let base = temp_dir("traversal");
        let backup_dir = base.join("backup");
        fs::create_dir_all(&backup_dir).unwrap();

        // Materialize a payload the traversal entry points at, so the failure
        // is the name policy — not a missing/oversized file or digest mismatch.
        let payload = b"hostile-bytes".to_vec();
        let traversal_name = "../escape.json";
        let entry = ManifestEntry {
            file_name: traversal_name.to_owned(),
            byte_len: payload.len() as u64,
            sha256: digest_bytes(&payload),
        };
        // `build` recomputes a correct manifest_digest, so `verify_self` passes.
        let manifest = BackupManifest::build(0, vec![entry]);
        manifest
            .verify_self()
            .expect("crafted manifest self-verifies");
        fs::write(
            backup_dir.join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();

        let runtime_root = base.join("runtime");
        let storage = OfflineStorage::open_offline(runtime_root.clone()).expect("offline lock");

        // load_manifest fails closed with UnknownArtifact (before any read/write
        // against runtime_root).
        let load_err = storage.load_manifest(&backup_dir).unwrap_err();
        assert!(
            matches!(load_err, OfflineStorageError::UnknownArtifact(_)),
            "load_manifest must reject traversal name: {load_err}"
        );

        // Both restore modes fail closed and never touch the runtime root.
        for mode in [RestoreMode::DryRun, RestoreMode::Apply] {
            let err = storage.restore(&backup_dir, mode).unwrap_err();
            assert!(
                matches!(err, OfflineStorageError::UnknownArtifact(_)),
                "restore {mode:?} must reject traversal name: {err}"
            );
        }
        // Restore never resolved the traversal name relative to runtime_root,
        // so no escape file was written beside it.
        assert!(!base.join("escape.json").exists());
        let _ = payload;
    }

    /// A separator-bearing but non-`..` name is likewise rejected.
    #[test]
    fn nested_separator_name_is_rejected() {
        assert!(!is_known_artifact_name("subdir/application-state.json"));
        assert!(!is_known_artifact_name("/etc/passwd"));
        assert!(!is_known_artifact_name(".."));
        assert!(is_known_artifact_name("application-state.json"));
        assert!(is_known_artifact_name(".security-application-uow.json"));
    }

    #[test]
    fn storage_full_during_atomic_write_preserves_prior_artifact() {
        let base = temp_dir("storage-full");
        let target = base.join("application-state.json");
        atomic_write(&target, b"prior-durable-state").expect("baseline");
        *test_storage_full_path().lock().expect("fault lock") = Some(target.clone());
        let error = atomic_write(&target, b"replacement-state").expect_err("storage full");
        *test_storage_full_path().lock().expect("fault lock") = None;
        assert!(
            matches!(error, OfflineStorageError::Io(message) if message.contains("storage-full"))
        );
        assert_eq!(
            fs::read(&target).expect("prior state"),
            b"prior-durable-state"
        );
        assert_eq!(
            fs::read_dir(&base)
                .expect("root")
                .filter_map(Result::ok)
                .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
                .count(),
            0,
            "failed temp write is cleaned"
        );
    }
}

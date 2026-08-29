//! Safe output writer (AT-EXPORT-001): atomic, no-replace, no-follow, fsynced
//! artifact/data output.
//!
//! Protocol per write:
//!
//! 1. Write the content to a unique temp file in the same directory.
//! 2. fsync the temp file.
//! 3. fsync the parent directory.
//! 4. Publish to the destination with **no overwrite** semantics (the
//!    destination is never clobbered, even under a check/rename race).
//! 5. If the destination already exists, return [`Error::AlreadyExists`].
//!
//! `no_follow` refuses to write when the destination is a symlink
//! ([`Error::SymlinkDetected`]), failing closed so an export can never be
//! redirected through a symlink to an arbitrary location. Bounded writes
//! truncate to a caller byte ceiling and report the number of bytes actually
//! written.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Internal, atomic rename fault classification so the caller-level
/// [`Error`] stays small and precise.
enum Fault {
    /// The destination already exists; nothing was changed.
    DestExists,
    /// An OS I/O failure.
    Io(String),
}

/// Errors produced by the safe writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The destination already exists; the write was refused (no overwrite).
    AlreadyExists(PathBuf),
    /// The destination is a symlink and `no_follow` was requested; refused.
    SymlinkDetected(PathBuf),
    /// An OS I/O failure while writing to `path`.
    Io { path: PathBuf, message: String },
}

impl Error {
    fn io(path: &Path, e: std::io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            message: e.to_string(),
        }
    }

    /// Projects onto the canonical [`dxbot_core::error::DxbotError`] surface.
    pub fn to_dxbot_error(&self) -> dxbot_core::error::DxbotError {
        use dxbot_core::error::{ErrorCategory, ErrorCode};
        let (code, category, message) = match self {
            Self::AlreadyExists(p) => (
                ErrorCode::Conflict,
                ErrorCategory::Conflict,
                format!("refusing to overwrite existing output: {}", p.display()),
            ),
            Self::SymlinkDetected(p) => (
                ErrorCode::PermissionDenied,
                ErrorCategory::Permission,
                format!("refusing to follow symlink output: {}", p.display()),
            ),
            Self::Io { path, message } => (
                ErrorCode::StorageOrCorruption,
                ErrorCategory::Local,
                format!("cannot write output {}: {message}", path.display()),
            ),
        };
        dxbot_core::error::DxbotError {
            code,
            category,
            message,
            retryable: false,
            operation_ref: None,
            target_refs: Vec::new(),
            field_violations: Vec::new(),
            current_revision: None,
            current_generation: None,
            resume_cursor: None,
            next_actions: Vec::new(),
        }
    }
}

/// The safe output writer.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SafeWriter;

impl SafeWriter {
    pub fn new() -> Self {
        Self
    }

    /// Safely writes `content` to `path` following the module protocol.
    ///
    /// `no_follow` refuses when `path` is a symlink. If `path` already exists
    /// (symlink or not), returns [`Error::AlreadyExists`]; the existing
    /// destination is never overwritten.
    pub fn write_to_path(&self, path: &Path, content: &[u8], no_follow: bool) -> Result<(), Error> {
        if no_follow {
            match path.symlink_metadata() {
                Ok(md) if md.file_type().is_symlink() => {
                    return Err(Error::SymlinkDetected(path.to_path_buf()));
                }
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(Error::io(path, e)),
            }
        }

        let parent = parent_dir(path);
        let tmp = unique_temp_path(path, &parent);

        let faults = (|| {
            // 1. Unique temp file in the same directory (exclusive create).
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .map_err(|e| Fault::Io(e.to_string()))?;
            file.write_all(content)
                .map_err(|e| Fault::Io(e.to_string()))?;
            // 2. fsync the temp file.
            file.sync_all().map_err(|e| Fault::Io(e.to_string()))?;
            drop(file);

            // 3. fsync the parent directory before publishing.
            File::open(&parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| Fault::Io(e.to_string()))?;

            // 4. Publish with no-overwrite semantics. A hard link fails with
            // AlreadyExists when the destination exists, atomically, without
            // ever clobbering it — the no-replace analog of an atomic rename.
            match fs::hard_link(&tmp, path) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                    return Err(Fault::DestExists);
                }
                Err(e) => return Err(Fault::Io(e.to_string())),
            }

            // Persist the new directory entry, then drop the temp name.
            File::open(&parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| Fault::Io(e.to_string()))?;
            fs::remove_file(&tmp).map_err(|e| Fault::Io(e.to_string()))?;
            File::open(&parent)
                .and_then(|f| f.sync_all())
                .map_err(|e| Fault::Io(e.to_string()))?;
            Ok(())
        })();

        match faults {
            Ok(()) => Ok(()),
            Err(Fault::DestExists) => {
                let _ = fs::remove_file(&tmp);
                Err(Error::AlreadyExists(path.to_path_buf()))
            }
            Err(Fault::Io(message)) => {
                let _ = fs::remove_file(&tmp);
                Err(Error::io(path, std::io::Error::other(message)))
            }
        }
    }

    /// Writes at most `max_bytes` of `content`, returning the number of bytes
    /// actually written. Uses the same safe write protocol (no-replace,
    /// no-follow), so a bounded export is still atomic and never overwrites.
    pub fn write_bounded(
        &self,
        path: &Path,
        content: &[u8],
        max_bytes: usize,
    ) -> Result<usize, Error> {
        let n = content.len().min(max_bytes);
        self.write_to_path(path, &content[..n], true)?;
        Ok(n)
    }
}

/// The directory that must hold both the temp file and the destination,
/// defaulting to `.` when `path` has no parent.
fn parent_dir(path: &Path) -> PathBuf {
    match path.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

/// A unique temp path in `parent` so `create_new` never collides with another
/// writer (single published output, concurrent writers safe).
fn unique_temp_path(path: &Path, parent: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    parent.join(format!(".{name}.safewrite.{}.{nanos}", std::process::id()))
}

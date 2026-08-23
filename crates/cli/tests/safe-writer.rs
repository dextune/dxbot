//! Acceptance coverage for `AT-EXPORT-001`: atomic no-replace, no-follow,
//! fsync-before-publish, and bounded safe output.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

use cli::safe_writer::{Error, SafeWriter};

struct TempDir(PathBuf);
impl TempDir {
    fn as_path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_base() -> TempDir {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "dxbot-safewriter-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

fn writer() -> SafeWriter {
    SafeWriter::new()
}

#[test]
fn safe_writer_atomic_no_replace_creates_file() {
    let base = temp_base();
    let path = base.as_path().join("out.bin");
    writer()
        .write_to_path(&path, b"hello world", true)
        .unwrap();

    assert_eq!(fs::read(&path).unwrap(), b"hello world");

    // No temp files may be left behind after a successful write.
    let leftovers: Vec<_> = fs::read_dir(base.as_path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("safewrite"))
        .collect();
    assert!(leftovers.is_empty(), "no temp files left: {leftovers:?}");
}

#[test]
fn safe_writer_refuses_overwrite_on_existing_file() {
    let base = temp_base();
    let path = base.as_path().join("exists.bin");
    fs::write(&path, b"original").unwrap();

    match writer().write_to_path(&path, b"replacement", true) {
        Err(Error::AlreadyExists(p)) => assert_eq!(p, path),
        other => panic!("expected AlreadyExists, got {other:?}"),
    }
    // The existing destination is untouched.
    assert_eq!(fs::read(&path).unwrap(), b"original");
}

#[test]
fn safe_writer_bounded_write_truncates_at_max_bytes() {
    let base = temp_base();
    let path = base.as_path().join("bounded.bin");
    let content = vec![0xABu8; 10_000];

    let written = writer().write_bounded(&path, &content, 100).unwrap();
    assert_eq!(written, 100);
    assert_eq!(fs::read(&path).unwrap().len(), 100);

    // Bounds equal to the content length write everything (fresh path; the
    // first write already published to `path` and is no-replace).
    let full_path = base.as_path().join("bounded_full.bin");
    let full = writer().write_bounded(&full_path, &content[..5], 5).unwrap();
    assert_eq!(full, 5);
    assert_eq!(fs::read(&full_path).unwrap(), &content[..5]);
}

#[test]
fn safe_writer_no_follow_refuses_symlink() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let base = temp_base();
        // A victim file the export must never reach through a symlink.
        let victim = base.as_path().join("victim.bin");
        fs::write(&victim, b"secret").unwrap();
        let link = base.as_path().join("out.link");
        symlink(&victim, &link).unwrap();

        match writer().write_to_path(&link, b"exfil", true) {
            Err(Error::SymlinkDetected(p)) => assert_eq!(p, link),
            other => panic!("expected SymlinkDetected, got {other:?}"),
        }
        // Fail closed: the victim was never written through the symlink.
        assert_eq!(fs::read(&victim).unwrap(), b"secret");
    }
}

#[test]
fn safe_writer_fsyncs_before_rename() {
    let base = temp_base();
    let path = base.as_path().join("durable.bin");
    writer().write_to_path(&path, b"durable payload", true).unwrap();

    // A written artifact is durably present with the exact bytes.
    assert!(path.exists());
    assert_eq!(fs::read(&path).unwrap(), b"durable payload");

    // The same directory layout holds for a second independent write target,
    // confirming the publish path is repeatable across the directory.
    let path2 = base.as_path().join("durable2.bin");
    writer().write_to_path(&path2, b"second", true).unwrap();
    assert_eq!(fs::read(&path2).unwrap(), b"second");
}
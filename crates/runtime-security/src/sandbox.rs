//! Docker-independent local subprocess sandbox prototype.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use runtime_audit::{
    DiagnosticComponent, DiagnosticFamily, DiagnosticReason, DiagnosticSeverity, DiagnosticSink,
    SafeAttribute, SafeAttributeKey,
};
use sha2::{Digest, Sha256};

const MAX_EXECUTION_ID_BYTES: usize = 96;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxError {
    InvalidExecutionId,
    Unowned { expected: String, provided: String },
    GenerationFenced { expected: i64, provided: i64 },
    ArtifactCorrupt,
    ArtifactDigestMismatch,
    InvalidMountPolicy,
    WorkspaceExists,
    Io(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPolicy {
    Deny,
    LoopbackOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountClass {
    RuntimeArtifact,
    ExecutionWorkspace,
    UserProject,
    CanonicalStore,
    CredentialDirectory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountSpec {
    pub class: MountClass,
    pub mode: MountMode,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeArtifact {
    path: PathBuf,
    sha256: String,
    size: u64,
}

impl RuntimeArtifact {
    pub fn publish(root: &Path, bytes: &[u8]) -> Result<Self, SandboxError> {
        fs::create_dir_all(root).map_err(io_error)?;
        let sha256 = sha256_bytes(bytes);
        let path = root.join(&sha256);

        if path.exists() {
            let artifact = Self {
                path,
                sha256,
                size: u64::try_from(bytes.len()).map_err(|_| SandboxError::ArtifactCorrupt)?,
            };
            artifact.verify()?;
            return Ok(artifact);
        }

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(io_error)?;
        file.write_all(bytes).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
        drop(file);
        make_read_only_executable(&path)?;

        let artifact = Self {
            path,
            sha256,
            size: u64::try_from(bytes.len()).map_err(|_| SandboxError::ArtifactCorrupt)?,
        };
        artifact.verify()?;
        Ok(artifact)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn verify(&self) -> Result<(), SandboxError> {
        let metadata = fs::metadata(&self.path).map_err(io_error)?;
        if metadata.len() != self.size {
            return Err(SandboxError::ArtifactCorrupt);
        }
        let observed = sha256_file(&self.path)?;
        if observed != self.sha256 {
            return Err(SandboxError::ArtifactDigestMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    pub owner_instance_id: String,
    pub execution_id: String,
    pub host_generation: i64,
    pub runtime_artifact: RuntimeArtifact,
    pub network_policy: NetworkPolicy,
    pub mounts: Vec<MountSpec>,
    pub execution_deadline: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxState {
    Active,
    Stopped,
    Removed,
}

#[derive(Debug)]
pub struct SandboxHandle {
    sandbox_id: String,
    owner_instance_id: String,
    host_generation: i64,
    artifact_sha256: String,
    workspace: PathBuf,
    state: SandboxState,
    child: Option<Child>,
}

impl SandboxHandle {
    pub fn sandbox_id(&self) -> &str {
        &self.sandbox_id
    }

    pub fn state(&self) -> SandboxState {
        self.state
    }

    pub fn artifact_sha256(&self) -> &str {
        &self.artifact_sha256
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxTerminal {
    Completed,
    Cancelled,
    DeadlineExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxRunResult {
    pub terminal: SandboxTerminal,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct LocalSubprocessSandbox {
    owner_instance_id: String,
    host_generation: i64,
    workspace_root: PathBuf,
    diagnostics: Option<DiagnosticSink>,
}

impl LocalSubprocessSandbox {
    pub fn new(owner_instance_id: &str, host_generation: i64, workspace_root: &Path) -> Self {
        Self {
            owner_instance_id: owner_instance_id.to_owned(),
            host_generation,
            workspace_root: workspace_root.to_path_buf(),
            diagnostics: None,
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticSink) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    pub fn start(
        &self,
        spec: &SandboxSpec,
        args: &[String],
    ) -> Result<SandboxHandle, SandboxError> {
        self.validate_spec(spec)?;
        if let Err(error) = spec.runtime_artifact.verify() {
            self.emit(
                DiagnosticFamily::SandboxArtifact,
                DiagnosticReason::SandboxDigestMismatch,
                DiagnosticSeverity::Error,
                &spec.execution_id,
            );
            return Err(error);
        }

        fs::create_dir_all(&self.workspace_root).map_err(io_error)?;
        let workspace = self.workspace_root.join(&spec.execution_id);
        match fs::create_dir(&workspace) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(SandboxError::WorkspaceExists);
            }
            Err(error) => return Err(io_error(error)),
        }

        let spawn = Command::new(spec.runtime_artifact.path())
            .args(args)
            .current_dir(&workspace)
            .env_clear()
            .env(
                "DXBOT_SANDBOX_NETWORK",
                match spec.network_policy {
                    NetworkPolicy::Deny => "deny",
                    NetworkPolicy::LoopbackOnly => "loopback-only",
                },
            )
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        let child = match spawn {
            Ok(child) => child,
            Err(error) => {
                let _ = fs::remove_dir_all(&workspace);
                return Err(io_error(error));
            }
        };

        Ok(SandboxHandle {
            sandbox_id: format!("{}-{}", spec.execution_id, child.id()),
            owner_instance_id: spec.owner_instance_id.clone(),
            host_generation: spec.host_generation,
            artifact_sha256: spec.runtime_artifact.sha256().to_owned(),
            workspace,
            state: SandboxState::Active,
            child: Some(child),
        })
    }

    pub fn run_with_control(
        &self,
        spec: &SandboxSpec,
        args: &[String],
        cancelled: &AtomicBool,
    ) -> Result<SandboxRunResult, SandboxError> {
        let mut handle = self.start(spec, args)?;
        let deadline = Instant::now() + spec.execution_deadline;

        loop {
            if cancelled.load(Ordering::Acquire) {
                self.terminate(&mut handle)?;
                return Ok(SandboxRunResult {
                    terminal: SandboxTerminal::Cancelled,
                    exit_code: None,
                });
            }
            if Instant::now() >= deadline {
                self.terminate(&mut handle)?;
                return Ok(SandboxRunResult {
                    terminal: SandboxTerminal::DeadlineExceeded,
                    exit_code: None,
                });
            }

            let status = match handle.child.as_mut() {
                Some(child) => child.try_wait().map_err(io_error)?,
                None => None,
            };
            if let Some(status) = status {
                handle.child = None;
                handle.state = SandboxState::Stopped;
                self.cleanup_workspace(&mut handle)?;
                return Ok(SandboxRunResult {
                    terminal: SandboxTerminal::Completed,
                    exit_code: status.code(),
                });
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    pub fn stop(
        &self,
        handle: &mut SandboxHandle,
        requester_owner: &str,
        requester_generation: i64,
    ) -> Result<(), SandboxError> {
        self.validate_handle_authority(handle, requester_owner, requester_generation)?;
        self.terminate(handle)
    }

    pub fn cleanup(
        &self,
        handle: &mut SandboxHandle,
        requester_owner: &str,
        requester_generation: i64,
    ) -> Result<(), SandboxError> {
        self.validate_handle_authority(handle, requester_owner, requester_generation)?;
        self.terminate(handle)
    }

    fn validate_spec(&self, spec: &SandboxSpec) -> Result<(), SandboxError> {
        if !valid_execution_id(&spec.execution_id) {
            return Err(SandboxError::InvalidExecutionId);
        }
        if spec.owner_instance_id != self.owner_instance_id {
            self.emit(
                DiagnosticFamily::SandboxOwnership,
                DiagnosticReason::SandboxUnowned,
                DiagnosticSeverity::Error,
                &spec.execution_id,
            );
            return Err(SandboxError::Unowned {
                expected: self.owner_instance_id.clone(),
                provided: spec.owner_instance_id.clone(),
            });
        }
        if spec.host_generation != self.host_generation {
            self.emit(
                DiagnosticFamily::SandboxOwnership,
                DiagnosticReason::SandboxStaleGeneration,
                DiagnosticSeverity::Error,
                &spec.execution_id,
            );
            return Err(SandboxError::GenerationFenced {
                expected: self.host_generation,
                provided: spec.host_generation,
            });
        }
        if spec.execution_deadline.is_zero() {
            return Err(SandboxError::InvalidMountPolicy);
        }
        validate_mounts(&spec.mounts)
    }

    fn validate_handle_authority(
        &self,
        handle: &SandboxHandle,
        requester_owner: &str,
        requester_generation: i64,
    ) -> Result<(), SandboxError> {
        if requester_owner != self.owner_instance_id || requester_owner != handle.owner_instance_id {
            return Err(SandboxError::Unowned {
                expected: handle.owner_instance_id.clone(),
                provided: requester_owner.to_owned(),
            });
        }
        if requester_generation != self.host_generation
            || requester_generation != handle.host_generation
        {
            return Err(SandboxError::GenerationFenced {
                expected: handle.host_generation,
                provided: requester_generation,
            });
        }
        Ok(())
    }

    fn terminate(&self, handle: &mut SandboxHandle) -> Result<(), SandboxError> {
        if handle.state == SandboxState::Removed {
            return Ok(());
        }
        if let Some(mut child) = handle.child.take() {
            if child.try_wait().map_err(io_error)?.is_none() {
                child.kill().map_err(io_error)?;
            }
            child.wait().map_err(io_error)?;
        }
        handle.state = SandboxState::Stopped;
        self.cleanup_workspace(handle)
    }

    fn cleanup_workspace(&self, handle: &mut SandboxHandle) -> Result<(), SandboxError> {
        if handle.workspace.exists() {
            fs::remove_dir_all(&handle.workspace).map_err(|error| {
                self.emit(
                    DiagnosticFamily::SandboxReadiness,
                    DiagnosticReason::SandboxCleanupFailed,
                    DiagnosticSeverity::Error,
                    &handle.sandbox_id,
                );
                io_error(error)
            })?;
        }
        handle.state = SandboxState::Removed;
        Ok(())
    }

    fn emit(
        &self,
        family: DiagnosticFamily,
        reason: DiagnosticReason,
        severity: DiagnosticSeverity,
        execution_id: &str,
    ) {
        let Some(sink) = &self.diagnostics else {
            return;
        };
        let attributes = [SafeAttribute::new(
            SafeAttributeKey::ResourceKind,
            "local-subprocess",
        )];
        let _ = sink.emit(
            DiagnosticComponent::Sandbox,
            execution_id,
            family,
            reason,
            severity,
            &attributes,
        );
    }
}

fn validate_mounts(mounts: &[MountSpec]) -> Result<(), SandboxError> {
    for mount in mounts {
        match (mount.class, mount.mode) {
            (MountClass::RuntimeArtifact, MountMode::ReadOnly)
            | (MountClass::ExecutionWorkspace, MountMode::ReadWrite)
            | (MountClass::UserProject, MountMode::ReadOnly) => {}
            (MountClass::CanonicalStore, _)
            | (MountClass::CredentialDirectory, _)
            | (MountClass::RuntimeArtifact, MountMode::ReadWrite)
            | (MountClass::ExecutionWorkspace, MountMode::ReadOnly)
            | (MountClass::UserProject, MountMode::ReadWrite) => {
                return Err(SandboxError::InvalidMountPolicy);
            }
        }
    }
    Ok(())
}

fn valid_execution_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_EXECUTION_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn sha256_file(path: &Path) -> Result<String, SandboxError> {
    let mut file = fs::File::open(path).map_err(io_error)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer).map_err(io_error)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(unix)]
fn make_read_only_executable(path: &Path) -> Result<(), SandboxError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o500)).map_err(io_error)
}

#[cfg(not(unix))]
fn make_read_only_executable(path: &Path) -> Result<(), SandboxError> {
    let mut permissions = fs::metadata(path).map_err(io_error)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).map_err(io_error)
}

fn io_error(error: std::io::Error) -> SandboxError {
    SandboxError::Io(error.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use std::sync::atomic::AtomicBool;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("dxbot-{label}-{nanos}"))
    }

    fn script(root: &Path, body: &str) -> Result<RuntimeArtifact, SandboxError> {
        RuntimeArtifact::publish(root, format!("#!/bin/sh\n{body}\n").as_bytes())
    }

    fn spec(artifact: RuntimeArtifact, execution_id: &str) -> SandboxSpec {
        SandboxSpec {
            owner_instance_id: "instance-1".to_owned(),
            execution_id: execution_id.to_owned(),
            host_generation: 7,
            runtime_artifact: artifact,
            network_policy: NetworkPolicy::Deny,
            mounts: Vec::new(),
            execution_deadline: Duration::from_secs(1),
        }
    }

    #[test]
    fn artifact_is_content_addressed_and_digest_checked() {
        let root = temp_root("artifact");
        let artifact = script(&root, "exit 0");
        assert!(artifact.is_ok());
        let Ok(artifact) = artifact else {
            return;
        };
        assert_eq!(
            artifact.path().file_name().and_then(|value| value.to_str()),
            Some(artifact.sha256())
        );
        assert!(artifact.verify().is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_or_unowned_authority_cannot_stop_sandbox() {
        let artifact_root = temp_root("authority-artifact");
        let workspace_root = temp_root("authority-workspace");
        let artifact = script(&artifact_root, "sleep 1");
        assert!(artifact.is_ok());
        let Ok(artifact) = artifact else {
            return;
        };
        let sandbox = LocalSubprocessSandbox::new("instance-1", 7, &workspace_root);
        let handle = sandbox.start(&spec(artifact, "exec-authority"), &[]);
        assert!(handle.is_ok());
        let Ok(mut handle) = handle else {
            return;
        };

        assert!(matches!(
            sandbox.stop(&mut handle, "instance-2", 7),
            Err(SandboxError::Unowned { .. })
        ));
        assert_eq!(handle.state(), SandboxState::Active);
        assert!(matches!(
            sandbox.stop(&mut handle, "instance-1", 6),
            Err(SandboxError::GenerationFenced { .. })
        ));
        assert_eq!(handle.state(), SandboxState::Active);
        assert!(sandbox.stop(&mut handle, "instance-1", 7).is_ok());
        assert_eq!(handle.state(), SandboxState::Removed);
        let _ = fs::remove_dir_all(artifact_root);
        let _ = fs::remove_dir_all(workspace_root);
    }

    #[test]
    fn cancellation_and_deadline_cleanup_workspace() {
        let artifact_root = temp_root("control-artifact");
        let workspace_root = temp_root("control-workspace");
        let artifact = script(&artifact_root, "sleep 5");
        assert!(artifact.is_ok());
        let Ok(artifact) = artifact else {
            return;
        };
        let sandbox = LocalSubprocessSandbox::new("instance-1", 7, &workspace_root);

        let cancelled = AtomicBool::new(true);
        let result = sandbox.run_with_control(
            &spec(artifact.clone(), "exec-cancel"),
            &[],
            &cancelled,
        );
        assert!(result.is_ok());
        assert_eq!(
            result.map(|value| value.terminal),
            Ok(SandboxTerminal::Cancelled)
        );
        assert!(!workspace_root.join("exec-cancel").exists());

        let mut deadline_spec = spec(artifact, "exec-deadline");
        deadline_spec.execution_deadline = Duration::from_millis(20);
        let not_cancelled = AtomicBool::new(false);
        let result = sandbox.run_with_control(&deadline_spec, &[], &not_cancelled);
        assert!(result.is_ok());
        assert_eq!(
            result.map(|value| value.terminal),
            Ok(SandboxTerminal::DeadlineExceeded)
        );
        assert!(!workspace_root.join("exec-deadline").exists());

        let _ = fs::remove_dir_all(artifact_root);
        let _ = fs::remove_dir_all(workspace_root);
    }

    #[test]
    fn canonical_store_and_credential_directory_mounts_are_rejected() {
        let artifact_root = temp_root("mount-artifact");
        let workspace_root = temp_root("mount-workspace");
        let artifact = script(&artifact_root, "exit 0");
        assert!(artifact.is_ok());
        let Ok(artifact) = artifact else {
            return;
        };
        let sandbox = LocalSubprocessSandbox::new("instance-1", 7, &workspace_root);
        let mut invalid = spec(artifact, "exec-mount");
        invalid.mounts.push(MountSpec {
            class: MountClass::CanonicalStore,
            mode: MountMode::ReadOnly,
            path: PathBuf::from("/canonical"),
        });
        assert_eq!(
            sandbox.start(&invalid, &[]).map(|_| ()),
            Err(SandboxError::InvalidMountPolicy)
        );
        let _ = fs::remove_dir_all(artifact_root);
        let _ = fs::remove_dir_all(workspace_root);
    }
}

//! CLI discovery: local help/version, exact Instance selection, and provider
//! descriptor diagnostics.
//!
//! Discovery is a read/selection boundary, not a Runtime Host. It never
//! fabricates a Runtime endpoint, authenticated peer, provider readiness, or
//! first-Instance state. First-run bootstrap remains owned by the Runtime Host
//! coordinator defined by DXB-RUN-035.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, PermissionsExt};

use application_contract::{LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{InstanceId, VersionInfo};
use serde::{Deserialize, Serialize};

pub const DISCOVERY_STATE_FILE: &str = "discovery.json";
pub const REQUIRED_PROVIDER_CAPABILITIES: &[&str] = &["llm-chat", "embeddings", "auth"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryEndpoint {
    pub instance_id: InstanceId,
    pub profile: Option<String>,
    pub endpoint: String,
    pub host_generation: i64,
    pub provider_id: String,
    pub provider_ready: bool,
}

impl DiscoveryEndpoint {
    #[cfg(unix)]
    pub fn unix_socket_path(&self) -> Result<PathBuf, DiscoveryError> {
        let path = self.endpoint.strip_prefix("unix://").ok_or_else(|| {
            DiscoveryError::InvalidEndpoint {
                message: format!("unsupported local endpoint scheme: {}", self.endpoint),
            }
        })?;
        if path.is_empty() {
            return Err(DiscoveryError::InvalidEndpoint {
                message: "Unix endpoint path is empty".to_owned(),
            });
        }
        Ok(PathBuf::from(path))
    }
}

/// Selected descriptor plus the UID that owns the durable discovery record.
/// The transport must verify that the live endpoint has the same owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedEndpoint {
    pub descriptor: DiscoveryEndpoint,
    pub state_owner_uid: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDiagnostic {
    pub provider_id: String,
    pub status: String,
    pub required_capabilities: Vec<String>,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryState {
    pub instance_endpoints: HashMap<InstanceId, DiscoveryEndpoint>,
}

impl DiscoveryState {
    pub fn load_state(base_path: &Path) -> Result<Self, DiscoveryError> {
        let path = base_path.join(DISCOVERY_STATE_FILE);
        let data = match fs::read(&path) {
            Ok(data) => data,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Self {
                    instance_endpoints: HashMap::new(),
                });
            }
            Err(error) => return Err(DiscoveryError::state_io(&path, error)),
        };
        validate_state_file(&path)?;
        serde_json::from_slice(&data).map_err(|error| DiscoveryError::StateIo {
            path,
            message: error.to_string(),
        })
    }

    /// Persists only already-verified descriptors supplied by the Runtime Host
    /// integration boundary. This method does not discover or verify peers.
    pub fn save_state(&self, base_path: &Path) -> Result<(), DiscoveryError> {
        fs::create_dir_all(base_path)
            .map_err(|error| DiscoveryError::state_io(base_path, error))?;
        harden_directory(base_path)?;
        let path = base_path.join(DISCOVERY_STATE_FILE);
        let tmp = base_path.join(format!("{DISCOVERY_STATE_FILE}.tmp"));
        let bytes = serde_json::to_vec(self).map_err(|error| DiscoveryError::StateIo {
            path: path.clone(),
            message: error.to_string(),
        })?;
        {
            let mut file = File::create(&tmp)
                .map_err(|error| DiscoveryError::state_io(&tmp, error))?;
            harden_file(&tmp)?;
            file.write_all(&bytes)
                .map_err(|error| DiscoveryError::state_io(&tmp, error))?;
            file.sync_all()
                .map_err(|error| DiscoveryError::state_io(&tmp, error))?;
        }
        fs::rename(&tmp, &path).map_err(|error| DiscoveryError::state_io(&tmp, error))?;
        sync_directory(base_path)?;
        validate_state_file(&path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    NoInstance { message: String },
    Ambiguous { candidates: Vec<InstanceId> },
    NotFound { instance_id: InstanceId },
    StateIo { path: PathBuf, message: String },
    BootstrapConflict { existing: Vec<InstanceId> },
    InvalidEndpoint { message: String },
}

impl DiscoveryError {
    fn state_io(path: &Path, error: std::io::Error) -> Self {
        Self::StateIo {
            path: path.to_path_buf(),
            message: error.to_string(),
        }
    }

    pub fn to_dxbot_error(&self) -> DxbotError {
        match self {
            Self::NoInstance { message } => dxbot_error(
                ErrorCode::RuntimeUnavailable,
                ErrorCategory::Availability,
                message.clone(),
            ),
            Self::Ambiguous { candidates } => {
                let names: Vec<String> = candidates.iter().map(|id| id.0.clone()).collect();
                dxbot_error(
                    ErrorCode::AmbiguousTarget,
                    ErrorCategory::Conflict,
                    format!("multiple Instances matched; specify one with --instance: {names:?}"),
                )
            }
            Self::NotFound { instance_id } => dxbot_error(
                ErrorCode::NotFound,
                ErrorCategory::Input,
                format!("Instance not found: {}", instance_id.0),
            ),
            Self::StateIo { path, message } => dxbot_error(
                ErrorCode::StorageOrCorruption,
                ErrorCategory::Local,
                format!("cannot access discovery state {}: {message}", path.display()),
            ),
            Self::BootstrapConflict { existing } => {
                let names: Vec<String> = existing.iter().map(|id| id.0.clone()).collect();
                dxbot_error(
                    ErrorCode::Conflict,
                    ErrorCategory::Conflict,
                    format!("first-run bootstrap refused: Instances already exist ({names:?})"),
                )
            }
            Self::InvalidEndpoint { message } => dxbot_error(
                ErrorCode::Incompatible,
                ErrorCategory::Conflict,
                message.clone(),
            ),
        }
    }
}

fn dxbot_error(code: ErrorCode, category: ErrorCategory, message: String) -> DxbotError {
    DxbotError {
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

fn default_base_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_STATE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("dxbot").join("cli");
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join(".local")
                .join("state")
                .join("dxbot")
                .join("cli");
        }
    }
    PathBuf::from("dxbot-state").join("cli")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    base_path: PathBuf,
}

impl Discovery {
    pub fn new() -> Self {
        Self::at(default_base_path())
    }

    pub fn at(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    pub fn show_help() -> String {
        "\
dxb — DXBOT command line interface

Usage:
  dxb <command> [options]
  dxb --help
  dxb --version [--format human|json|jsonl]
  dxb [--profile <name>] [--instance <id>] <group> <command> [options]

Global options:
  --profile <name>  Profile binding hint for Instance selection (not Authority).
  --instance <id>   Explicit Instance selection hint (not Authority).
  --format <fmt>    Output format: human | json | jsonl.
  --color <mode>    Color mode: auto | always | never.
  --wait <policy>   Wait policy for operations.
  --timeout <dur>   Operation timeout.
  -y, --yes         Non-interactive confirmation.
  -h, --help        Show this help and exit 0.
  --version         Show client version and protocol compatibility.

Command groups:
  runtime       Start, status, stop, doctor.
  bot           Create, list, show, activate, deactivate, archive, restore.
  conversation  Show, send, history.
  thread        Create, list, show, send, history, branch.
  task          Submit, list, show, watch, cancel, suspend, resume, redirect, result.
  project       Create, list, show, archive, restore, member.
  channel       Create, list, show, member, send, history.
  memory        Get, search, history, propose, promote.
  approval      List, show, approve, deny.
  provider      List, show.
  operation     Show, reconcile.
  process       Show, watch.
  side-effect   Reconcile.
  version       Show client version and protocol compatibility.

Instance selection:
  explicit --instance, then profile binding, then exactly one verified local
  endpoint. No silent fallback. No Instance is runtime-unavailable (exit 10);
  more than one candidate is ambiguous-target (exit 5).

Runtime bootstrap:
  First-Instance creation belongs to the Runtime Host coordinator. Discovery
  never fabricates an endpoint or provider-ready descriptor. Until the
  authenticated control path is connected, runtime operations fail closed.
"
        .to_string()
    }

    pub fn show_version() -> VersionInfo {
        VersionInfo {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            supported_protocol_versions: vec![LOCAL_CONTROL_PROTOCOL_VERSION.to_owned()],
            supported_schema_versions: vec![LOCAL_CONTROL_SCHEMA_VERSION.to_owned()],
            remote_compatibility: None,
        }
    }

    pub fn select_instance(
        &self,
        profile: Option<&str>,
        instance: Option<&str>,
    ) -> Result<InstanceId, DiscoveryError> {
        self.select_endpoint(profile, instance)
            .map(|selected| selected.descriptor.instance_id)
    }

    pub fn select_endpoint(
        &self,
        profile: Option<&str>,
        instance: Option<&str>,
    ) -> Result<SelectedEndpoint, DiscoveryError> {
        let state = DiscoveryState::load_state(&self.base_path)?;
        let state_file = self.base_path.join(DISCOVERY_STATE_FILE);
        let owner_uid = discovery_owner_uid(&state_file)?;
        let mut candidates: Vec<DiscoveryEndpoint> = if let Some(name) = instance {
            state
                .instance_endpoints
                .values()
                .filter(|endpoint| endpoint.instance_id.0 == name)
                .cloned()
                .collect()
        } else if let Some(profile) = profile {
            state
                .instance_endpoints
                .values()
                .filter(|endpoint| endpoint.profile.as_deref() == Some(profile))
                .cloned()
                .collect()
        } else {
            state.instance_endpoints.values().cloned().collect()
        };
        candidates.sort_by(|a, b| a.instance_id.0.cmp(&b.instance_id.0));
        candidates.dedup_by(|a, b| a.instance_id == b.instance_id);

        if candidates.len() == 1 {
            let descriptor = candidates.remove(0);
            validate_descriptor(&descriptor)?;
            return Ok(SelectedEndpoint {
                descriptor,
                state_owner_uid: owner_uid,
            });
        }
        if candidates.is_empty() {
            let wanted = match (profile, instance) {
                (_, Some(name)) => format!("instance '{name}'"),
                (Some(profile), None) => format!("profile '{profile}'"),
                (None, None) => "any verified local Instance".to_string(),
            };
            return Err(DiscoveryError::NoInstance {
                message: format!("no Runtime Instance found matching {wanted}"),
            });
        }
        Err(DiscoveryError::Ambiguous {
            candidates: candidates
                .into_iter()
                .map(|endpoint| endpoint.instance_id)
                .collect(),
        })
    }

    /// Refuses to synthesize first-run state in the CLI layer. The Runtime Host
    /// must perform atomic bootstrap and endpoint authentication, then publish
    /// the verified descriptor through the integration boundary.
    pub fn first_run_bootstrap(&self) -> Result<InstanceId, DiscoveryError> {
        let state = DiscoveryState::load_state(&self.base_path)?;
        if !state.instance_endpoints.is_empty() {
            return Err(DiscoveryError::BootstrapConflict {
                existing: state.instance_endpoints.keys().cloned().collect(),
            });
        }
        Err(DiscoveryError::NoInstance {
            message: "first-run bootstrap requires the Runtime Host coordinator; discovery did not publish an unverified endpoint"
                .to_string(),
        })
    }

    pub fn doctor_provider(
        &self,
        instance_id: &InstanceId,
    ) -> Result<ProviderDiagnostic, DiscoveryError> {
        let state = DiscoveryState::load_state(&self.base_path)?;
        let descriptor = state
            .instance_endpoints
            .get(instance_id)
            .ok_or_else(|| DiscoveryError::NotFound {
                instance_id: instance_id.clone(),
            })?;
        let status = if descriptor.provider_ready {
            "ready"
        } else {
            "unavailable"
        };
        Ok(ProviderDiagnostic {
            provider_id: descriptor.provider_id.clone(),
            status: status.to_string(),
            required_capabilities: REQUIRED_PROVIDER_CAPABILITIES
                .iter()
                .map(|capability| (*capability).to_string())
                .collect(),
            available: descriptor.provider_ready,
        })
    }
}

impl Default for Discovery {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_descriptor(descriptor: &DiscoveryEndpoint) -> Result<(), DiscoveryError> {
    if descriptor.instance_id.0.trim().is_empty() {
        return Err(DiscoveryError::InvalidEndpoint {
            message: "discovery descriptor has an empty InstanceId".to_owned(),
        });
    }
    if descriptor.host_generation <= 0 {
        return Err(DiscoveryError::InvalidEndpoint {
            message: format!(
                "discovery descriptor has invalid HostGeneration {}",
                descriptor.host_generation
            ),
        });
    }
    #[cfg(unix)]
    descriptor.unix_socket_path()?;
    Ok(())
}

fn validate_state_file(path: &Path) -> Result<(), DiscoveryError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| DiscoveryError::state_io(path, error))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(DiscoveryError::StateIo {
            path: path.to_path_buf(),
            message: "discovery state must be a direct regular file".to_owned(),
        });
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(DiscoveryError::StateIo {
            path: path.to_path_buf(),
            message: "discovery state permissions must be owner-only".to_owned(),
        });
    }
    Ok(())
}

#[cfg(unix)]
fn discovery_owner_uid(path: &Path) -> Result<u32, DiscoveryError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| DiscoveryError::state_io(path, error))?;
    Ok(metadata.uid())
}

#[cfg(not(unix))]
fn discovery_owner_uid(_path: &Path) -> Result<u32, DiscoveryError> {
    Err(DiscoveryError::InvalidEndpoint {
        message: "P0 local control discovery requires a Unix platform".to_owned(),
    })
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> Result<(), DiscoveryError> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| DiscoveryError::state_io(path, error))
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> Result<(), DiscoveryError> {
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<(), DiscoveryError> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| DiscoveryError::state_io(path, error))
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<(), DiscoveryError> {
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), DiscoveryError> {
    let directory = File::open(path).map_err(|error| DiscoveryError::state_io(path, error))?;
    directory
        .sync_all()
        .map_err(|error| DiscoveryError::state_io(path, error))
}

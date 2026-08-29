//! CLI discovery: local help/version, exact Instance selection, and provider
//! descriptor diagnostics.
//!
//! Discovery is read-only from the CLI perspective. Runtime bootstrap owns the
//! durable descriptor schema and publication; the CLI only selects and
//! validates a published endpoint and never fabricates Runtime state.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::path::{Path, PathBuf};

use application_contract::{LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION};
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{InstanceId, VersionInfo};

use runtime_bootstrap::bootstrap::Error as BootstrapError;
use runtime_bootstrap::discovery_state_owner_uid;
pub use runtime_bootstrap::{DISCOVERY_STATE_FILE, DiscoveryEndpoint, DiscoveryState};

pub const REQUIRED_PROVIDER_CAPABILITIES: &[&str] = &["llm-chat", "embeddings", "auth"];

/// Selected descriptor plus the UID that owns the durable discovery record.
/// The transport must verify that the live endpoint has the same owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedEndpoint {
    pub descriptor: DiscoveryEndpoint,
    pub state_owner_uid: u32,
}

impl SelectedEndpoint {
    #[cfg(unix)]
    pub fn unix_socket_path(&self) -> Result<PathBuf, DiscoveryError> {
        let path = self
            .descriptor
            .endpoint
            .strip_prefix("unix://")
            .ok_or_else(|| DiscoveryError::InvalidEndpoint {
                message: format!(
                    "unsupported local endpoint scheme: {}",
                    self.descriptor.endpoint
                ),
            })?;
        if path.is_empty() {
            return Err(DiscoveryError::InvalidEndpoint {
                message: "Unix endpoint path is empty".to_owned(),
            });
        }
        Ok(PathBuf::from(path))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDiagnostic {
    pub provider_id: String,
    pub generation: i64,
    pub status: String,
    pub required_capabilities: Vec<String>,
    pub available: bool,
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
                format!(
                    "cannot access discovery state {}: {message}",
                    path.display()
                ),
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
  never fabricates an endpoint or provider-ready descriptor.
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
        let state = DiscoveryState::load_state(&self.base_path)
            .map_err(|error| map_state_error(&self.base_path, error))?;
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
            let state_owner_uid = discovery_state_owner_uid(&self.base_path)
                .map_err(|error| map_state_error(&self.base_path, error))?
                .ok_or_else(|| DiscoveryError::StateIo {
                    path: self.base_path.join(DISCOVERY_STATE_FILE),
                    message: "selected descriptor has no durable discovery owner".to_owned(),
                })?;
            return Ok(SelectedEndpoint {
                descriptor,
                state_owner_uid,
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
        let state = DiscoveryState::load_state(&self.base_path)
            .map_err(|error| map_state_error(&self.base_path, error))?;
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
        let state = DiscoveryState::load_state(&self.base_path)
            .map_err(|error| map_state_error(&self.base_path, error))?;
        let descriptor =
            state
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
            generation: descriptor.provider_generation,
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
    if descriptor.provider_generation < 0
        || (descriptor.provider_ready && descriptor.provider_generation == 0)
    {
        return Err(DiscoveryError::InvalidEndpoint {
            message: format!(
                "discovery descriptor has invalid ProviderGeneration {}",
                descriptor.provider_generation
            ),
        });
    }
    #[cfg(unix)]
    if !descriptor.endpoint.starts_with("unix://") {
        return Err(DiscoveryError::InvalidEndpoint {
            message: format!("unsupported local endpoint scheme: {}", descriptor.endpoint),
        });
    }
    Ok(())
}

fn map_state_error(base_path: &Path, error: BootstrapError) -> DiscoveryError {
    DiscoveryError::StateIo {
        path: base_path.join(DISCOVERY_STATE_FILE),
        message: format!("{error:?}"),
    }
}

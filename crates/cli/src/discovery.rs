//! CLI discovery: local help/version, first-run bootstrap, exact Instance
//! selection, and provider doctor.
//!
//! Local Instance selection follows the [`DXB-RUN-035`] precedence order:
//!
//! ```text
//! explicit --instance
//! → explicit/selected profile binding
//! → exactly one verified local endpoint
//! ```
//!
//! Selection is exact: it never silently falls back, never auto-starts a
//! Runtime, and never bootstraps except through [`Discovery::first_run_bootstrap`].
//! A missing candidate is `runtime-unavailable` (exit 10); multiple candidates
//! are `ambiguous-target` (exit 5).
//!
//! [`DXB-RUN-035`]: https://dxbot.local/docs/plan/35-configuration-deployment

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{InstanceId, VersionInfo};
use serde::{Deserialize, Serialize};

/// Name of the discovery state file inside the CLI state directory.
pub const DISCOVERY_STATE_FILE: &str = "discovery.json";

/// Capabilities a ready production provider must expose. Reported by
/// [`Discovery::doctor_provider`] as the required capability surface.
pub const REQUIRED_PROVIDER_CAPABILITIES: &[&str] = &["llm-chat", "embeddings", "auth"];

/// A verified local endpoint for a Runtime Instance.
///
/// Persisted in [`DiscoveryState`]. `profile` is a discovery hint only and is
/// not an Authority; instance identity is owned by the InstanceId.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryEndpoint {
    /// The Instance this endpoint serves.
    pub instance_id: InstanceId,
    /// Optional profile binding hint (discovery hint, not Authority).
    pub profile: Option<String>,
    /// Local endpoint address (scheme + path) used to reach the Runtime.
    pub endpoint: String,
    /// HostGeneration fencing this endpoint's process incarnation.
    pub host_generation: i64,
    /// Representative provider id reported by [`Discovery::doctor_provider`].
    pub provider_id: String,
    /// Whether the configured production provider is ready on this Instance.
    pub provider_ready: bool,
}

/// Result of [`Discovery::doctor_provider`] for a single Instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderDiagnostic {
    /// The provider id being diagnosed.
    pub provider_id: String,
    /// `"ready"` when the provider is available, `"unavailable"` otherwise.
    pub status: String,
    /// Capabilities the provider is required to expose.
    pub required_capabilities: Vec<String>,
    /// Whether the provider is currently available on this Instance.
    pub available: bool,
}

/// Discovery state persisted under the CLI state directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryState {
    /// Known Instance endpoints keyed by InstanceId.
    pub instance_endpoints: HashMap<InstanceId, DiscoveryEndpoint>,
}

impl DiscoveryState {
    /// Loads discovery state from `base_path` (the `…/dxbot/cli/` directory).
    ///
    /// A missing or empty state file yields an empty [`DiscoveryState`] rather
    /// than an error, so a fresh install reports "no Instance" instead of
    /// failing on I/O.
    pub fn load_state(base_path: &Path) -> Result<Self, DiscoveryError> {
        let path = base_path.join(DISCOVERY_STATE_FILE);
        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Ok(Self {
                    instance_endpoints: HashMap::new(),
                });
            }
            Err(e) => {
                return Err(DiscoveryError::state_io(&path, e));
            }
        };
        match serde_json::from_slice(&data) {
            Ok(state) => Ok(state),
            Err(e) => Err(DiscoveryError::state_io(&path, e.into())),
        }
    }

    /// Persists `self` to `base_path` using a temp file + fsync + atomic
    /// rename so a failed write never leaves a half-written state file.
    pub fn save_state(&self, base_path: &Path) -> Result<(), DiscoveryError> {
        fs::create_dir_all(base_path)
            .map_err(|e| DiscoveryError::state_io(base_path, e))?;
        let path = base_path.join(DISCOVERY_STATE_FILE);
        let tmp = base_path.join(format!("{DISCOVERY_STATE_FILE}.tmp"));
        let bytes =
            serde_json::to_vec(self).map_err(|e| DiscoveryError::state_io(&path, e.into()))?;
        {
            let mut file =
                File::create(&tmp).map_err(|e| DiscoveryError::state_io(&tmp, e))?;
            file.write_all(&bytes)
                .map_err(|e| DiscoveryError::state_io(&tmp, e))?;
            file.sync_all().map_err(|e| DiscoveryError::state_io(&tmp, e))?;
        }
        fs::rename(&tmp, &path).map_err(|e| DiscoveryError::state_io(&tmp, e))?;
        Ok(())
    }
}

/// Errors produced by the CLI discovery module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// No Instance matched the selection hints. Maps to `runtime-unavailable`
    /// (exit 10).
    NoInstance { message: String },
    /// More than one candidate matched. Maps to `ambiguous-target` (exit 5).
    Ambiguous { candidates: Vec<InstanceId> },
    /// A requested Instance id is not present.
    NotFound { instance_id: InstanceId },
    /// Cannot read/write discovery state.
    StateIo { path: PathBuf, message: String },
    /// First-run bootstrap attempted while Instances already exist.
    BootstrapConflict { existing: Vec<InstanceId> },
}

impl DiscoveryError {
    fn state_io(path: &Path, e: std::io::Error) -> Self {
        Self::StateIo {
            path: path.to_path_buf(),
            message: e.to_string(),
        }
    }

    /// Projects this error onto the canonical [`DxbotError`] surface so the
    /// CLI can fall through to the standard error/exit-code contract.
    pub fn to_dxbot_error(&self) -> DxbotError {
        match self {
            Self::NoInstance { message } => dxbot_error(
                ErrorCode::RuntimeUnavailable,
                ErrorCategory::Availability,
                message.clone(),
            ),
            Self::Ambiguous { candidates } => {
                let names: Vec<String> =
                    candidates.iter().map(|id| id.0.clone()).collect();
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
                let names: Vec<String> =
                    existing.iter().map(|id| id.0.clone()).collect();
                dxbot_error(
                    ErrorCode::Conflict,
                    ErrorCategory::Conflict,
                    format!("first-run bootstrap refused: Instances already exist ({names:?})"),
                )
            }
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
            return PathBuf::from(home).join(".local").join("state").join("dxbot").join("cli");
        }
    }
    PathBuf::from("dxbot-state").join("cli")
}

/// CLI discovery entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    base_path: PathBuf,
}

impl Discovery {
    /// Creates a discovery view rooted at the default CLI state directory
    /// (`$XDG_STATE_HOME/dxbot/cli`, falling back to `~/.local/state/dxbot/cli`).
    pub fn new() -> Self {
        Self::at(default_base_path())
    }

    /// Creates a discovery view rooted at `base_path`.
    ///
    /// Primarily used by tests and embedders that must isolate state.
    pub fn at(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    /// Returns the top-level help text covering every command group.
    pub fn show_help() -> String {
        "\
dxb — DXBOT command line interface

Usage:
  dxb <command> [options]
  dxb --help
  dxb --version [--format json|jsonl]
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
  version       Show client version and protocol compatibility.

Instance selection (DXB-RUN-035 precedence):
  explicit --instance, then profile binding, then exactly one verified local
  endpoint. No silent fallback. No Instance is runtime-unavailable (exit 10);
  more than one candidate is ambiguous-target (exit 5).

First run:
  `dxb runtime start` bootstraps the first Instance only when none exist.
  Other commands never auto-start or auto-bootstrap a Runtime.
"
    .to_string()
    }

    /// Returns the client version information.
    pub fn show_version() -> VersionInfo {
        VersionInfo {
            client_version: env!("CARGO_PKG_VERSION").to_string(),
            supported_protocol_versions: vec!["1".to_string()],
            supported_schema_versions: vec!["v1".to_string()],
            remote_compatibility: None,
        }
    }

    /// Selects exactly one Instance from `profile`/`instance` hints.
    ///
    /// Selection is exact and follows DXB-RUN-035 precedence. Returns
    /// [`DiscoveryError::NoInstance`] when nothing matches and
    /// [`DiscoveryError::Ambiguous`] when multiple candidates match. It never
    /// bootstraps and never silently falls back to another Instance.
    pub fn select_instance(
        &self,
        profile: Option<&str>,
        instance: Option<&str>,
    ) -> Result<InstanceId, DiscoveryError> {
        let state = DiscoveryState::load_state(&self.base_path)?;
        let mut candidates: Vec<InstanceId> = if let Some(name) = instance {
            state
                .instance_endpoints
                .keys()
                .filter(|id| id.0 == name)
                .cloned()
                .collect()
        } else if let Some(prof) = profile {
            state
                .instance_endpoints
                .values()
                .filter(|ep| ep.profile.as_deref() == Some(prof))
                .map(|ep| ep.instance_id.clone())
                .collect()
        } else {
            state.instance_endpoints.keys().cloned().collect()
        };
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        candidates.dedup();

        if candidates.len() == 1 {
            return Ok(candidates.remove(0));
        }
        if candidates.is_empty() {
            let wanted = match (profile, instance) {
                (_, Some(name)) => format!("instance '{name}'"),
                (Some(prof), None) => format!("profile '{prof}'"),
                (None, None) => "any local Instance".to_string(),
            };
            return Err(DiscoveryError::NoInstance {
                message: format!("no Runtime Instance found matching {wanted}"),
            });
        }
        Err(DiscoveryError::Ambiguous { candidates })
    }

    /// Bootstraps the first Instance when none exist.
    ///
    /// Creates the default state directory and the default Instance, persists
    /// the state, and returns the new InstanceId. Refuses
    /// ([`DiscoveryError::BootstrapConflict`]) when Instances already exist.
    pub fn first_run_bootstrap(&self) -> Result<InstanceId, DiscoveryError> {
        let mut state = DiscoveryState::load_state(&self.base_path)?;
        if !state.instance_endpoints.is_empty() {
            let existing: Vec<InstanceId> =
                state.instance_endpoints.keys().cloned().collect();
            return Err(DiscoveryError::BootstrapConflict { existing });
        }

        let id = InstanceId("default".to_string());
        let descriptor = DiscoveryEndpoint {
            instance_id: id.clone(),
            profile: Some("default".to_string()),
            endpoint: "local://default".to_string(),
            host_generation: 1,
            provider_id: "reference".to_string(),
            provider_ready: true,
        };
        state.instance_endpoints.insert(id.clone(), descriptor);
        state.save_state(&self.base_path)?;
        Ok(id)
    }

    /// Diagnoses provider availability for the given Instance.
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
                .map(|s| s.to_string())
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
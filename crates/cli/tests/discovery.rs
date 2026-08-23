//! Acceptance coverage for `AT-CLI-DISCOVERY-001`: local help/version, first-run
//! bootstrap, exact Instance selection, and provider doctor.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use cli::discovery::{Discovery, DiscoveryError, DiscoveryState, DiscoveryEndpoint};
use dxbot_core::error::ErrorCode;
use dxbot_core::types::InstanceId;

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
    let dir = std::env::temp_dir().join(format!("dxbot-discovery-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    TempDir(dir)
}

fn discovery(base: &Path) -> Discovery {
    Discovery::at(base.to_path_buf())
}

fn endpoint(name: &str, profile: &str) -> (InstanceId, DiscoveryEndpoint) {
    let id = InstanceId(name.to_string());
    let descriptor = DiscoveryEndpoint {
        instance_id: id.clone(),
        profile: Some(profile.to_string()),
        endpoint: format!("local://{name}"),
        host_generation: 1,
        provider_id: "reference".to_string(),
        provider_ready: true,
    };
    (id, descriptor)
}

#[test]
fn discovery_help_contains_all_command_groups() {
    let help = Discovery::show_help();
    assert!(!help.trim().is_empty(), "help text must not be empty");
    for group in [
        "bot",
        "conversation",
        "thread",
        "task",
        "runtime",
        "project",
        "channel",
        "memory",
        "approval",
        "provider",
    ] {
        assert!(
            help.contains(group),
            "help must mention command group '{group}'"
        );
    }
}

#[test]
fn discovery_version_returns_client_version() {
    let version = Discovery::show_version();
    assert!(
        !version.client_version.is_empty(),
        "client version must be non-empty"
    );
    assert!(!version.supported_protocol_versions.is_empty());
    assert!(!version.supported_schema_versions.is_empty());
}

#[test]
fn discovery_select_instance_returns_error_when_none_found() {
    let base = temp_base();
    let d = discovery(base.as_path());
    let result = d.select_instance(None, None);
    let err = match result {
        Err(e) => e,
        Ok(id) => panic!("expected NoInstance, got Instance {id:?}"),
    };
    assert!(matches!(err, DiscoveryError::NoInstance { .. }));
    // Maps to runtime-unavailable, exit 10.
    let dx = err.to_dxbot_error();
    assert_eq!(dx.code, ErrorCode::RuntimeUnavailable);
    assert_eq!(dx.code.exit_code(), 10);
}

#[test]
fn discovery_select_instance_returns_error_when_ambiguous() {
    let base = temp_base();
    let mut endpoints = HashMap::new();
    let (id_a, ep_a) = endpoint("a", "alpha");
    let (id_b, ep_b) = endpoint("b", "beta");
    endpoints.insert(id_a.clone(), ep_a);
    endpoints.insert(id_b.clone(), ep_b);
    DiscoveryState {
        instance_endpoints: endpoints,
    }
    .save_state(base.as_path())
    .unwrap();

    let d = discovery(base.as_path());
    match d.select_instance(None, None) {
        Err(DiscoveryError::Ambiguous { candidates }) => {
            assert_eq!(candidates, vec![id_a.clone(), id_b.clone()]);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }

    // Explicit --instance resolves exactly.
    assert_eq!(d.select_instance(None, Some("a")).unwrap(), id_a);
    // Profile binding resolves exactly.
    assert_eq!(d.select_instance(Some("beta"), None).unwrap(), id_b);
    // Explicit --instance takes precedence over an unmatched profile.
    assert_eq!(d.select_instance(Some("nope"), Some("b")).unwrap(), id_b);
}

#[test]
fn discovery_first_run_bootstrap_creates_instance() {
    let base = temp_base();
    let d = discovery(base.as_path());
    let id = d.first_run_bootstrap().unwrap();
    assert_eq!(id, InstanceId("default".to_string()));

    // State file created inside the state directory.
    let state_file = base.as_path().join(cli::discovery::DISCOVERY_STATE_FILE);
    assert!(
        state_file.exists(),
        "bootstrap must persist discovery state"
    );

    // The bootstrapped Instance is now selectable.
    assert_eq!(d.select_instance(None, None).unwrap(), id);

    // Bootstrap refuses once an Instance exists.
    match d.first_run_bootstrap() {
        Err(DiscoveryError::BootstrapConflict { .. }) => {}
        other => panic!("expected BootstrapConflict, got {other:?}"),
    }
}

#[test]
fn discovery_doctor_provider_reports_status() {
    let base = temp_base();
    let d = discovery(base.as_path());
    let id = d.first_run_bootstrap().unwrap();

    let diagnostic = d.doctor_provider(&id).unwrap();
    assert_eq!(diagnostic.provider_id, "reference");
    assert_eq!(diagnostic.status, "ready");
    assert!(diagnostic.available);
    assert!(!diagnostic.required_capabilities.is_empty());

    // Unknown Instance reports NotFound.
    match d.doctor_provider(&InstanceId("missing".to_string())) {
        Err(DiscoveryError::NotFound { instance_id }) => {
            assert_eq!(instance_id, InstanceId("missing".to_string()));
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn discovery_no_auto_start_on_other_commands() {
    let base = temp_base();
    let d = discovery(base.as_path());

    // Merely selecting/bootstrapping-checks must not auto-start a Runtime or
    // auto-bootstrap an Instance; selection on an empty state fails closed.
    assert!(matches!(
        d.select_instance(None, None),
        Err(DiscoveryError::NoInstance { .. })
    ));

    // And it must not have created any state or state directory.
    let state_file = base.as_path().join(cli::discovery::DISCOVERY_STATE_FILE);
    assert!(
        !state_file.exists(),
        "selection must not create discovery state"
    );
}

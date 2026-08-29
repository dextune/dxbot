//! Acceptance coverage for `AT-CLI-DISCOVERY-001`: local help/version, exact
//! Instance selection, fail-closed first-run bootstrap, and provider doctor.
#![allow(clippy::unwrap_used)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use cli::discovery::{Discovery, DiscoveryEndpoint, DiscoveryError, DiscoveryState};
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
        endpoint: format!("unix:///tmp/{name}.sock"),
        host_generation: 1,
        provider_id: format!("provider-{name}"),
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
    assert!(help.contains("never fabricates an endpoint"));
}

#[test]
fn discovery_version_returns_client_version() {
    let version = Discovery::show_version();
    assert!(!version.client_version.is_empty());
    assert!(!version.supported_protocol_versions.is_empty());
    assert!(!version.supported_schema_versions.is_empty());
}

#[test]
fn discovery_select_instance_returns_error_when_none_found() {
    let base = temp_base();
    let d = discovery(base.as_path());
    let err = d.select_instance(None, None).unwrap_err();
    assert!(matches!(err, DiscoveryError::NoInstance { .. }));
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

    assert_eq!(d.select_instance(None, Some("a")).unwrap(), id_a);
    assert_eq!(d.select_instance(Some("beta"), None).unwrap(), id_b);
    assert_eq!(d.select_instance(Some("nope"), Some("b")).unwrap(), id_b);
}

#[test]
fn discovery_first_run_bootstrap_does_not_publish_unverified_state() {
    let base = temp_base();
    let d = discovery(base.as_path());

    let error = d.first_run_bootstrap().unwrap_err();
    assert!(matches!(error, DiscoveryError::NoInstance { .. }));
    assert_eq!(error.to_dxbot_error().code, ErrorCode::RuntimeUnavailable);

    let state_file = base.as_path().join(cli::discovery::DISCOVERY_STATE_FILE);
    assert!(
        !state_file.exists(),
        "CLI discovery must not synthesize a first-Instance descriptor"
    );
    assert!(matches!(
        d.select_instance(None, None),
        Err(DiscoveryError::NoInstance { .. })
    ));
}

#[test]
fn discovery_first_run_refuses_when_verified_descriptors_exist() {
    let base = temp_base();
    let (id, descriptor) = endpoint("existing", "default");
    let mut endpoints = HashMap::new();
    endpoints.insert(id, descriptor);
    DiscoveryState {
        instance_endpoints: endpoints,
    }
    .save_state(base.as_path())
    .unwrap();

    assert!(matches!(
        discovery(base.as_path()).first_run_bootstrap(),
        Err(DiscoveryError::BootstrapConflict { .. })
    ));
}

#[test]
fn discovery_doctor_provider_reports_existing_descriptor_status() {
    let base = temp_base();
    let (id, descriptor) = endpoint("ready", "default");
    let mut endpoints = HashMap::new();
    endpoints.insert(id.clone(), descriptor.clone());
    DiscoveryState {
        instance_endpoints: endpoints,
    }
    .save_state(base.as_path())
    .unwrap();

    let d = discovery(base.as_path());
    let diagnostic = d.doctor_provider(&id).unwrap();
    assert_eq!(diagnostic.provider_id, descriptor.provider_id);
    assert_eq!(diagnostic.status, "ready");
    assert!(diagnostic.available);
    assert!(!diagnostic.required_capabilities.is_empty());

    match d.doctor_provider(&InstanceId("missing".to_string())) {
        Err(DiscoveryError::NotFound { instance_id }) => {
            assert_eq!(instance_id, InstanceId("missing".to_string()));
        }
        other => panic!("expected NotFound, got {other:?}"),
    }
}

#[test]
fn discovery_selection_does_not_auto_start_or_create_state() {
    let base = temp_base();
    let d = discovery(base.as_path());

    assert!(matches!(
        d.select_instance(None, None),
        Err(DiscoveryError::NoInstance { .. })
    ));
    let state_file = base.as_path().join(cli::discovery::DISCOVERY_STATE_FILE);
    assert!(!state_file.exists());
}

//! Integration tests for runtime host lifecycle, composition readiness, and generation fencing.

use std::path::PathBuf;

use dxbot_core::types::InstanceId;
use runtime_bootstrap::BootstrapEndpoint;
use runtime_host::{
    Error, ExtensionKind, ExtensionReadiness, HostStatus, RuntimeComposition, RuntimeExtension,
    RuntimeHost,
};

fn endpoint(id: &str, generation: i64, verified: bool) -> BootstrapEndpoint {
    BootstrapEndpoint {
        instance_id: InstanceId(id.to_owned()),
        endpoint_path: PathBuf::from(format!("/tmp/endpoint-{id}-{generation}.sock")),
        host_generation: generation,
        verified,
    }
}

#[derive(Debug)]
struct TestExtension {
    id: String,
    dependencies: Vec<String>,
    start_result: Result<ExtensionReadiness, String>,
}

impl RuntimeExtension for TestExtension {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind(&self) -> ExtensionKind {
        ExtensionKind::InternalService
    }

    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    fn start(&mut self) -> Result<ExtensionReadiness, String> {
        self.start_result.clone()
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn host_start_stop_graceful_lifecycle() -> Result<(), Error> {
    let mut host = RuntimeHost::new(endpoint("i-1", 1, true));

    assert_eq!(host.status(), HostStatus::Stopped);
    assert_eq!(host.start()?, HostStatus::Running);
    assert_eq!(host.status(), HostStatus::Running);

    assert_eq!(host.stop_graceful()?, HostStatus::Stopped);
    assert_eq!(host.status(), HostStatus::Stopped);
    assert_eq!(host.stop_graceful()?, HostStatus::Stopped);
    Ok(())
}

#[test]
fn host_stop_with_wrong_generation_is_fenced() -> Result<(), Error> {
    let mut host = RuntimeHost::new(endpoint("i-2", 7, true));
    assert_eq!(host.start()?, HostStatus::Running);

    assert_eq!(
        host.stop_host(6),
        Err(Error::GenerationFenced {
            expected: 7,
            provided: 6,
        })
    );
    assert_eq!(host.status(), HostStatus::Running);

    assert_eq!(host.stop_host(7)?, HostStatus::Stopped);
    assert_eq!(host.status(), HostStatus::Stopped);
    Ok(())
}

#[test]
fn host_status_transitions_are_correct() -> Result<(), Error> {
    let mut degraded = RuntimeHost::new(endpoint("i-3", 1, false));
    assert_eq!(degraded.start()?, HostStatus::Degraded);
    assert_eq!(degraded.status(), HostStatus::Degraded);
    assert_eq!(degraded.stop_graceful()?, HostStatus::Stopped);
    assert_eq!(degraded.status(), HostStatus::Stopped);

    let mut running = RuntimeHost::new(endpoint("i-4", 2, true));
    assert_eq!(running.status(), HostStatus::Stopped);
    assert_eq!(running.start()?, HostStatus::Running);
    assert_eq!(running.status(), HostStatus::Running);
    assert_eq!(running.start()?, HostStatus::Running);
    assert_eq!(running.stop_graceful()?, HostStatus::Stopped);
    assert_eq!(running.status(), HostStatus::Stopped);
    Ok(())
}

#[test]
fn composed_host_is_not_running_before_mandatory_readiness() {
    let mut host = RuntimeHost::new(endpoint("i-5", 1, true));
    let mut composition = RuntimeComposition::new();
    assert!(composition
        .register(Box::new(TestExtension {
            id: "provider".to_owned(),
            dependencies: Vec::new(),
            start_result: Err("not ready".to_owned()),
        }))
        .is_ok());

    assert!(matches!(
        host.start_composed(&mut composition),
        Err(Error::Composition(_))
    ));
    assert_eq!(host.status(), HostStatus::Degraded);
    assert!(host.stop_composed(&mut composition).is_ok());
    assert_eq!(host.status(), HostStatus::Stopped);
}

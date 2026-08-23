//! Integration tests for runtime host lifecycle and generation fencing.

use std::path::PathBuf;

use runtime_bootstrap::BootstrapEndpoint;
use runtime_host::{Error, HostStatus, RuntimeHost};
use dxbot_core::types::InstanceId;

fn endpoint(id: &str, generation: i64, verified: bool) -> BootstrapEndpoint {
    BootstrapEndpoint {
        instance_id: InstanceId(id.to_owned()),
        endpoint_path: PathBuf::from(format!("/tmp/endpoint-{id}-{generation}.sock")),
        host_generation: generation,
        verified,
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

    // Graceful stop is idempotent from the stopped state.
    assert_eq!(host.stop_graceful()?, HostStatus::Stopped);
    assert_eq!(host.status(), HostStatus::Stopped);
    Ok(())
}

#[test]
fn host_stop_with_wrong_generation_is_fenced() -> Result<(), Error> {
    let mut host = RuntimeHost::new(endpoint("i-2", 7, true));
    assert_eq!(host.start()?, HostStatus::Running);

    // A stale generation is rejected and the host keeps running.
    assert_eq!(
        host.stop_host(6),
        Err(Error::GenerationFenced { expected: 7, provided: 6 })
    );
    assert_eq!(host.status(), HostStatus::Running);

    // The current generation may stop it.
    assert_eq!(host.stop_host(7)?, HostStatus::Stopped);
    assert_eq!(host.status(), HostStatus::Stopped);
    Ok(())
}

#[test]
fn host_status_transitions_are_correct() -> Result<(), Error> {
    // Unverified endpoint: start leads to Degraded, then can be stopped.
    let mut degraded = RuntimeHost::new(endpoint("i-3", 1, false));
    assert_eq!(degraded.start()?, HostStatus::Degraded);
    assert_eq!(degraded.status(), HostStatus::Degraded);
    assert_eq!(degraded.stop_graceful()?, HostStatus::Stopped);
    assert_eq!(degraded.status(), HostStatus::Stopped);

    // Verified endpoint: full Stopped -> Running -> Stopped lifecycle.
    let mut running = RuntimeHost::new(endpoint("i-4", 2, true));
    assert_eq!(running.status(), HostStatus::Stopped);
    assert_eq!(running.start()?, HostStatus::Running);
    assert_eq!(running.status(), HostStatus::Running);
    // Idempotent start while running.
    assert_eq!(running.start()?, HostStatus::Running);
    assert_eq!(running.stop_graceful()?, HostStatus::Stopped);
    assert_eq!(running.status(), HostStatus::Stopped);
    Ok(())
}
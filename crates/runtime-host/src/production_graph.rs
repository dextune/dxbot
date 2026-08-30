//! Production `RuntimeComposition` wiring for `LocalRuntimeHost`
//! (`DXB-DEL-068` H8).
//!
//! `LocalRuntimeHost` constructs the single canonical owners
//! (Application/Security/Audit/Provider host + Runtime-owned Tokio runtime /
//! Execution coordinator / Control endpoint) exactly once, then drives startup
//! **publication** through one `RuntimeComposition`. There is no second owner
//! graph: every extension here is a thin *lifecycle adapter* holding an `Arc`
//! (or owned handle) to an already-constructed owner. No extension constructs a
//! new instance of any canonical owner.
//!
//! The composition exists to give H8 its two guarantees:
//!
//! 1. **Single owner graph.** One `RuntimeComposition` covers every owner, so
//!    `dxb runtime start/status/doctor/stop` observe one composition generation
//!    rather than a parallel ad-hoc startup path.
//! 2. **Rollback before publication.** PID, discovery, and the control endpoint
//!    are the *terminal* extensions. `RuntimeComposition::start` disposes
//!    started extensions in strict reverse order on any failure, so a failure
//!    at any publication step leaves **zero** published endpoint, PID, or
//!    discovery entry and zero live Execution permits.
//!
//! Graceful shutdown is a *distinct* documented order (admission stop →
//! execution cancel/join → ProviderHost bounded drain → provider Tokio stop →
//! audit/persistence → endpoint/discovery/PID unpublish) owned by
//! `LocalRuntimeHost::shutdown`; it is not the strict reverse of startup and is
//! therefore not driven through `RuntimeComposition::stop`, which is reserved
//! for the reverse-order startup rollback.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use control_server::LocalControlServer;
use dxbot_core::types::InstanceId;
use runtime_bootstrap::{DiscoveryEndpoint, DiscoveryState};

use crate::composition::{ExtensionKind, ExtensionReadiness, RuntimeExtension};

/// Shared publication state, mutated only by the terminal publication
/// extensions and read back by `LocalRuntimeHost` after a successful start.
///
/// Each flag records whether that artifact is currently published, so both the
/// rollback path (`RuntimeComposition::start` reverse disposal) and the
/// graceful shutdown path can idempotently unpublish exactly what exists.
#[derive(Debug, Default)]
pub(crate) struct PublicationLedger {
    pub endpoint_published: bool,
    pub pid_published: bool,
    pub discovery_published: bool,
}

impl PublicationLedger {
    pub(crate) fn boxed() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }
}

/// A lifecycle adapter for an already-constructed internal owner. Its `start`
/// is a no-op that reports the readiness the owner was verified to have at
/// construction; its `stop` is the rollback action for that owner if a later
/// publication step fails during startup.
pub(crate) struct OwnerExtension {
    id: String,
    dependencies: Vec<String>,
    ready: ExtensionReadiness,
    required: bool,
    rollback: Box<dyn FnMut() + Send>,
}

impl OwnerExtension {
    pub(crate) fn boxed(
        id: &str,
        dependencies: &[&str],
        ready: ExtensionReadiness,
        rollback: Box<dyn FnMut() + Send>,
    ) -> Box<dyn RuntimeExtension> {
        Box::new(Self {
            id: id.to_owned(),
            dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
            ready,
            required: true,
            rollback,
        })
    }

    /// A non-mandatory owner adapter: a degraded readiness (e.g. an
    /// unconfigured/unavailable provider) does not fail startup, so the Runtime
    /// still publishes and `doctor`/`status` can report the gap.
    pub(crate) fn optional(
        id: &str,
        dependencies: &[&str],
        ready: ExtensionReadiness,
        rollback: Box<dyn FnMut() + Send>,
    ) -> Box<dyn RuntimeExtension> {
        Box::new(Self {
            id: id.to_owned(),
            dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
            ready,
            required: false,
            rollback,
        })
    }
}

impl std::fmt::Debug for OwnerExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OwnerExtension")
            .field("id", &self.id)
            .field("ready", &self.ready)
            .finish_non_exhaustive()
    }
}

impl RuntimeExtension for OwnerExtension {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> ExtensionKind {
        ExtensionKind::InternalService
    }
    fn dependencies(&self) -> &[String] {
        &self.dependencies
    }
    fn required(&self) -> bool {
        self.required
    }
    fn start(&mut self) -> Result<ExtensionReadiness, String> {
        Ok(self.ready)
    }
    fn stop(&mut self) -> Result<(), String> {
        (self.rollback)();
        Ok(())
    }
}

/// Terminal extension that marks the already-bound control endpoint as the
/// published, ready endpoint. Its `stop` unpublishes the endpoint (used only on
/// startup rollback; graceful shutdown unpublishes through its own ordered
/// step). The socket itself was bound before composition so the live server can
/// serve; this extension owns only the *published* transition and its reversal.
pub(crate) struct ControlEndpointExtension {
    id: String,
    dependencies: Vec<String>,
    server: Arc<LocalControlServer>,
    ledger: Arc<Mutex<PublicationLedger>>,
}

impl ControlEndpointExtension {
    pub(crate) fn boxed(
        server: Arc<LocalControlServer>,
        ledger: Arc<Mutex<PublicationLedger>>,
        dependencies: &[&str],
    ) -> Box<dyn RuntimeExtension> {
        Box::new(Self {
            id: "control-endpoint".to_owned(),
            dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
            server,
            ledger,
        })
    }
}

impl std::fmt::Debug for ControlEndpointExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ControlEndpointExtension")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl RuntimeExtension for ControlEndpointExtension {
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
        if let Ok(mut ledger) = self.ledger.lock() {
            ledger.endpoint_published = true;
        }
        Ok(ExtensionReadiness::Ready)
    }
    fn stop(&mut self) -> Result<(), String> {
        // Reverse-order startup rollback: stop admission and unpublish the
        // socket so a failed start leaves no bound endpoint.
        self.server.stop_admission();
        self.server
            .unpublish_endpoint()
            .map_err(|error| format!("endpoint unpublish failed: {error}"))?;
        if let Ok(mut ledger) = self.ledger.lock() {
            ledger.endpoint_published = false;
        }
        Ok(())
    }
}

/// Terminal extension that writes the owned PID artifact on `start` and removes
/// it on `stop` (startup rollback). `write_pid`/`remove_pid` are owner-supplied
/// closures so the single artifact-owning logic in `local_runtime` is not
/// duplicated here.
pub(crate) struct PidArtifactExtension {
    id: String,
    dependencies: Vec<String>,
    pid_path: PathBuf,
    ledger: Arc<Mutex<PublicationLedger>>,
    write_pid: fn(&std::path::Path) -> Result<(), std::io::Error>,
    remove_pid: fn(&std::path::Path),
}

impl PidArtifactExtension {
    pub(crate) fn boxed(
        pid_path: PathBuf,
        ledger: Arc<Mutex<PublicationLedger>>,
        write_pid: fn(&std::path::Path) -> Result<(), std::io::Error>,
        remove_pid: fn(&std::path::Path),
        dependencies: &[&str],
    ) -> Box<dyn RuntimeExtension> {
        Box::new(Self {
            id: "pid-artifact".to_owned(),
            dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
            pid_path,
            ledger,
            write_pid,
            remove_pid,
        })
    }
}

impl std::fmt::Debug for PidArtifactExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PidArtifactExtension")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl RuntimeExtension for PidArtifactExtension {
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
        (self.write_pid)(&self.pid_path).map_err(|error| format!("pid publish failed: {error}"))?;
        if let Ok(mut ledger) = self.ledger.lock() {
            ledger.pid_published = true;
        }
        Ok(ExtensionReadiness::Ready)
    }
    fn stop(&mut self) -> Result<(), String> {
        (self.remove_pid)(&self.pid_path);
        if let Ok(mut ledger) = self.ledger.lock() {
            ledger.pid_published = false;
        }
        Ok(())
    }
}

/// Terminal extension that publishes the discovery endpoint on `start` and
/// removes the owned entry on `stop` (startup rollback). This is the last
/// extension so any earlier failure never reaches discovery publication, and a
/// discovery-publish failure rolls back PID + endpoint + owners.
pub(crate) struct DiscoveryExtension {
    id: String,
    dependencies: Vec<String>,
    discovery_root: PathBuf,
    instance_id: InstanceId,
    host_generation: i64,
    endpoint_uri: String,
    endpoint: Mutex<Option<DiscoveryEndpoint>>,
    ledger: Arc<Mutex<PublicationLedger>>,
}

impl DiscoveryExtension {
    pub(crate) fn boxed(
        discovery_root: PathBuf,
        instance_id: InstanceId,
        host_generation: i64,
        endpoint_uri: String,
        endpoint: DiscoveryEndpoint,
        ledger: Arc<Mutex<PublicationLedger>>,
        dependencies: &[&str],
    ) -> Box<dyn RuntimeExtension> {
        Box::new(Self {
            id: "discovery".to_owned(),
            dependencies: dependencies.iter().map(|d| (*d).to_owned()).collect(),
            discovery_root,
            instance_id,
            host_generation,
            endpoint_uri,
            endpoint: Mutex::new(Some(endpoint)),
            ledger,
        })
    }
}

impl std::fmt::Debug for DiscoveryExtension {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiscoveryExtension")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

impl RuntimeExtension for DiscoveryExtension {
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
        let endpoint = self
            .endpoint
            .lock()
            .ok()
            .and_then(|mut slot| slot.take())
            .ok_or_else(|| "discovery endpoint already consumed".to_owned())?;
        publish_discovery(&self.discovery_root, endpoint)
            .map_err(|error| format!("discovery publish failed: {error}"))?;
        if let Ok(mut ledger) = self.ledger.lock() {
            ledger.discovery_published = true;
        }
        Ok(ExtensionReadiness::Ready)
    }
    fn stop(&mut self) -> Result<(), String> {
        remove_owned_discovery_entry(
            &self.discovery_root,
            &self.instance_id,
            self.host_generation,
            &self.endpoint_uri,
        );
        if let Ok(mut ledger) = self.ledger.lock() {
            ledger.discovery_published = false;
        }
        Ok(())
    }
}

fn publish_discovery(
    root: &std::path::Path,
    endpoint: DiscoveryEndpoint,
) -> Result<(), std::io::Error> {
    let mut state = DiscoveryState::load_state(root).map_err(crate::local_runtime::bootstrap_io)?;
    state
        .instance_endpoints
        .insert(endpoint.instance_id.clone(), endpoint);
    state
        .save_state(root)
        .map_err(crate::local_runtime::bootstrap_io)
}

/// Remove only the discovery entry this host owns (matching generation and
/// endpoint), idempotently. Used by both startup rollback and graceful
/// shutdown so the single removal rule is not duplicated.
pub(crate) fn remove_owned_discovery_entry(
    discovery_root: &std::path::Path,
    instance_id: &InstanceId,
    host_generation: i64,
    endpoint_uri: &str,
) {
    if let Ok(mut state) = DiscoveryState::load_state(discovery_root) {
        let should_remove = state
            .instance_endpoints
            .get(instance_id)
            .is_some_and(|endpoint| {
                endpoint.host_generation == host_generation && endpoint.endpoint == endpoint_uri
            });
        if should_remove {
            state.instance_endpoints.remove(instance_id);
            let _ = state.save_state(discovery_root);
        }
    }
}

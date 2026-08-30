//! Single-writer local Runtime composition and verified control endpoint
//! publication.
//!
//! This is the Runtime Host owner for first-Instance bootstrap, generation
//! fencing, stale endpoint replacement, durable Application/Security
//! composition, control-server composition, and discovery publication. The CLI
//! only spawns/contacts this owner.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use application::{ApplicationMutator, ExecutionStatus, TaskStatus};
use control_server::{ControlServer, LocalControlServer, SecurityCoordinationStore, SecurityState};
use dxbot_core::types::InstanceId;
use fs2::FileExt;
use provider_host::{ProviderHost, ProviderStatus};
use runtime_audit::AuditOutbox;
use runtime_bootstrap::bootstrap::Error as BootstrapError;
use runtime_bootstrap::{DiscoveryEndpoint, RuntimeBootstrap};
use runtime_security::SecurityStateStore;

use crate::audit_projection::AuditProjection;
use crate::composition::{ExtensionReadiness, RuntimeComposition};
use crate::production_graph::{
    self, ControlEndpointExtension, DiscoveryExtension, OwnerExtension, PidArtifactExtension,
    PublicationLedger,
};
use crate::provider_config::load_provider_composition;
use crate::scheduler::ExecutionCoordinator;

const HOST_LOCK_FILE: &str = ".runtime-host.lock";
const HOST_PID_FILE: &str = ".runtime-host.pid";
const APPLICATION_STATE_FILE: &str = "application-state.json";
const SECURITY_STATE_FILE: &str = "security-state.json";
const SECURITY_COORDINATION_FILE: &str = ".security-application-uow.json";
const AUDIT_OUTBOX_FILE: &str = "audit-outbox.json";
const BACKUPS_DIR: &str = "backups";
const OWNER_DIRECTORY_MODE: u32 = 0o700;
const OWNER_FILE_MODE: u32 = 0o600;

/// Bounded window a starting Runtime Host waits for a departing host to release
/// the single-writer lock. `runtime stop --host-stop` acknowledges the stop
/// request as soon as admission is halted, but the outgoing process still holds
/// the exclusive lock while it drains activity, checkpoints audit, and
/// unpublishes its endpoint (observed to take on the order of a second). A
/// supervised restart therefore hands the lock over rather than racing it: the
/// incoming host retries acquisition until this deadline, then fails closed so
/// a genuinely wedged owner is still surfaced instead of blocking forever.
const HOST_LOCK_HANDOFF_TIMEOUT: Duration = Duration::from_secs(10);
const HOST_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);

/// Bounded deadline for the all-provider drain during graceful shutdown
/// (`DXB-DEL-068` H8/H9). The Execution coordinator has already joined its
/// worker threads, so in the normal path in-flight leases are zero and this
/// returns immediately; this deadline only bounds the wait for a provider call
/// that is still unwinding.
const PROVIDER_SHUTDOWN_DRAIN_DEADLINE: Duration = Duration::from_secs(5);
/// Bounded reap window after cancellation is fired at the drain deadline, giving
/// parked provider calls time to observe cancellation and release their leases
/// before shutdown verifies quiescence.
const PROVIDER_SHUTDOWN_REAP_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownReport {
    pub steps: Vec<String>,
}

#[derive(Debug)]
pub struct LocalRuntimeHost {
    _host_lock: File,
    pid_path: PathBuf,
    runtime_root: PathBuf,
    discovery_root: PathBuf,
    instance_id: InstanceId,
    host_generation: i64,
    endpoint_uri: String,
    server: Arc<LocalControlServer>,
    /// The single production `RuntimeComposition` (`DXB-DEL-068` H8). It owns
    /// startup publication ordering and reverse-order rollback; every extension
    /// is a thin lifecycle adapter over an already-constructed canonical owner.
    composition: RuntimeComposition,
    /// Shared record of which artifacts are currently published, read back to
    /// drive idempotent unpublish on graceful shutdown.
    publication: Arc<Mutex<production_graph::PublicationLedger>>,
    coordinator: ExecutionCoordinator,
    /// The Provider host owner, retained for the bounded ProviderHost drain step
    /// during graceful shutdown (`DXB-DEL-068` H8/H9).
    providers: Arc<ProviderHost>,
    audit: Arc<AuditProjection>,
    application: Arc<ApplicationMutator>,
    security: Arc<Mutex<SecurityState>>,
    /// Runtime-owned Tokio runtime backing provider async execution at the
    /// scheduler boundary (`DXB-DEL-068` H8). The provider host holds only a
    /// `Handle`; ownership and shutdown live here so there is no nested,
    /// adapter-owned runtime.
    provider_runtime: Option<tokio::runtime::Runtime>,
    shutdown_steps: Vec<String>,
    shutdown_complete: bool,
}

impl LocalRuntimeHost {
    pub fn start(
        runtime_root: PathBuf,
        discovery_root: PathBuf,
        profile: Option<String>,
    ) -> Result<Self, io::Error> {
        fs::create_dir_all(&runtime_root)?;
        fs::set_permissions(
            &runtime_root,
            fs::Permissions::from_mode(OWNER_DIRECTORY_MODE),
        )?;
        let host_lock_path = runtime_root.join(HOST_LOCK_FILE);
        let host_lock = open_host_lock(&host_lock_path)?;
        acquire_single_writer_lock(&host_lock, &runtime_root)?;

        let mut bootstrap = RuntimeBootstrap::new();
        let (instance_id, host_generation) = match bootstrap.bootstrap_first_instance(&runtime_root)
        {
            Ok(instance_id) => (instance_id, 1),
            Err(BootstrapError::AlreadyBootstrapped) => {
                let instance_id = bootstrap
                    .attach_existing(&runtime_root)
                    .map_err(bootstrap_io)?;
                let generation = bootstrap.advance_host_generation().map_err(bootstrap_io)?;
                (instance_id, generation)
            }
            Err(error) => return Err(bootstrap_io(error)),
        };
        let endpoint_path = bootstrap
            .endpoint_path(&instance_id)
            .map_err(bootstrap_io)?;
        replace_bootstrap_or_stale_endpoint(&endpoint_path)?;

        // Offline fence: the exclusive Runtime Host lock is held and no control
        // server is bound yet. Migrate any legacy v1 Application snapshot to the
        // current version before the canonical Application store is opened. The
        // migration owner captures a pre-migration backup of the whole
        // consistent artifact set first, so a v1→v2 rewrite is always rollback
        // recoverable.
        let backups_root = runtime_root.join(BACKUPS_DIR);
        crate::state_migration::migrate_application_state_under_owned_lock(
            &runtime_root,
            &backups_root,
        )
        .map_err(|error| io::Error::other(format!("cannot migrate Application state: {error}")))?;

        let application = Arc::new(
            ApplicationMutator::with_persistent_state(runtime_root.join(APPLICATION_STATE_FILE))
                .map_err(|error| {
                    io::Error::other(format!("cannot restore Application state: {error:?}"))
                })?,
        );
        let (security_store, security_state) =
            SecurityStateStore::open(runtime_root.join(SECURITY_STATE_FILE)).map_err(|error| {
                io::Error::other(format!("cannot restore Security state: {error}"))
            })?;
        let mut security_state = security_state;
        // Consume the first-init manifest and register the derived local
        // operator into the canonical Security registry. Bootstrap owns the
        // initialization decision; the Security PrincipalManager remains the
        // canonical owner of principal state. Registration is idempotent across
        // restarts.
        register_owner_principal(&bootstrap, &security_store, &mut security_state)?;
        let security: Arc<Mutex<SecurityState>> = Arc::new(Mutex::new(security_state));
        let coordination_store = Arc::new(SecurityCoordinationStore::new(
            runtime_root.join(SECURITY_COORDINATION_FILE),
        ));
        let audit_outbox = Arc::new(
            AuditOutbox::open(runtime_root.join(AUDIT_OUTBOX_FILE)).map_err(|error| {
                io::Error::other(format!("cannot restore Runtime Audit outbox: {error}"))
            })?,
        );
        let audit = AuditProjection::new(audit_outbox);
        // Runtime Host owns the Tokio runtime that backs provider async
        // execution (`DXB-DEL-068` H8). The provider host receives only a
        // `Handle`; there is no nested adapter-owned runtime.
        let provider_runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_io()
            .enable_time()
            .thread_name("dxbot-provider-runtime")
            .build()
            .map_err(|error| io::Error::other(format!("cannot build provider runtime: {error}")))?;
        let provider_handle = provider_runtime.handle().clone();
        let provider_composition = load_provider_composition(&runtime_root, provider_handle)?;
        let provider_id = provider_composition.provider_id.clone();
        let provider_generation = provider_composition.provider_generation;
        let provider_ready = provider_composition.provider_ready;
        let providers = Arc::new(provider_composition.host);
        let coordinator = ExecutionCoordinator::start(
            Arc::clone(&application),
            Arc::clone(&providers),
            host_generation,
            Arc::clone(&audit),
            Arc::clone(&security),
        )
        .map_err(|error| {
            io::Error::other(format!("cannot start Execution coordinator: {error}"))
        })?;
        let diagnostics_application = Arc::clone(&application);
        let diagnostics_providers = Arc::clone(&providers);
        let diagnostics_audit = Arc::clone(&audit);
        let diagnostics_governor = coordinator.governor_handle();
        let diagnostics_runtime_root = runtime_root.clone();
        let diagnostics = Arc::new(move || {
            let snapshot = diagnostics_application.snapshot();
            let (storage_ready, recovery_tasks, recovery_executions) = match snapshot {
                Ok(state) => (
                    true,
                    state
                        .tasks
                        .values()
                        .filter(|task| task.status == TaskStatus::RecoveryRequired)
                        .count(),
                    state
                        .executions
                        .values()
                        .filter(|execution| execution.status == ExecutionStatus::RecoveryRequired)
                        .count(),
                ),
                Err(_) => (false, 0, 0),
            };
            let providers = diagnostics_providers
                .list_providers(None)
                .into_iter()
                .take(16)
                .map(|provider| {
                    serde_json::json!({
                        "provider_id": provider.id,
                        "generation": provider.generation,
                        "capability": provider.capability,
                        "status": match provider.status {
                            ProviderStatus::Ready => "ready",
                            ProviderStatus::Draining => "draining",
                            ProviderStatus::Unavailable => "unavailable",
                        },
                    })
                })
                .collect::<Vec<_>>();
            let active_permits = diagnostics_governor.active_permits();
            let audit_records = diagnostics_audit.record_count();
            serde_json::json!({
                "sections": [
                    {
                        "section": "provider",
                        "status": if providers.iter().any(|provider| provider["status"] == "ready") { "ready" } else { "unavailable" },
                        "available": providers.iter().any(|provider| provider["status"] == "ready"),
                        "providers": providers,
                    },
                    {
                        "section": "storage",
                        "status": if storage_ready { "ready" } else { "unavailable" },
                        "available": storage_ready,
                        "application_state_present": diagnostics_runtime_root.join(APPLICATION_STATE_FILE).is_file(),
                        "security_state_present": diagnostics_runtime_root.join(SECURITY_STATE_FILE).is_file(),
                    },
                    {
                        "section": "audit",
                        "status": if diagnostics_audit.is_healthy() && audit_records.is_ok() { "ready" } else { "unavailable" },
                        "available": diagnostics_audit.is_healthy() && audit_records.is_ok(),
                        "record_count": audit_records.ok(),
                    },
                    {
                        "section": "resource",
                        "status": if active_permits.is_ok() { "ready" } else { "unavailable" },
                        "available": active_permits.is_ok(),
                        "active_permits": active_permits.ok(),
                        "capacity": diagnostics_governor.capacity(),
                        "scheduler_generation": diagnostics_governor.scheduler_generation(),
                    },
                    {
                        "section": "recovery",
                        "status": if storage_ready { "ready" } else { "unavailable" },
                        "available": storage_ready,
                        "task_recovery_required": recovery_tasks,
                        "execution_recovery_required": recovery_executions,
                    }
                ]
            })
        });
        let control = Arc::new(
            ControlServer::with_persistence_shared(
                Arc::clone(&security),
                Arc::clone(&application),
                Arc::clone(&providers),
                Arc::new(security_store),
                coordination_store,
            )
            .map_err(|error| {
                io::Error::other(format!("cannot recover Control coordination: {error}"))
            })?,
        );
        let server = Arc::new(LocalControlServer::bind_with_diagnostics(
            endpoint_path.clone(),
            instance_id.clone(),
            host_generation,
            control,
            diagnostics,
        )?);
        let endpoint_uri = endpoint_uri(&endpoint_path)?;
        let pid_path = runtime_root.join(HOST_PID_FILE);

        // Build the single production RuntimeComposition (`DXB-DEL-068` H8).
        // Every extension is a thin lifecycle adapter over an owner that is
        // already constructed and verified above; none constructs a new
        // instance. The Application/Security/Audit/Provider/Execution owners are
        // no-op-ready adapters (their teardown is the graceful shutdown order,
        // and on a startup-rollback path the owners built above are dropped,
        // which reaps the Tokio runtime and joins the coordinator). The
        // terminal control-endpoint → PID → discovery extensions perform the
        // only real publication, so `RuntimeComposition::start`'s reverse-order
        // rollback guarantees zero published endpoint/PID/discovery on any
        // failure at or after the first publication step.
        let publication = PublicationLedger::boxed();
        let mut composition = RuntimeComposition::new();
        composition
            .register(OwnerExtension::boxed(
                "application-owner",
                &[],
                ExtensionReadiness::Ready,
                Box::new(|| {}),
            ))
            .map_err(composition_io)?;
        composition
            .register(OwnerExtension::boxed(
                "security-owner",
                &["application-owner"],
                ExtensionReadiness::Ready,
                Box::new(|| {}),
            ))
            .map_err(composition_io)?;
        composition
            .register(OwnerExtension::boxed(
                "audit-owner",
                &["application-owner", "security-owner"],
                ExtensionReadiness::Ready,
                Box::new(|| {}),
            ))
            .map_err(composition_io)?;
        composition
            .register(OwnerExtension::optional(
                "provider-runtime",
                &["audit-owner"],
                if provider_ready {
                    ExtensionReadiness::Ready
                } else {
                    // An unconfigured/unavailable provider is a non-mandatory
                    // degraded readiness, not a start failure: the Runtime still
                    // publishes so doctor/status can report the provider gap.
                    ExtensionReadiness::Degraded
                },
                Box::new(|| {}),
            ))
            .map_err(composition_io)?;
        composition
            .register(OwnerExtension::boxed(
                "execution-coordinator",
                &["provider-runtime"],
                ExtensionReadiness::Ready,
                Box::new(|| {}),
            ))
            .map_err(composition_io)?;
        composition
            .register(ControlEndpointExtension::boxed(
                Arc::clone(&server),
                Arc::clone(&publication),
                &["execution-coordinator"],
            ))
            .map_err(composition_io)?;
        composition
            .register(PidArtifactExtension::boxed(
                pid_path.clone(),
                Arc::clone(&publication),
                write_pid_file,
                remove_owned_pid_file,
                &["control-endpoint"],
            ))
            .map_err(composition_io)?;
        composition
            .register(DiscoveryExtension::boxed(
                discovery_root.clone(),
                instance_id.clone(),
                host_generation,
                endpoint_uri.clone(),
                DiscoveryEndpoint {
                    instance_id: instance_id.clone(),
                    profile,
                    endpoint: endpoint_uri.clone(),
                    host_generation,
                    provider_id,
                    provider_generation,
                    provider_ready,
                },
                Arc::clone(&publication),
                &["pid-artifact"],
            ))
            .map_err(composition_io)?;

        // Drive startup through the single composition. On any failure the
        // reverse-order rollback has already unpublished the endpoint, removed
        // the PID artifact, and removed the discovery entry; the coordinator and
        // provider runtime are dropped with this stack frame, releasing permits.
        composition.start().map_err(composition_io)?;

        Ok(Self {
            _host_lock: host_lock,
            pid_path,
            runtime_root,
            discovery_root,
            instance_id,
            host_generation,
            endpoint_uri,
            server,
            composition,
            publication,
            coordinator,
            providers,
            audit,
            application,
            security,
            provider_runtime: Some(provider_runtime),
            shutdown_steps: Vec::new(),
            shutdown_complete: false,
        })
    }

    pub fn instance_id(&self) -> &InstanceId {
        &self.instance_id
    }

    pub fn host_generation(&self) -> i64 {
        self.host_generation
    }

    pub fn endpoint_uri(&self) -> &str {
        &self.endpoint_uri
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    /// The single production composition's readiness snapshot (`DXB-DEL-068`
    /// H8). `dxb runtime status`/`doctor` observe this one composition rather
    /// than a parallel startup path, so mandatory-ready counts and any degraded
    /// extension ids reflect the real owner graph.
    pub fn composition_readiness(&self) -> crate::composition::RuntimeReadiness {
        self.composition.readiness()
    }

    pub fn audit_healthy(&self) -> bool {
        self.audit.is_healthy()
    }

    pub fn audit_record_count(&self) -> Result<usize, String> {
        self.audit.record_count()
    }

    pub fn serve(&self) -> Result<(), io::Error> {
        self.server.serve()
    }

    pub fn serve_one(&self) -> Result<(), io::Error> {
        self.server.serve_one()
    }
    pub fn shutdown(&mut self) -> Result<ShutdownReport, io::Error> {
        if self.shutdown_complete {
            return Ok(ShutdownReport {
                steps: self.shutdown_steps.clone(),
            });
        }

        let mut first_error: Option<io::Error> = None;
        self.server.stop_admission();
        self.shutdown_steps.push("admission-stopped".to_owned());

        if let Err(error) = self.coordinator.shutdown() {
            first_error = Some(io::Error::other(error));
        }
        self.shutdown_steps.push("activity-drained".to_owned());

        // ProviderHost bounded all-provider drain (`DXB-DEL-068` H8/H9). The
        // host is shared with the control server as an `Arc<ProviderHost>`, so
        // this drives the real shared drain directly through `&self` — no
        // `Arc::get_mut` and no no-op. Every generation stops admitting new
        // activity, the drain waits a bounded deadline for in-flight leases to
        // reach zero, and any call still parked at the deadline has its tracked
        // cancellation token fired so it can unwind. The coordinator has already
        // joined its worker threads above, so in the normal path leases are
        // zero and this returns immediately; the bounded cancel/reap window only
        // matters if a provider call is still unwinding.
        let provider_drained = self
            .providers
            .drain_all_to_quiescence(PROVIDER_SHUTDOWN_DRAIN_DEADLINE);
        if provider_drained {
            self.shutdown_steps.push("provider-drained".to_owned());
        } else {
            // Cancellation has been fired for every tracked token; give the
            // parked calls a bounded reap window to observe it and release their
            // leases, then verify quiescence. Only report provider-drained after
            // verified zero leases; otherwise fail closed with a shutdown error
            // rather than claiming a drain that did not happen.
            let reaped = self
                .providers
                .drain_all_to_quiescence(PROVIDER_SHUTDOWN_REAP_DEADLINE);
            if reaped {
                self.shutdown_steps.push("provider-drained".to_owned());
            } else {
                self.shutdown_steps
                    .push("provider-drain-incomplete".to_owned());
                if first_error.is_none() {
                    first_error = Some(io::Error::other(
                        "Provider host did not reach zero in-flight leases within the bounded shutdown drain/cancel/reap window",
                    ));
                }
            }
        }

        // The Execution coordinator has joined all worker threads, so no
        // provider `block_on` is in flight. Shut the Runtime-owned Tokio
        // runtime down without blocking the caller thread indefinitely on any
        // lingering background task (`DXB-DEL-068` H8).
        if let Some(runtime) = self.provider_runtime.take() {
            runtime.shutdown_timeout(Duration::from_secs(5));
        }
        self.shutdown_steps
            .push("provider-runtime-stopped".to_owned());

        if let Err(error) = self.audit.drain(&self.application, &self.security) {
            if first_error.is_none() {
                first_error = Some(io::Error::other(error));
            }
        }
        self.shutdown_steps.push("audit-checkpointed".to_owned());

        if let Err(error) = self.server.unpublish_endpoint() {
            if first_error.is_none() {
                first_error = Some(error);
            }
        }
        if let Ok(mut ledger) = self.publication.lock() {
            ledger.endpoint_published = false;
        }
        self.shutdown_steps.push("endpoint-unpublished".to_owned());

        production_graph::remove_owned_discovery_entry(
            &self.discovery_root,
            &self.instance_id,
            self.host_generation,
            &self.endpoint_uri,
        );
        if let Ok(mut ledger) = self.publication.lock() {
            ledger.discovery_published = false;
        }
        self.shutdown_steps.push("discovery-unpublished".to_owned());

        remove_owned_pid_file(&self.pid_path);
        if let Ok(mut ledger) = self.publication.lock() {
            ledger.pid_published = false;
        }
        self.shutdown_steps.push("pid-removed".to_owned());
        self.shutdown_complete = true;

        if let Some(error) = first_error {
            Err(error)
        } else {
            Ok(ShutdownReport {
                steps: self.shutdown_steps.clone(),
            })
        }
    }
}

impl Drop for LocalRuntimeHost {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn open_host_lock(path: &Path) -> Result<File, io::Error> {
    let file = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!(
                        "Runtime Host lock is not a direct regular file: {}",
                        path.display()
                    ),
                ));
            }
            OpenOptions::new().read(true).write(true).open(path)?
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?,
        Err(error) => return Err(error),
    };
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_FILE_MODE))?;
    Ok(file)
}

/// Acquire the exclusive single-writer lock, tolerating a bounded handoff window
/// during which an outgoing host is still releasing it after a graceful stop.
/// Retries `try_lock_exclusive` until [`HOST_LOCK_HANDOFF_TIMEOUT`]; if the lock
/// is still held past the deadline the incoming host fails closed with the
/// owning PID so a wedged process is diagnosable rather than silently retried
/// forever. This preserves the single-writer invariant — only one host ever
/// holds the lock at a time — while making supervised stop/restart robust.
fn acquire_single_writer_lock(host_lock: &File, runtime_root: &Path) -> Result<(), io::Error> {
    let deadline = Instant::now() + HOST_LOCK_HANDOFF_TIMEOUT;
    loop {
        match host_lock.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(error) => {
                if Instant::now() >= deadline {
                    let diag = fs::read_to_string(runtime_root.join(HOST_PID_FILE))
                        .unwrap_or_else(|_| "<no-pidfile>".to_owned());
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        format!(
                            "another Runtime Host owns {}: {error} [pidfile={}]",
                            runtime_root.display(),
                            diag.trim()
                        ),
                    ));
                }
                std::thread::sleep(HOST_LOCK_RETRY_INTERVAL);
            }
        }
    }
}

fn write_pid_file(path: &Path) -> Result<(), io::Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.file_type().is_file() => {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "Runtime Host PID artifact is not a direct regular file: {}",
                    path.display()
                ),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_FILE_MODE))?;
    writeln!(file, "{}", std::process::id())?;
    file.sync_all()?;
    if let Some(parent) = path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

fn remove_owned_pid_file(path: &Path) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return;
    }
    let Ok(contents) = fs::read_to_string(path) else {
        return;
    };
    if contents.trim() == std::process::id().to_string() {
        let _ = fs::remove_file(path);
    }
}
/// Register the derived local operator described by the first-init manifest
/// into the canonical Security `PrincipalManager`. Idempotent: an already
/// registered owner is left untouched and no needless persist occurs.
fn register_owner_principal(
    bootstrap: &RuntimeBootstrap,
    security_store: &SecurityStateStore,
    security_state: &mut SecurityState,
) -> Result<(), io::Error> {
    let manifest = bootstrap.load_manifest().map_err(bootstrap_io)?;
    let owner_ref = manifest.owner.principal_ref.clone();
    let principal_exists = security_state
        .principals
        .resolve_principal(&owner_ref)
        .is_ok();
    let operator_bound = security_state
        .authority
        .check_global_authority(&owner_ref, "operator")
        .map_err(|error| io::Error::other(format!("cannot inspect owner authority: {error}")))?;
    if principal_exists && operator_bound {
        return Ok(());
    }
    if !principal_exists {
        security_state
            .principals
            .register_principal(owner_ref.clone())
            .map_err(|error| {
                io::Error::other(format!("cannot register owner principal: {error}"))
            })?;
    }
    security_state
        .authority
        .bind_global_authority(&owner_ref, "operator")
        .map_err(|error| io::Error::other(format!("cannot bind owner authority: {error}")))?;
    security_store.persist(security_state)?;
    Ok(())
}

fn replace_bootstrap_or_stale_endpoint(path: &Path) -> Result<(), io::Error> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("refusing to replace symlink endpoint: {}", path.display()),
                ));
            }
            if !file_type.is_file() && !file_type.is_socket() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "refusing to replace unexpected endpoint type: {}",
                        path.display()
                    ),
                ));
            }
            fs::remove_file(path)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn endpoint_uri(path: &Path) -> Result<String, io::Error> {
    let path = path.to_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "local control endpoint path is not valid UTF-8",
        )
    })?;
    Ok(format!("unix://{path}"))
}

fn composition_io(error: crate::composition::CompositionError) -> io::Error {
    io::Error::other(format!("runtime composition failed: {error:?}"))
}

pub(crate) fn bootstrap_io(error: BootstrapError) -> io::Error {
    match error {
        BootstrapError::AlreadyBootstrapped => io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Runtime Instance already bootstrapped",
        ),
        BootstrapError::InstanceNotFound => {
            io::Error::new(io::ErrorKind::NotFound, "Runtime Instance not found")
        }
        BootstrapError::NoStateRoot => io::Error::new(
            io::ErrorKind::InvalidInput,
            "Runtime state root is unavailable",
        ),
        BootstrapError::CorruptState(message) | BootstrapError::Io(message) => {
            io::Error::other(message)
        }
    }
}

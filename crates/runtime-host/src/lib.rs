#![forbid(unsafe_code)]

mod audit_projection;
pub mod composition;
pub mod host;
#[cfg(unix)]
pub mod local_runtime;
#[cfg(unix)]
pub mod offline_storage;
#[cfg(unix)]
mod provider_config;
pub mod scheduler;
#[cfg(unix)]
pub mod state_migration;

pub use composition::{
    CompositionError, ExtensionKind, ExtensionReadiness, InstanceHealth, RuntimeComposition,
    RuntimeCompositionState, RuntimeExtension, RuntimeReadiness,
};
pub use host::{Error, HostStatus, RuntimeHost};
#[cfg(unix)]
pub use local_runtime::{LocalRuntimeHost, ShutdownReport};
#[cfg(unix)]
pub use offline_storage::{
    ArtifactSpec, BACKUP_ARTIFACTS, BackupManifest, ManifestEntry, OfflineStorage,
    OfflineStorageError, RestoreMode, RestoreOutcome,
};
pub use scheduler::{CoreLease, ExecutionCoordinator, ResourceGovernor};
#[cfg(unix)]
pub use state_migration::{MigrationError, MigrationOutcome, migrate_application_state};

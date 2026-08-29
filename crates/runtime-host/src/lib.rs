#![forbid(unsafe_code)]

pub mod composition;
pub mod host;
#[cfg(unix)]
pub mod local_runtime;

pub use composition::{
    CompositionError, ExtensionKind, ExtensionReadiness, InstanceHealth, RuntimeComposition,
    RuntimeCompositionState, RuntimeExtension, RuntimeReadiness,
};
pub use host::{Error, HostStatus, RuntimeHost};
#[cfg(unix)]
pub use local_runtime::LocalRuntimeHost;

#![forbid(unsafe_code)]

pub mod composition;
pub mod host;

pub use composition::{
    CompositionError, ExtensionKind, ExtensionReadiness, InstanceHealth, RuntimeComposition,
    RuntimeCompositionState, RuntimeExtension, RuntimeReadiness,
};
pub use host::{Error, HostStatus, RuntimeHost};

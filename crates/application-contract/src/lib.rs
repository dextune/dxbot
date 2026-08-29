#![forbid(unsafe_code)]
// `dxbot_core::DxbotError` is the canonical, intentionally wide error struct; every
// fallible API in this crate returns `Result<_, DxbotError>`.
#![allow(clippy::result_large_err)]

/// User-facing CLI input types parsed from argv/stdin.
pub mod cli_input;
/// Generated command payload and target materialization.
pub mod contract;
/// Canonical command metadata and user-facing path resolution.
pub mod command;
/// Verified Instance/authenticated Principal projection for production execution.
pub mod execution;
/// Versioned bounded local control protocol shared by client/server.
pub mod local_control;
/// Golden-file contract test support.
pub mod golden;

mod registry;
mod util;

pub use cli_input::CliInput;
pub use command::{
    CommandMetadata, ResolvedCommand, cli_path_tokens, command_metadata, commands_in_group,
    metadata_for_key, resolve_cli_path,
};
pub use contract::{CommandPayload, PreflightPlan, TargetMaterialization};
pub use execution::{ExecutionContext, project_for_execution};
pub use golden::GoldenContract;
pub use local_control::{
    LOCAL_CONTROL_PROTOCOL_VERSION, LOCAL_CONTROL_SCHEMA_VERSION, LocalControlCodecError,
    LocalControlHandshake, LocalControlHello, LocalControlRequest, LocalControlResponse,
    MAX_LOCAL_CONTROL_FRAME_BYTES, read_local_control_frame, write_local_control_frame,
};

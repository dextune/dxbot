#![forbid(unsafe_code)]
// `dxbot_core::DxbotError` is the canonical, intentionally wide error struct; every
// fallible API in this crate returns `Result<_, DxbotError>`.
#![allow(clippy::result_large_err)]

/// User-facing CLI input types parsed from argv/stdin.
pub mod cli_input;
/// Generated command payload and target materialization.
pub mod contract;
/// Golden-file contract test support.
pub mod golden;

mod registry;
mod util;

pub use cli_input::CliInput;
pub use contract::{CommandPayload, PreflightPlan, TargetMaterialization};
pub use golden::GoldenContract;
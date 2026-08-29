#![forbid(unsafe_code)]

#[path = "journal-sha256.rs"]
pub(crate) mod sha256;

/// Local journal: OS-locked single-writer ownership, hash chain, bounded retention.
pub mod journal;

/// Adapter from the CLI-owned durable journal to the control-client submission protocol.
pub mod submission_journal;

/// Bounded local content materialization before Prepared.
pub mod content;

/// Single-owner replay identity projection before Prepared.
pub mod identity;

/// CLI discovery: help/version, exact Instance selection, fail-closed Runtime
/// bootstrap delegation, and provider descriptor diagnostics.
pub mod discovery;

/// Production registry-driven execution orchestration.
pub mod runner;

/// Thin hidden process entrypoint delegating Runtime ownership to runtime-host.
pub mod host_process;

/// Exact, ambiguity-safe resource selector resolution (`AT-CLI-009`).
pub mod selector;

/// Stable machine interface for non-interactive automation
/// (`AT-CLI-AUTOMATION-001`).
pub mod automation;

/// Safe output writer: atomic no-replace, no-follow, fsync (`AT-EXPORT-001`).
pub mod safe_writer;

/// CLI journal recovery: bounded scan, prepared reuse, dispatching binding
/// lookup, replay projection, and stale-prepared pruning.
pub mod recovery;

/// CLI core command projection: the Bot-only path (`AT-CLI-CORE-001`).
pub mod commands;

/// Interactive journey preflight (`AT-CLI-INTERACTIVE-001`).
pub mod interactive;

/// Destructive / interactive confirmation (`AT-CLI-CONFIRM-001`).
pub mod confirmation;

/// Live task directive controls: cancel/suspend/resume/redirect projection and
/// applied-wait (`AT-CLI-010`).
pub mod directive;

pub use automation::{MachineRenderer, StreamEvent};
pub use commands::{CommandsError, CoreCommands, TaskOptions};
pub use confirmation::{Confirmation, ConfirmationError};
pub use content::{MAX_MATERIALIZED_CONTENT_BYTES, materialize_content};
pub use directive::{
    CMD_TASK_CANCEL, CMD_TASK_REDIRECT, CMD_TASK_RESUME, CMD_TASK_SUSPEND, DirectiveController,
    DirectiveError, WaitResult,
};
pub use discovery::{Discovery, DiscoveryState, ProviderDiagnostic};
pub use host_process::run_runtime_host_process;
pub use identity::{build_operation_request, request_digest_for_input};
pub use interactive::{
    ConflictStatus, InteractiveError, InteractiveJourney, PreflightResult, ProviderStatus,
};
pub use journal::LocalJournal;
pub use recovery::{
    BindingKey, BindingLookup, RecoveryAction, RecoveryEntry, RecoveryError, RecoveryManager,
    ReusedIds,
};
pub use runner::{CliOutput, execute};
pub use safe_writer::SafeWriter;
pub use selector::SelectorResolver;

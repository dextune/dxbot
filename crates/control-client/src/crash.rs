//! Crash fixture support for process-level testing.
//!
//! The [`CrashPoint`] variants mark the durable-state boundaries of the
//! submission protocol. The executable crash probe runs
//! [`SubmissionClient::submit_with_crash_point`](crate::SubmissionClient::submit_with_crash_point)
//! in a child process and provokes a genuine `std::process::exit` at the
//! requested boundary, using a distinct per-boundary exit code so the parent
//! harness can verify exactly where the process terminated.

/// Distinct non-zero exit codes for each crash boundary.
pub mod exit {
    /// Crashed before any `Prepared` journal write (no operation committed).
    pub const BEFORE_COMMIT: i32 = 86;
    /// Crashed after `Prepared`/`Dispatching` are durable but before the response is recorded.
    pub const AFTER_COMMIT_BEFORE_RESPONSE: i32 = 87;
    /// Crashed after `Prepared` but before the `Dispatching` write.
    pub const BEFORE_DISPATCHING: i32 = 88;
    /// Crashed after `Dispatching` is durable but before the request is sent.
    pub const AFTER_DISPATCHING_BEFORE_SEND: i32 = 89;
}

/// A deliberate process-termination boundary in the submission protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrashPoint {
    /// Terminate before any durable record is written; no operation is created.
    BeforeCommit,
    /// Terminate after the operation is committed but before the response is recorded.
    AfterCommitBeforeResponse,
    /// Terminate after the `Prepared` write but before the `Dispatching` write.
    BeforeDispatching,
    /// Terminate after the `Dispatching` write but before the request is sent.
    AfterDispatchingBeforeSend,
}

impl CrashPoint {
    /// The deterministic process exit code associated with this boundary.
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::BeforeCommit => exit::BEFORE_COMMIT,
            Self::AfterCommitBeforeResponse => exit::AFTER_COMMIT_BEFORE_RESPONSE,
            Self::BeforeDispatching => exit::BEFORE_DISPATCHING,
            Self::AfterDispatchingBeforeSend => exit::AFTER_DISPATCHING_BEFORE_SEND,
        }
    }
}
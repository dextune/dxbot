#![forbid(unsafe_code)]

/// Submission client: durable Prepared/Dispatching before send, binding lookup recovery.
pub mod client;
/// Storage contract injected into the submission state machine.
pub mod backend;
/// Crash fixture support for process-level testing.
pub mod crash;
/// In-memory journal modelling the durable submission protocol.
pub mod journal;

pub use backend::SubmissionJournal;
pub use client::{ClientError, SubmissionClient, SubmissionClientBuilder, Transport};
pub use crash::{exit, CrashPoint};
pub use journal::JournalStore;

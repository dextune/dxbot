#![forbid(unsafe_code)]

/// Storage contract injected into the submission state machine.
pub mod backend;
/// Submission client: durable Prepared/Dispatching before send, binding lookup recovery.
pub mod client;
/// Crash fixture support for process-level testing.
pub mod crash;
/// In-memory journal modelling the durable submission protocol.
pub mod journal;
/// Owner-verified bounded Unix-domain control transport.
#[cfg(unix)]
pub mod local_transport;

pub use backend::SubmissionJournal;
pub use client::{ClientError, SubmissionClient, SubmissionClientBuilder, Transport};
pub use crash::{CrashPoint, exit};
pub use journal::JournalStore;
#[cfg(unix)]
pub use local_transport::LocalControlClient;

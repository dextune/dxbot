#![forbid(unsafe_code)]

mod backstop;
mod pressure;
mod schema;
mod snapshot;
mod store;
mod types;

use rusqlite::Connection;

pub use backstop::{
    BackstopAggregate, BackstopBudget, BackstopCaptureOutcome, BackstopCoordinator, BackstopError,
    BackstopRestorePlan, BackstopSink, BackstopSnapshot, MemoryBackstopSink, QuarantineReason,
    QuarantineRegistry, QuarantinedBackstop,
};
pub use dxbot_core::receipt::ReceiptDisposition;
pub use store::ReferenceStore;
pub use types::{
    CrashPoint, OperationArtifactCounts, OperationEffect, OperationRequest, SnapshotBudget,
    SnapshotPage, SpikeError, StorageRuntimeProfile, SubmitOutcome,
};

/// Opens an in-memory SQLite connection for the isolated M1A storage spike.
///
/// This is deliberately not a public persistence contract for DXBOT.
pub fn open_reference_connection() -> rusqlite::Result<Connection> {
    Connection::open_in_memory()
}

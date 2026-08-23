#![forbid(unsafe_code)]

mod pressure;
mod schema;
mod snapshot;
mod store;
mod types;

use rusqlite::Connection;

pub use store::ReferenceStore;
pub use dxbot_core::receipt::ReceiptDisposition;
pub use types::{
    CrashPoint, OperationArtifactCounts, OperationEffect, OperationRequest,
    SnapshotBudget, SnapshotPage, SpikeError, StorageRuntimeProfile, SubmitOutcome,
};

/// Opens an in-memory SQLite connection for the isolated M1A storage spike.
///
/// This is deliberately not a public persistence contract for DXBOT.
pub fn open_reference_connection() -> rusqlite::Result<Connection> {
    Connection::open_in_memory()
}

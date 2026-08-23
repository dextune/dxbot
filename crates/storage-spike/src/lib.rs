#![forbid(unsafe_code)]

mod store;
mod types;

use rusqlite::Connection;

pub use store::ReferenceStore;
pub use types::{
    OperationArtifactCounts, OperationEffect, OperationRequest, ReceiptDisposition, SpikeError,
    SubmitOutcome,
};

/// Opens an in-memory SQLite connection for the isolated M1A storage spike.
///
/// This is deliberately not a public persistence contract for DXBOT.
pub fn open_reference_connection() -> rusqlite::Result<Connection> {
    Connection::open_in_memory()
}

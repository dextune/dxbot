#[path = "m1a-support/mod.rs"]
mod m1a_fixture;

use std::error::Error;

use rusqlite::Connection;
use storage_spike::{
    ReceiptDisposition, ReferenceStore, SnapshotBudget, SubmitOutcome,
};

use m1a_fixture::{database_path, effect, request};

#[test]
fn v1_fixture_migrates_and_orphan_receipt_requires_recovery() -> Result<(), Box<dyn Error>> {
    let path = database_path("v1-migration-recovery");
    {
        let connection = Connection::open(&path)?;
        connection.execute_batch(include_str!("fixtures/m1a-storage-v1.sql"))?;
    }

    let mut store = ReferenceStore::open_file(&path, "instance-1", 2)?;
    assert_eq!(store.schema_version()?, 2);

    let retry = request(
        "principal-a",
        "key-old",
        "command-old",
        "request-old",
        "operation-new",
    );
    assert_eq!(
        store.submit(&retry, &effect())?,
        SubmitOutcome::Existing {
            operation_id: "operation-old".to_owned(),
            disposition: ReceiptDisposition::RecoveryRequired,
            compacted: false,
        }
    );
    assert!(!store.compact_terminal_receipt("operation-old")?);

    let watermark = store.create_snapshot(
        "snapshot-after-migration",
        "query-all",
        100,
        SnapshotBudget {
            max_items: 8,
            max_retained_bytes: 1_024,
        },
    )?;
    assert_eq!(watermark, 1);
    Ok(())
}

#[path = "m1a-support/mod.rs"]
mod m1a_fixture;

use std::error::Error;
use std::io;
use std::sync::{Arc, Barrier};
use std::thread;

use storage_spike::{ReferenceStore, SnapshotBudget, SnapshotPage, SpikeError, SubmitOutcome};

use m1a_fixture::{database_path, effect_for, request};

fn submit_projection_row(
    store: &mut ReferenceStore,
    aggregate_id: &str,
    version: &str,
) -> Result<(), SpikeError> {
    let key = format!("key-{aggregate_id}-{version}");
    let command = format!("command-{aggregate_id}-{version}");
    let digest = format!("request-{aggregate_id}-{version}");
    let operation_id = format!("operation-{aggregate_id}-{version}");
    let operation = request("principal-a", &key, &command, &digest, &operation_id);
    let state = format!("state-{aggregate_id}-{version}");
    let effect = effect_for(aggregate_id, &state);
    match store.submit(&operation, &effect)? {
        SubmitOutcome::Created { .. } => Ok(()),
        _ => Err(SpikeError::InvariantViolation(
            "snapshot fixture operation was not created",
        )),
    }
}

fn required_cursor(page: &SnapshotPage) -> Result<i64, io::Error> {
    page.next_cursor
        .ok_or_else(|| io::Error::other("snapshot page is missing expected cursor"))
}

#[test]
fn snapshot_keyset_is_stable_during_concurrent_projection_mutation() -> Result<(), Box<dyn Error>> {
    let path = database_path("snapshot-concurrency");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    for aggregate_id in ["a", "b", "c", "d", "e", "f"] {
        submit_projection_row(&mut store, aggregate_id, "v1")?;
    }

    store.create_snapshot(
        "snapshot-1",
        "query-all",
        100,
        SnapshotBudget {
            max_items: 8,
            max_retained_bytes: 1_024,
        },
    )?;

    let barrier = Arc::new(Barrier::new(2));
    let mutator_barrier = Arc::clone(&barrier);
    let mutator_path = path.clone();
    let mutator = thread::spawn(move || -> Result<(), SpikeError> {
        let mut mutator_store = ReferenceStore::open_file(&mutator_path, "instance-1", 1)?;
        let _mutator_started = mutator_barrier.wait();
        submit_projection_row(&mut mutator_store, "g", "v1")?;
        submit_projection_row(&mut mutator_store, "c", "v2")?;
        if mutator_store.delete_projection_row_for_fixture("e")? != 1 {
            return Err(SpikeError::InvariantViolation(
                "snapshot fixture delete did not remove one row",
            ));
        }
        Ok(())
    });

    let _reader_started = barrier.wait();
    let page_1 = store.read_snapshot_page("snapshot-1", None, 2, 10)?;
    let cursor_1 = required_cursor(&page_1)?;
    let page_2 = store.read_snapshot_page("snapshot-1", Some(cursor_1), 2, 10)?;
    let cursor_2 = required_cursor(&page_2)?;
    let page_3 = store.read_snapshot_page("snapshot-1", Some(cursor_2), 2, 10)?;
    assert_eq!(page_3.next_cursor, None);

    let mutation_result = mutator
        .join()
        .map_err(|_| io::Error::other("snapshot mutator thread panicked"))?;
    mutation_result?;

    let snapshot_items: Vec<String> = page_1
        .items
        .into_iter()
        .chain(page_2.items)
        .chain(page_3.items)
        .collect();
    assert_eq!(snapshot_items.join(","), "a,b,c,d,e,f");

    assert!(matches!(
        store.read_snapshot_page("snapshot-1", None, 2, 101),
        Err(SpikeError::SnapshotExpired)
    ));
    assert_eq!(store.compact_expired_snapshots(101)?, 1);
    assert!(matches!(
        store.read_snapshot_page("snapshot-1", None, 2, 10),
        Err(SpikeError::SnapshotNotFound)
    ));
    Ok(())
}

#[test]
fn snapshot_creation_rejects_item_and_byte_overflow() -> Result<(), SpikeError> {
    let path = database_path("snapshot-budget");
    let mut store = ReferenceStore::open_file(&path, "instance-1", 1)?;
    submit_projection_row(&mut store, "a", "v1")?;
    submit_projection_row(&mut store, "b", "v1")?;

    assert!(matches!(
        store.create_snapshot(
            "too-many",
            "query-all",
            100,
            SnapshotBudget {
                max_items: 1,
                max_retained_bytes: 1_024,
            },
        ),
        Err(SpikeError::SnapshotLimitExceeded)
    ));
    assert!(matches!(
        store.create_snapshot(
            "too-large",
            "query-all",
            100,
            SnapshotBudget {
                max_items: 8,
                max_retained_bytes: 8,
            },
        ),
        Err(SpikeError::SnapshotLimitExceeded)
    ));
    Ok(())
}

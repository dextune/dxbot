use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::schema::verify_writer_fence;
use crate::{ReferenceStore, SnapshotBudget, SnapshotPage, SpikeError};

const MAX_SNAPSHOT_ITEMS: usize = 1_024;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1_024;
const MAX_PAGE_ITEMS: usize = 100;

impl ReferenceStore {
    /// Materializes a bounded durable keyset without retaining a long-lived read transaction.
    pub fn create_snapshot(
        &mut self,
        snapshot_id: &str,
        query_digest: &str,
        expires_at: i64,
        budget: SnapshotBudget,
    ) -> Result<i64, SpikeError> {
        validate_snapshot_budget(budget)?;

        let instance_id = self.instance_id.as_str();
        let host_generation = self.host_generation;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_writer_fence(&transaction, instance_id, host_generation)?;

        let projection_watermark: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM events",
            [],
            |row| row.get(0),
        )?;
        let query_limit = i64::try_from(budget.max_items + 1)
            .map_err(|_| SpikeError::SnapshotLimitExceeded)?;
        let items = {
            let mut statement = transaction.prepare(
                "SELECT aggregate_id FROM aggregate_state ORDER BY aggregate_id LIMIT ?1",
            )?;
            let rows = statement.query_map([query_limit], |row| row.get::<_, String>(0))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        if items.len() > budget.max_items {
            return Err(SpikeError::SnapshotLimitExceeded);
        }

        let mut retained_bytes = snapshot_id
            .len()
            .checked_add(query_digest.len())
            .ok_or(SpikeError::SnapshotLimitExceeded)?;
        for item in &items {
            retained_bytes = retained_bytes
                .checked_add(item.len())
                .ok_or(SpikeError::SnapshotLimitExceeded)?;
            if retained_bytes > budget.max_retained_bytes {
                return Err(SpikeError::SnapshotLimitExceeded);
            }
        }

        let item_count = i64::try_from(items.len()).map_err(|_| SpikeError::SnapshotLimitExceeded)?;
        let retained_bytes_i64 =
            i64::try_from(retained_bytes).map_err(|_| SpikeError::SnapshotLimitExceeded)?;
        transaction.execute(
            "INSERT INTO snapshots(\
                 snapshot_id, projection_watermark, query_digest, expires_at, item_count, retained_bytes\
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot_id,
                projection_watermark,
                query_digest,
                expires_at,
                item_count,
                retained_bytes_i64
            ],
        )?;
        for (ordinal, aggregate_id) in items.iter().enumerate() {
            let ordinal =
                i64::try_from(ordinal).map_err(|_| SpikeError::SnapshotLimitExceeded)?;
            transaction.execute(
                "INSERT INTO snapshot_items(snapshot_id, ordinal, aggregate_id) \
                 VALUES (?1, ?2, ?3)",
                params![snapshot_id, ordinal, aggregate_id],
            )?;
        }

        transaction.commit()?;
        Ok(projection_watermark)
    }

    /// Reads one bounded page from a previously materialized snapshot keyset.
    pub fn read_snapshot_page(
        &self,
        snapshot_id: &str,
        after_ordinal: Option<i64>,
        page_size: usize,
        now: i64,
    ) -> Result<SnapshotPage, SpikeError> {
        if page_size == 0 || page_size > MAX_PAGE_ITEMS {
            return Err(SpikeError::SnapshotLimitExceeded);
        }

        let metadata: Option<(i64, i64)> = self
            .connection
            .query_row(
                "SELECT projection_watermark, expires_at FROM snapshots WHERE snapshot_id = ?1",
                [snapshot_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let Some((projection_watermark, expires_at)) = metadata else {
            return Err(SpikeError::SnapshotNotFound);
        };
        if now > expires_at {
            return Err(SpikeError::SnapshotExpired);
        }

        let after_ordinal = after_ordinal.unwrap_or(-1);
        let query_limit = i64::try_from(page_size + 1)
            .map_err(|_| SpikeError::SnapshotLimitExceeded)?;
        let mut statement = self.connection.prepare(
            "SELECT ordinal, aggregate_id FROM snapshot_items \
             WHERE snapshot_id = ?1 AND ordinal > ?2 \
             ORDER BY ordinal LIMIT ?3",
        )?;
        let rows = statement.query_map(params![snapshot_id, after_ordinal, query_limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut rows = rows.collect::<Result<Vec<_>, _>>()?;
        let has_more = rows.len() > page_size;
        if has_more {
            rows.truncate(page_size);
        }
        let next_cursor = if has_more {
            rows.last().map(|(ordinal, _)| *ordinal)
        } else {
            None
        };
        let items = rows.into_iter().map(|(_, aggregate_id)| aggregate_id).collect();

        Ok(SnapshotPage {
            projection_watermark,
            items,
            next_cursor,
        })
    }

    /// Deletes only expired materialized snapshots; canonical aggregate state is untouched.
    pub fn compact_expired_snapshots(&mut self, now: i64) -> Result<usize, SpikeError> {
        let instance_id = self.instance_id.as_str();
        let host_generation = self.host_generation;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_writer_fence(&transaction, instance_id, host_generation)?;
        let deleted = transaction.execute("DELETE FROM snapshots WHERE expires_at < ?1", [now])?;
        transaction.commit()?;
        Ok(deleted)
    }
}

fn validate_snapshot_budget(budget: SnapshotBudget) -> Result<(), SpikeError> {
    if budget.max_items == 0
        || budget.max_items > MAX_SNAPSHOT_ITEMS
        || budget.max_retained_bytes == 0
        || budget.max_retained_bytes > MAX_SNAPSHOT_BYTES
    {
        return Err(SpikeError::SnapshotLimitExceeded);
    }
    Ok(())
}

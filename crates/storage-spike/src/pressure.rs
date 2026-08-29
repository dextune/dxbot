use crate::{ReferenceStore, SpikeError, StorageRuntimeProfile};

impl ReferenceStore {
    /// Reads back the effective bounded-storage pragmas used by the M1A reference profile.
    pub fn storage_runtime_profile(&self) -> Result<StorageRuntimeProfile, SpikeError> {
        let journal_mode = self
            .connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        let synchronous = self
            .connection
            .query_row("PRAGMA synchronous", [], |row| row.get(0))?;
        let wal_autocheckpoint_pages =
            self.connection
                .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))?;
        let journal_size_limit_bytes =
            self.connection
                .query_row("PRAGMA journal_size_limit", [], |row| row.get(0))?;
        let temp_store: i64 = self
            .connection
            .query_row("PRAGMA temp_store", [], |row| row.get(0))?;

        Ok(StorageRuntimeProfile {
            journal_mode,
            synchronous,
            wal_autocheckpoint_pages,
            journal_size_limit_bytes,
            temp_store_memory: temp_store == 2,
        })
    }

    /// Pins the database page ceiling to its current size for the disk-full fixture.
    ///
    /// The returned byte count is a lower-bound payload size that must require more pages
    /// than the current freelist can satisfy.
    #[doc(hidden)]
    pub fn constrain_database_for_disk_full_fixture(&self) -> Result<usize, SpikeError> {
        self.connection
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

        let page_size: i64 = self
            .connection
            .query_row("PRAGMA page_size", [], |row| row.get(0))?;
        let page_count: i64 = self
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        let freelist_count: i64 =
            self.connection
                .query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
        if page_size <= 0 || page_count <= 0 || freelist_count < 0 {
            return Err(SpikeError::InvariantViolation(
                "invalid sqlite page accounting for disk-full fixture",
            ));
        }

        self.connection
            .pragma_update(Some("main"), "max_page_count", page_count)?;
        let max_page_count: i64 =
            self.connection
                .query_row("PRAGMA main.max_page_count", [], |row| row.get(0))?;
        if max_page_count != page_count {
            return Err(SpikeError::InvariantViolation(
                "sqlite did not apply disk-full fixture page ceiling",
            ));
        }

        let required_pages =
            freelist_count
                .checked_add(8)
                .ok_or(SpikeError::InvariantViolation(
                    "disk-full fixture page count overflow",
                ))?;
        let required_bytes =
            required_pages
                .checked_mul(page_size)
                .ok_or(SpikeError::InvariantViolation(
                    "disk-full fixture byte count overflow",
                ))?;
        usize::try_from(required_bytes).map_err(|_| {
            SpikeError::InvariantViolation("disk-full fixture byte count does not fit usize")
        })
    }
}

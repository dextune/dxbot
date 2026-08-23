use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::SpikeError;

pub(crate) const SCHEMA_VERSION: i64 = 2;
const WAL_AUTOCHECKPOINT_PAGES: i64 = 64;
const JOURNAL_SIZE_LIMIT_BYTES: i64 = 262_144;

pub(crate) fn configure_connection(connection: &Connection) -> Result<(), SpikeError> {
    connection.busy_timeout(Duration::from_secs(2))?;
    connection.execute_batch(&format!(
        "PRAGMA foreign_keys = ON;\
         PRAGMA journal_mode = WAL;\
         PRAGMA synchronous = FULL;\
         PRAGMA wal_autocheckpoint = {WAL_AUTOCHECKPOINT_PAGES};\
         PRAGMA journal_size_limit = {JOURNAL_SIZE_LIMIT_BYTES};\
         PRAGMA temp_store = MEMORY;"
    ))?;
    Ok(())
}

pub(crate) fn initialize_schema(connection: &mut Connection) -> Result<(), SpikeError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(SpikeError::UnsupportedSchema(version));
    }

    match version {
        0 => create_schema_v2(connection),
        1 => migrate_v1_to_v2(connection),
        SCHEMA_VERSION => Ok(()),
        other => Err(SpikeError::UnsupportedSchema(other)),
    }
}

pub(crate) fn initialize_writer_fence(
    connection: &mut Connection,
    instance_id: &str,
    host_generation: i64,
) -> Result<(), SpikeError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let current: Option<(String, i64)> = transaction
        .query_row(
            "SELECT instance_id, host_generation FROM runtime_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match current {
        None => {
            transaction.execute(
                "INSERT INTO runtime_metadata(singleton, instance_id, host_generation) VALUES (1, ?1, ?2)",
                params![instance_id, host_generation],
            )?;
        }
        Some((stored_instance, _)) if stored_instance != instance_id => {
            return Err(SpikeError::InvariantViolation("instance identity mismatch"));
        }
        Some((_, current_generation)) if host_generation > current_generation => {
            transaction.execute(
                "UPDATE runtime_metadata SET host_generation = ?1 WHERE singleton = 1",
                [host_generation],
            )?;
            transaction.execute(
                "UPDATE receipts SET disposition = 'recovery-required' \
                 WHERE disposition = 'accepted'",
                [],
            )?;
        }
        Some(_) => {}
    }

    transaction.commit()?;
    Ok(())
}

pub(crate) fn verify_writer_fence(
    transaction: &Transaction<'_>,
    instance_id: &str,
    host_generation: i64,
) -> Result<(), SpikeError> {
    let current: Option<(String, i64)> = transaction
        .query_row(
            "SELECT instance_id, host_generation FROM runtime_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match current {
        Some((stored_instance, stored_generation))
            if stored_instance == instance_id && stored_generation == host_generation =>
        {
            Ok(())
        }
        Some(_) => Err(SpikeError::StaleWriter),
        None => Err(SpikeError::InvariantViolation("writer fence is missing")),
    }
}

fn create_schema_v2(connection: &mut Connection) -> Result<(), SpikeError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "CREATE TABLE IF NOT EXISTS runtime_metadata (\
             singleton INTEGER PRIMARY KEY CHECK (singleton = 1),\
             instance_id TEXT NOT NULL,\
             host_generation INTEGER NOT NULL\
         );\
         CREATE TABLE IF NOT EXISTS aggregate_state (\
             aggregate_id TEXT PRIMARY KEY,\
             revision INTEGER NOT NULL,\
             state_value TEXT NOT NULL,\
             last_operation_id TEXT NOT NULL\
         );\
         CREATE TABLE IF NOT EXISTS events (\
             sequence INTEGER PRIMARY KEY AUTOINCREMENT,\
             operation_id TEXT NOT NULL,\
             aggregate_id TEXT NOT NULL,\
             payload TEXT NOT NULL\
         );\
         CREATE TABLE IF NOT EXISTS outbox (\
             operation_id TEXT PRIMARY KEY,\
             payload TEXT NOT NULL\
         );\
         CREATE TABLE IF NOT EXISTS receipts (\
             operation_id TEXT PRIMARY KEY,\
             disposition TEXT NOT NULL,\
             result_ref TEXT NOT NULL,\
             resolved_binding_digest TEXT NOT NULL,\
             owner_kind TEXT NOT NULL DEFAULT 'none',\
             lease_until INTEGER,\
             last_progress INTEGER NOT NULL DEFAULT 0,\
             reconciliation_policy TEXT NOT NULL DEFAULT 'none'\
         );\
         CREATE TABLE IF NOT EXISTS command_bindings (\
             command_id TEXT PRIMARY KEY,\
             principal_ref TEXT NOT NULL,\
             key_digest TEXT NOT NULL,\
             request_digest TEXT NOT NULL,\
             operation_id TEXT NOT NULL UNIQUE,\
             terminal_disposition TEXT,\
             expires_at INTEGER NOT NULL,\
             compacted INTEGER NOT NULL DEFAULT 0 CHECK (compacted IN (0, 1))\
         );\
         CREATE TABLE IF NOT EXISTS principal_bindings (\
             principal_ref TEXT NOT NULL,\
             key_digest TEXT NOT NULL,\
             command_id TEXT NOT NULL,\
             request_digest TEXT NOT NULL,\
             operation_id TEXT NOT NULL UNIQUE,\
             terminal_disposition TEXT,\
             expires_at INTEGER NOT NULL,\
             compacted INTEGER NOT NULL DEFAULT 0 CHECK (compacted IN (0, 1)),\
             PRIMARY KEY (principal_ref, key_digest),\
             FOREIGN KEY (command_id) REFERENCES command_bindings(command_id) ON DELETE CASCADE\
         );\
         CREATE TABLE IF NOT EXISTS audit_intents (\
             operation_id TEXT PRIMARY KEY,\
             payload TEXT NOT NULL\
         );\
         CREATE TABLE IF NOT EXISTS snapshots (\
             snapshot_id TEXT PRIMARY KEY,\
             projection_watermark INTEGER NOT NULL,\
             query_digest TEXT NOT NULL,\
             expires_at INTEGER NOT NULL,\
             item_count INTEGER NOT NULL,\
             retained_bytes INTEGER NOT NULL\
         );\
         CREATE TABLE IF NOT EXISTS snapshot_items (\
             snapshot_id TEXT NOT NULL,\
             ordinal INTEGER NOT NULL,\
             aggregate_id TEXT NOT NULL,\
             PRIMARY KEY (snapshot_id, ordinal),\
             UNIQUE (snapshot_id, aggregate_id),\
             FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id) ON DELETE CASCADE\
         );\
         PRAGMA user_version = 2;",
    )?;
    transaction.commit()?;
    Ok(())
}

fn migrate_v1_to_v2(connection: &mut Connection) -> Result<(), SpikeError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute_batch(
        "ALTER TABLE receipts ADD COLUMN owner_kind TEXT NOT NULL DEFAULT 'none';\
         ALTER TABLE receipts ADD COLUMN lease_until INTEGER;\
         ALTER TABLE receipts ADD COLUMN last_progress INTEGER NOT NULL DEFAULT 0;\
         ALTER TABLE receipts ADD COLUMN reconciliation_policy TEXT NOT NULL DEFAULT 'none';\
         CREATE TABLE snapshots (\
             snapshot_id TEXT PRIMARY KEY,\
             projection_watermark INTEGER NOT NULL,\
             query_digest TEXT NOT NULL,\
             expires_at INTEGER NOT NULL,\
             item_count INTEGER NOT NULL,\
             retained_bytes INTEGER NOT NULL\
         );\
         CREATE TABLE snapshot_items (\
             snapshot_id TEXT NOT NULL,\
             ordinal INTEGER NOT NULL,\
             aggregate_id TEXT NOT NULL,\
             PRIMARY KEY (snapshot_id, ordinal),\
             UNIQUE (snapshot_id, aggregate_id),\
             FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id) ON DELETE CASCADE\
         );\
         PRAGMA user_version = 2;",
    )?;
    transaction.commit()?;
    Ok(())
}

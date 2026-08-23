use std::path::Path;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::{
    OperationArtifactCounts, OperationEffect, OperationRequest, ReceiptDisposition, SpikeError,
    SubmitOutcome,
};

const SCHEMA_VERSION: i64 = 1;
const WAL_AUTOCHECKPOINT_PAGES: i64 = 64;
const JOURNAL_SIZE_LIMIT_BYTES: i64 = 262_144;

#[derive(Debug)]
struct BindingRecord {
    principal_ref: String,
    key_digest: String,
    command_id: String,
    request_digest: String,
    operation_id: String,
    disposition: Option<String>,
    expires_at: i64,
    compacted: bool,
}

/// SQLite-backed executable reference semantics for the M1A storage spike.
///
/// The type intentionally does not define DXBOT's future public persistence trait.
pub struct ReferenceStore {
    connection: Connection,
    instance_id: String,
    host_generation: i64,
}

impl std::fmt::Debug for ReferenceStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReferenceStore")
            .field("instance_id", &self.instance_id)
            .field("host_generation", &self.host_generation)
            .finish_non_exhaustive()
    }
}

impl ReferenceStore {
    /// Opens or bootstraps a file-backed reference store for one Runtime Instance.
    pub fn open_file(
        path: &Path,
        instance_id: &str,
        host_generation: i64,
    ) -> Result<Self, SpikeError> {
        let connection = Connection::open(path)?;
        configure_connection(&connection)?;
        initialize_schema(&connection)?;
        initialize_writer_fence(&connection, instance_id, host_generation)?;

        Ok(Self {
            connection,
            instance_id: instance_id.to_owned(),
            host_generation,
        })
    }

    /// Returns the schema version observed by this reference store.
    pub fn schema_version(&self) -> Result<i64, SpikeError> {
        Ok(self
            .connection
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// Atomically accepts a new operation or returns the previously bound operation.
    pub fn submit(
        &mut self,
        request: &OperationRequest<'_>,
        effect: &OperationEffect<'_>,
    ) -> Result<SubmitOutcome, SpikeError> {
        if request.now > request.key_expires_at {
            return Ok(SubmitOutcome::IdempotencyExpired);
        }

        let instance_id = self.instance_id.as_str();
        let host_generation = self.host_generation;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_writer_fence(&transaction, instance_id, host_generation)?;

        let command_binding = lookup_command_binding(&transaction, request.command_id)?;
        let principal_binding = lookup_principal_binding(
            &transaction,
            request.principal_ref,
            request.idempotency_key_digest,
        )?;

        match (command_binding, principal_binding) {
            (None, None) => insert_new_operation(&transaction, request, effect)?,
            (Some(command), Some(principal)) => {
                let outcome = validate_existing_bindings(request, &command, &principal)?;
                transaction.commit()?;
                return Ok(outcome);
            }
            (Some(_), None) | (None, Some(_)) => return Err(SpikeError::IdempotencyConflict),
        }

        transaction.commit()?;
        Ok(SubmitOutcome::Created {
            operation_id: request.new_operation_id.to_owned(),
        })
    }

    /// Counts persisted artifacts tied to one operation identity.
    pub fn operation_artifact_counts(
        &self,
        request: &OperationRequest<'_>,
        operation_id: &str,
    ) -> Result<OperationArtifactCounts, SpikeError> {
        Ok(OperationArtifactCounts {
            aggregate_state: query_count(
                &self.connection,
                "SELECT COUNT(*) FROM aggregate_state WHERE last_operation_id = ?1",
                operation_id,
            )?,
            event: query_count(
                &self.connection,
                "SELECT COUNT(*) FROM events WHERE operation_id = ?1",
                operation_id,
            )?,
            outbox: query_count(
                &self.connection,
                "SELECT COUNT(*) FROM outbox WHERE operation_id = ?1",
                operation_id,
            )?,
            receipt: query_count(
                &self.connection,
                "SELECT COUNT(*) FROM receipts WHERE operation_id = ?1",
                operation_id,
            )?,
            command_binding: query_count(
                &self.connection,
                "SELECT COUNT(*) FROM command_bindings WHERE command_id = ?1",
                request.command_id,
            )?,
            principal_binding: self.connection.query_row(
                "SELECT COUNT(*) FROM principal_bindings WHERE principal_ref = ?1 AND key_digest = ?2",
                params![request.principal_ref, request.idempotency_key_digest],
                |row| row.get(0),
            )?,
            audit_intent: query_count(
                &self.connection,
                "SELECT COUNT(*) FROM audit_intents WHERE operation_id = ?1",
                operation_id,
            )?,
        })
    }

    /// Returns the number of accepted operations represented by global command bindings.
    pub fn operation_count(&self) -> Result<i64, SpikeError> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM command_bindings", [], |row| row.get(0))?)
    }
}

fn configure_connection(connection: &Connection) -> Result<(), SpikeError> {
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

fn initialize_schema(connection: &Connection) -> Result<(), SpikeError> {
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > SCHEMA_VERSION {
        return Err(SpikeError::UnsupportedSchema(version));
    }

    connection.execute_batch(
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
             resolved_binding_digest TEXT NOT NULL\
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
         PRAGMA user_version = 1;",
    )?;

    Ok(())
}

fn initialize_writer_fence(
    connection: &Connection,
    instance_id: &str,
    host_generation: i64,
) -> Result<(), SpikeError> {
    let current: Option<(String, i64)> = connection
        .query_row(
            "SELECT instance_id, host_generation FROM runtime_metadata WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    match current {
        None => {
            connection.execute(
                "INSERT INTO runtime_metadata(singleton, instance_id, host_generation) VALUES (1, ?1, ?2)",
                params![instance_id, host_generation],
            )?;
        }
        Some((stored_instance, _)) if stored_instance != instance_id => {
            return Err(SpikeError::InvariantViolation("instance identity mismatch"));
        }
        Some((_, current_generation)) if host_generation > current_generation => {
            connection.execute(
                "UPDATE runtime_metadata SET host_generation = ?1 WHERE singleton = 1",
                [host_generation],
            )?;
        }
        Some(_) => {}
    }

    Ok(())
}

fn verify_writer_fence(
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

fn lookup_command_binding(
    transaction: &Transaction<'_>,
    command_id: &str,
) -> Result<Option<BindingRecord>, SpikeError> {
    Ok(transaction
        .query_row(
            "SELECT principal_ref, key_digest, command_id, request_digest, operation_id, \
                    terminal_disposition, expires_at, compacted \
             FROM command_bindings WHERE command_id = ?1",
            [command_id],
            binding_from_row,
        )
        .optional()?)
}

fn lookup_principal_binding(
    transaction: &Transaction<'_>,
    principal_ref: &str,
    key_digest: &str,
) -> Result<Option<BindingRecord>, SpikeError> {
    Ok(transaction
        .query_row(
            "SELECT principal_ref, key_digest, command_id, request_digest, operation_id, \
                    terminal_disposition, expires_at, compacted \
             FROM principal_bindings WHERE principal_ref = ?1 AND key_digest = ?2",
            params![principal_ref, key_digest],
            binding_from_row,
        )
        .optional()?)
}

fn binding_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BindingRecord> {
    Ok(BindingRecord {
        principal_ref: row.get(0)?,
        key_digest: row.get(1)?,
        command_id: row.get(2)?,
        request_digest: row.get(3)?,
        operation_id: row.get(4)?,
        disposition: row.get(5)?,
        expires_at: row.get(6)?,
        compacted: row.get::<_, i64>(7)? != 0,
    })
}

#[allow(clippy::similar_names)]
fn validate_existing_bindings(
    request: &OperationRequest<'_>,
    command: &BindingRecord,
    principal: &BindingRecord,
) -> Result<SubmitOutcome, SpikeError> {
    let matches_request = command.principal_ref == request.principal_ref
        && command.key_digest == request.idempotency_key_digest
        && command.command_id == request.command_id
        && command.request_digest == request.request_digest
        && principal.principal_ref == request.principal_ref
        && principal.key_digest == request.idempotency_key_digest
        && principal.command_id == request.command_id
        && principal.request_digest == request.request_digest;

    let indexes_agree = command.operation_id == principal.operation_id
        && command.disposition == principal.disposition
        && command.expires_at == principal.expires_at
        && command.compacted == principal.compacted;

    if !matches_request || !indexes_agree {
        return Err(SpikeError::IdempotencyConflict);
    }
    if request.now > command.expires_at {
        return Ok(SubmitOutcome::IdempotencyExpired);
    }

    let disposition = command
        .disposition
        .as_deref()
        .ok_or(SpikeError::InvariantViolation(
            "binding is missing receipt disposition",
        ))
        .and_then(ReceiptDisposition::parse)?;

    Ok(SubmitOutcome::Existing {
        operation_id: command.operation_id.clone(),
        disposition,
        compacted: command.compacted,
    })
}

fn insert_new_operation(
    transaction: &Transaction<'_>,
    request: &OperationRequest<'_>,
    effect: &OperationEffect<'_>,
) -> Result<(), SpikeError> {
    transaction.execute(
        "INSERT INTO aggregate_state(aggregate_id, revision, state_value, last_operation_id) \
         VALUES (?1, 1, ?2, ?3) \
         ON CONFLICT(aggregate_id) DO UPDATE SET \
             revision = aggregate_state.revision + 1, \
             state_value = excluded.state_value, \
             last_operation_id = excluded.last_operation_id",
        params![effect.aggregate_id, effect.state_value, request.new_operation_id],
    )?;
    transaction.execute(
        "INSERT INTO events(operation_id, aggregate_id, payload) VALUES (?1, ?2, ?3)",
        params![
            request.new_operation_id,
            effect.aggregate_id,
            effect.event_payload
        ],
    )?;
    transaction.execute(
        "INSERT INTO outbox(operation_id, payload) VALUES (?1, ?2)",
        params![request.new_operation_id, effect.outbox_payload],
    )?;
    transaction.execute(
        "INSERT INTO receipts(operation_id, disposition, result_ref, resolved_binding_digest) \
         VALUES (?1, ?2, ?3, ?4)",
        params![
            request.new_operation_id,
            ReceiptDisposition::Committed.as_str(),
            effect.result_ref,
            effect.resolved_binding_digest
        ],
    )?;
    transaction.execute(
        "INSERT INTO command_bindings(\
             command_id, principal_ref, key_digest, request_digest, operation_id, \
             terminal_disposition, expires_at, compacted\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![
            request.command_id,
            request.principal_ref,
            request.idempotency_key_digest,
            request.request_digest,
            request.new_operation_id,
            ReceiptDisposition::Committed.as_str(),
            request.key_expires_at
        ],
    )?;
    transaction.execute(
        "INSERT INTO principal_bindings(\
             principal_ref, key_digest, command_id, request_digest, operation_id, \
             terminal_disposition, expires_at, compacted\
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![
            request.principal_ref,
            request.idempotency_key_digest,
            request.command_id,
            request.request_digest,
            request.new_operation_id,
            ReceiptDisposition::Committed.as_str(),
            request.key_expires_at
        ],
    )?;
    transaction.execute(
        "INSERT INTO audit_intents(operation_id, payload) VALUES (?1, ?2)",
        params![request.new_operation_id, effect.audit_payload],
    )?;

    Ok(())
}

fn query_count(connection: &Connection, sql: &str, value: &str) -> Result<i64, SpikeError> {
    Ok(connection.query_row(sql, [value], |row| row.get(0))?)
}

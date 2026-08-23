use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::schema::{
    configure_connection, initialize_schema, initialize_writer_fence, verify_writer_fence,
};
use crate::{
    CrashPoint, OperationArtifactCounts, OperationEffect, OperationRequest, ReceiptDisposition,
    SpikeError, SubmitOutcome,
};

const CRASH_EXIT_BEFORE_COMMIT: i32 = 86;
const CRASH_EXIT_AFTER_COMMIT: i32 = 87;

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
    pub(crate) connection: Connection,
    pub(crate) instance_id: String,
    pub(crate) host_generation: i64,
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
        let mut connection = Connection::open(path)?;
        configure_connection(&connection)?;
        initialize_schema(&mut connection)?;
        initialize_writer_fence(&mut connection, instance_id, host_generation)?;

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
        self.submit_internal(request, effect, None)
    }

    /// Runs the same atomic submission path with a deliberate process crash boundary.
    ///
    /// This exists only for the executable M1A child-process fixture.
    #[doc(hidden)]
    pub fn submit_with_crash_point(
        &mut self,
        request: &OperationRequest<'_>,
        effect: &OperationEffect<'_>,
        crash_point: CrashPoint,
    ) -> Result<SubmitOutcome, SpikeError> {
        self.submit_internal(request, effect, Some(crash_point))
    }

    /// Compacts a terminal Receipt payload while retaining both identity tombstones.
    ///
    /// Returns `false` when no full Receipt exists or the Receipt is recoverable/nonterminal.
    pub fn compact_terminal_receipt(&mut self, operation_id: &str) -> Result<bool, SpikeError> {
        let instance_id = self.instance_id.as_str();
        let host_generation = self.host_generation;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_writer_fence(&transaction, instance_id, host_generation)?;

        let disposition: Option<String> = transaction
            .query_row(
                "SELECT disposition FROM receipts WHERE operation_id = ?1",
                [operation_id],
                |row| row.get(0),
            )
            .optional()?;
        let Some(disposition) = disposition else {
            return Ok(false);
        };
        let parsed = ReceiptDisposition::parse(&disposition)?;
        if !parsed.is_terminal() {
            return Ok(false);
        }

        let command_rows = transaction.execute(
            "UPDATE command_bindings \
             SET terminal_disposition = ?2, compacted = 1 \
             WHERE operation_id = ?1",
            params![operation_id, disposition.as_str()],
        )?;
        let principal_rows = transaction.execute(
            "UPDATE principal_bindings \
             SET terminal_disposition = ?2, compacted = 1 \
             WHERE operation_id = ?1",
            params![operation_id, disposition.as_str()],
        )?;
        if command_rows != 1 || principal_rows != 1 {
            return Err(SpikeError::InvariantViolation(
                "terminal receipt does not have two binding indexes",
            ));
        }

        let deleted = transaction.execute(
            "DELETE FROM receipts WHERE operation_id = ?1",
            [operation_id],
        )?;
        if deleted != 1 {
            return Err(SpikeError::InvariantViolation(
                "terminal receipt compaction lost receipt row",
            ));
        }

        transaction.commit()?;
        Ok(true)
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
                "SELECT COUNT(*) FROM principal_bindings \
                 WHERE principal_ref = ?1 AND key_digest = ?2",
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

    /// Deletes one projection row only for the concurrent snapshot fixture.
    #[doc(hidden)]
    pub fn delete_projection_row_for_fixture(
        &mut self,
        aggregate_id: &str,
    ) -> Result<usize, SpikeError> {
        let instance_id = self.instance_id.as_str();
        let host_generation = self.host_generation;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        verify_writer_fence(&transaction, instance_id, host_generation)?;
        let deleted = transaction.execute(
            "DELETE FROM aggregate_state WHERE aggregate_id = ?1",
            [aggregate_id],
        )?;
        transaction.commit()?;
        Ok(deleted)
    }

    fn submit_internal(
        &mut self,
        request: &OperationRequest<'_>,
        effect: &OperationEffect<'_>,
        crash_point: Option<CrashPoint>,
    ) -> Result<SubmitOutcome, SpikeError> {
        if request.idempotency_key_principal_ref != request.principal_ref {
            return Err(SpikeError::IdempotencyConflict);
        }
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

        let outcome = match (command_binding, principal_binding) {
            (None, None) => {
                insert_new_operation(&transaction, request, effect)?;
                if crash_point == Some(CrashPoint::BeforeCommit) {
                    std::process::exit(CRASH_EXIT_BEFORE_COMMIT);
                }
                SubmitOutcome::Created {
                    operation_id: request.new_operation_id.to_owned(),
                }
            }
            (Some(command), Some(principal)) => {
                validate_existing_bindings(&transaction, request, &command, &principal)?
            }
            (Some(_), None) | (None, Some(_)) => return Err(SpikeError::IdempotencyConflict),
        };

        transaction.commit()?;
        if crash_point == Some(CrashPoint::AfterCommitBeforeResponse) {
            std::process::exit(CRASH_EXIT_AFTER_COMMIT);
        }
        Ok(outcome)
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
    transaction: &Transaction<'_>,
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

    let disposition = if command.compacted {
        command
            .disposition
            .as_deref()
            .ok_or(SpikeError::InvariantViolation(
                "compacted binding is missing terminal disposition",
            ))
            .and_then(ReceiptDisposition::parse)?
    } else {
        let receipt_disposition: Option<String> = transaction
            .query_row(
                "SELECT disposition FROM receipts WHERE operation_id = ?1",
                [command.operation_id.as_str()],
                |row| row.get(0),
            )
            .optional()?;
        ReceiptDisposition::parse(receipt_disposition.as_deref().ok_or(
            SpikeError::InvariantViolation("full receipt is missing for active binding"),
        )?)?
    };

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
        "INSERT INTO receipts(\
             operation_id, disposition, result_ref, resolved_binding_digest, owner_kind, \
             lease_until, last_progress, reconciliation_policy\
         ) VALUES (?1, ?2, ?3, ?4, 'none', NULL, ?5, 'none')",
        params![
            request.new_operation_id,
            ReceiptDisposition::Committed.as_str(),
            effect.result_ref,
            effect.resolved_binding_digest,
            request.now
        ],
    )?;
    transaction.execute(
        "INSERT INTO command_bindings(\
             command_id, principal_ref, key_digest, request_digest, operation_id, \
             terminal_disposition, expires_at, compacted\
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, 0)",
        params![
            request.command_id,
            request.principal_ref,
            request.idempotency_key_digest,
            request.request_digest,
            request.new_operation_id,
            request.key_expires_at
        ],
    )?;
    transaction.execute(
        "INSERT INTO principal_bindings(\
             principal_ref, key_digest, command_id, request_digest, operation_id, \
             terminal_disposition, expires_at, compacted\
         ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, 0)",
        params![
            request.principal_ref,
            request.idempotency_key_digest,
            request.command_id,
            request.request_digest,
            request.new_operation_id,
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

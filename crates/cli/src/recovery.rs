//! CLI journal recovery projection for DXB-RUN-033.
//!
//! Recovery decisions are made from chain-validated durable journal state plus
//! the *fresh invocation* digest. Stored values are never compared to their own
//! projection, stale scan entries are revalidated against disk, and replay keys
//! are checked before any continued observation.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use dxbot_core::types::*;
use dxbot_core::ReceiptRecord;

use crate::journal::{JournalError, LocalJournal, PrunePolicy};

const MAX_SCAN_ENTRIES: usize = 100_000;
const DEFAULT_PRUNE_MIN_RETENTION_SECONDS: u64 = 300;
const OPERATION_PREFIX: &str = "op-recovery:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    Journal(JournalError),
    Corrupt(String),
    UnsupportedVersion(String),
    ChainMismatch(String),
    NotDispatchable(CommandId),
    NotFound(CommandId),
    BindingConflict(CommandId),
    CapacityBoundExceeded(usize),
}

impl From<JournalError> for RecoveryError {
    fn from(error: JournalError) -> Self {
        match error {
            JournalError::ChainIntegrity(message) => Self::ChainMismatch(message),
            JournalError::Corrupt(message) => Self::Corrupt(message),
            JournalError::CapacityBoundExceeded(count) => Self::CapacityBoundExceeded(count),
            JournalError::NotFound(command_id) => Self::NotFound(command_id),
            other => Self::Journal(other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    pub command_id: CommandId,
    pub state: JournalState,
    /// Chain-validated stored RequestDigest projection. A fresh invocation must
    /// supply its own digest separately to [`RecoveryManager::reuse_prepared`].
    pub semantic_digest: String,
    pub records: Vec<JournalRecord>,
}

impl RecoveryEntry {
    fn last(&self) -> Result<&JournalRecord, RecoveryError> {
        self.records.last().ok_or_else(|| {
            RecoveryError::Corrupt(format!(
                "entry {} has no journal records",
                self.command_id.0
            ))
        })
    }

    fn primary_key(&self) -> Result<IdempotencyKey, RecoveryError> {
        Ok(self.last()?.idempotency_key.clone())
    }

    fn validate_projection(&self) -> Result<&JournalRecord, RecoveryError> {
        let last = self.last()?;
        if last.command_id != self.command_id
            || last.state != self.state
            || last.request_digest.0 != self.semantic_digest
        {
            return Err(RecoveryError::Corrupt(format!(
                "recovery entry projection diverged from journal for {}",
                self.command_id.0
            )));
        }
        Ok(last)
    }

    fn reused_ids(&self) -> Result<ReusedIds, RecoveryError> {
        self.validate_projection()?;
        Ok(ReusedIds {
            command_id: self.command_id.clone(),
            // Local recovery projection only. The current journal schema does
            // not persist the server OperationId; authenticated server binding
            // lookup remains authoritative for an already-dispatched request.
            operation_id: OperationId(format!("{OPERATION_PREFIX}{}", self.command_id.0)),
            idempotency_key: self.primary_key()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusedIds {
    pub command_id: CommandId,
    pub operation_id: OperationId,
    pub idempotency_key: IdempotencyKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    Continue { ids: ReusedIds },
    Prune,
    Abandon,
    Resolved { result: Box<OperationResult> },
    Replay { ids: ReusedIds },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingKey {
    pub command_id: CommandId,
    pub idempotency_key: IdempotencyKey,
}

pub type BindingLookup = fn(&BindingKey) -> Result<Option<OperationResult>, RecoveryError>;

fn mock_binding_lookup(_key: &BindingKey) -> Result<Option<OperationResult>, RecoveryError> {
    Ok(None)
}

fn continuation_result(
    command_id: &CommandId,
    instance_id: &InstanceId,
    request_digest: &RequestDigest,
    status: &str,
) -> OperationResult {
    let operation_id = OperationId(format!("{OPERATION_PREFIX}{}", command_id.0));
    OperationResult {
        operation_id: operation_id.clone(),
        command_id: command_id.clone(),
        instance_id: instance_id.clone(),
        receipt: ReceiptRecord {
            operation_id: operation_id.0.clone(),
            disposition: dxbot_core::ReceiptDisposition::Accepted,
            result_ref: operation_id.0.clone(),
            resolved_binding_digest: request_digest.0.clone(),
            owner_kind: "instance".to_owned(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: "binding-lookup-required".to_owned(),
        },
        status: status.to_owned(),
        committed_payload: None,
        error: None,
        operation_may_continue: true,
    }
}

#[derive(Debug, Clone)]
pub struct RecoveryManager {
    journal: Arc<Mutex<LocalJournal>>,
    binding_lookup: BindingLookup,
}

impl PartialEq for RecoveryManager {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.journal, &other.journal)
            && std::ptr::fn_addr_eq(self.binding_lookup, other.binding_lookup)
    }
}

impl RecoveryManager {
    pub fn new(journal: Arc<Mutex<LocalJournal>>) -> Self {
        Self {
            journal,
            binding_lookup: mock_binding_lookup,
        }
    }

    pub fn with_binding_lookup(
        journal: Arc<Mutex<LocalJournal>>,
        lookup: BindingLookup,
    ) -> Self {
        Self {
            journal,
            binding_lookup: lookup,
        }
    }

    pub fn scan_journal(&self) -> Result<Vec<RecoveryEntry>, RecoveryError> {
        let records = self.journal()?.scan()?;
        let mut groups: BTreeMap<String, Vec<JournalRecord>> = BTreeMap::new();
        for record in records {
            groups
                .entry(record.command_id.0.clone())
                .or_default()
                .push(record);
            if groups.len() > MAX_SCAN_ENTRIES {
                return Err(RecoveryError::CapacityBoundExceeded(groups.len()));
            }
        }

        let mut entries = Vec::with_capacity(groups.len());
        for (command_id, records) in groups {
            entries.push(to_entry(CommandId(command_id), records)?);
        }
        Ok(entries)
    }

    pub fn reuse_prepared(
        &self,
        entry: &RecoveryEntry,
        invocation_digest: &RequestDigest,
    ) -> Result<RecoveryAction, RecoveryError> {
        let last = self.durable_last(entry)?;
        if last.state != JournalState::Prepared {
            return Ok(RecoveryAction::Abandon);
        }

        if last.request_digest == *invocation_digest {
            return Ok(RecoveryAction::Continue {
                ids: entry.reused_ids()?,
            });
        }

        if self.journal()?.is_prepared_eligible_for_prune(
            &entry.command_id,
            DEFAULT_PRUNE_MIN_RETENTION_SECONDS,
        ) {
            Ok(RecoveryAction::Prune)
        } else {
            Ok(RecoveryAction::Abandon)
        }
    }

    pub fn lookup_dispatching(
        &self,
        entry: &RecoveryEntry,
    ) -> Result<RecoveryAction, RecoveryError> {
        let last = self.durable_last(entry)?;
        if !matches!(last.state, JournalState::Dispatching | JournalState::Observed) {
            return Err(RecoveryError::NotDispatchable(entry.command_id.clone()));
        }

        let binding = BindingKey {
            command_id: entry.command_id.clone(),
            idempotency_key: last.idempotency_key.clone(),
        };
        match (self.binding_lookup)(&binding)? {
            Some(result) => {
                validate_bound_result(&last, &result)?;
                Ok(RecoveryAction::Resolved {
                    result: Box::new(result),
                })
            }
            None => Ok(RecoveryAction::Replay {
                ids: entry.reused_ids()?,
            }),
        }
    }

    pub fn replay_operation(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<OperationResult, RecoveryError> {
        let guard = self.journal()?;
        let records = guard
            .lookup(command_id)?
            .ok_or_else(|| RecoveryError::NotFound(command_id.clone()))?;
        let last = records
            .last()
            .ok_or_else(|| RecoveryError::NotFound(command_id.clone()))?;
        if last.idempotency_key != *key {
            return Err(RecoveryError::BindingConflict(command_id.clone()));
        }
        if !matches!(last.state, JournalState::Dispatching | JournalState::Observed) {
            return Err(RecoveryError::NotDispatchable(command_id.clone()));
        }
        Ok(continuation_result(
            command_id,
            &last.instance_id,
            &last.request_digest,
            "in-flight",
        ))
    }

    pub fn prune_stale_prepared(
        &mut self,
        policy: &PrunePolicy,
    ) -> Result<usize, RecoveryError> {
        Ok(self.journal()?.prune_prepared(policy)?)
    }

    /// Re-read the command immediately before making a recovery decision. A
    /// scan result is only a snapshot and must not authorize reuse after the
    /// durable state has advanced.
    fn durable_last(&self, entry: &RecoveryEntry) -> Result<JournalRecord, RecoveryError> {
        entry.validate_projection()?;
        let records = self
            .journal()?
            .lookup(&entry.command_id)?
            .ok_or_else(|| RecoveryError::NotFound(entry.command_id.clone()))?;
        if records != entry.records {
            return Err(RecoveryError::Corrupt(format!(
                "recovery entry for {} is stale; rescan required",
                entry.command_id.0
            )));
        }
        records.last().cloned().ok_or_else(|| {
            RecoveryError::Corrupt(format!(
                "durable journal for {} is empty",
                entry.command_id.0
            ))
        })
    }

    fn journal(&self) -> Result<MutexGuard<'_, LocalJournal>, RecoveryError> {
        self.journal
            .lock()
            .map_err(|_| RecoveryError::Corrupt("local journal mutex poisoned".to_owned()))
    }
}

fn validate_bound_result(
    journal: &JournalRecord,
    result: &OperationResult,
) -> Result<(), RecoveryError> {
    let digest_matches = result.receipt.resolved_binding_digest.is_empty()
        || result.receipt.resolved_binding_digest == journal.request_digest.0;
    if result.command_id != journal.command_id
        || result.instance_id != journal.instance_id
        || !digest_matches
    {
        return Err(RecoveryError::BindingConflict(journal.command_id.clone()));
    }
    Ok(())
}

fn to_entry(
    command_id: CommandId,
    records: Vec<JournalRecord>,
) -> Result<RecoveryEntry, RecoveryError> {
    let last = records.last().ok_or_else(|| {
        RecoveryError::Corrupt(format!("journal group {} is empty", command_id.0))
    })?;
    if last.command_id != command_id {
        return Err(RecoveryError::Corrupt(format!(
            "journal group {} contains mismatched command {}",
            command_id.0, last.command_id.0
        )));
    }
    Ok(RecoveryEntry {
        command_id,
        state: last.state,
        semantic_digest: last.request_digest.0.clone(),
        records,
    })
}

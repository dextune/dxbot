//! CLI journal recovery: bounded scan, prepared reuse, dispatching binding
//! lookup, replay projection, and stale-prepared pruning.
//!
//! The recovery projection (DXB-RUN-033) layers on top of [`LocalJournal`]:
//!
//! - **Prepared** (provably unsent): a same-semantic-digest entry can be
//!   reused to continue with the original IDs; a stale/different-digest entry
//!   can be pruned only when lock-free, integrity-valid and retention-eligible;
//!   otherwise it is abandoned.
//! - **Dispatching/Observed**: a server binding lookup runs first. A found
//!   binding exposes the existing committed result; an absent binding allows a
//!   same-ID/digest/payload replay. A corrupt/unknown/uncertain journal is
//!   never auto-replayed or auto-deleted.
//!
//! Recovery never turns a local observation end (SIGINT, SIGTERM, broken pipe,
//! pager exit, local timeout) into a Runtime cancel: the projected result of a
//! `Dispatching`/`Observed` entry carries `operation_may_continue = true` and
//! its `OperationRef` (operation id) so the user does not misread an interrupt
//! as a cancel.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use dxbot_core::types::*;
use dxbot_core::ReceiptRecord;

use crate::journal::{JournalError, LocalJournal, PrunePolicy};

/// Default session-wide ceiling for recovery entries surfaced by a single
/// [`RecoveryManager::scan_journal`].
const MAX_SCAN_ENTRIES: usize = 100_000;

/// Default minimum age (seconds) a stale `Prepared` record must have before a
/// reuse decision may prune it (mirrors `DXB-RUN-033` retention bounds).
const DEFAULT_PRUNE_MIN_RETENTION_SECONDS: u64 = 300;

/// Prefix used to derive a stable operation id for a command from its journal
/// (the journal itself does not persist an operation id).
const OPERATION_PREFIX: &str = "op-recovery:";

/// Error produced by the CLI recovery projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryError {
    /// A lower-level journal operation failed (I/O, lock, capacity).
    Journal(JournalError),
    /// Records could not be interpreted or a mandatory invariant is absent.
    Corrupt(String),
    /// The journal carries an unknown client/journal schema version; auto
    /// replay or delete is forbidden until a doctor/schema action resolves it.
    UnsupportedVersion(String),
    /// The journal hash chain or a record digest failed validation; auto
    /// replay or delete is forbidden.
    ChainMismatch(String),
    /// Replay/reuse was requested for an entry that is not in the right state.
    NotDispatchable(CommandId),
    /// No journal records exist for a command.
    NotFound(CommandId),
    /// The bounded startup scan exceeded its ceiling.
    CapacityBoundExceeded(usize),
}

impl From<JournalError> for RecoveryError {
    fn from(e: JournalError) -> Self {
        match e {
            JournalError::ChainIntegrity(msg) => Self::ChainMismatch(msg),
            JournalError::Corrupt(msg) => Self::Corrupt(msg),
            JournalError::CapacityBoundExceeded(n) => Self::CapacityBoundExceeded(n),
            JournalError::NotFound(id) => Self::NotFound(id),
            other => Self::Journal(other),
        }
    }
}

/// A single journal entry reconstructed from the local journal, grouped by
/// command and projected into a stable recovery shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEntry {
    /// The command this entry names.
    pub command_id: CommandId,
    /// The most advanced state recorded for this command.
    pub state: JournalState,
    /// Semantic digest of the most recent recorded request (materialized
    /// `RequestDigest`). Reuse is allowed only when this matches a fresh
    /// invocation's digest.
    pub semantic_digest: String,
    /// The full, chain-validated record series for this command.
    pub records: Vec<JournalRecord>,
}

impl RecoveryEntry {
    /// The idempotency key recorded for this entry (shared across its series).
    fn primary_key(&self) -> Result<IdempotencyKey, RecoveryError> {
        self.records
            .last()
            .map(|r| r.idempotency_key.clone())
            .ok_or_else(|| {
                RecoveryError::Corrupt(format!(
                    "entry {} has no journal records",
                    self.command_id.0
                ))
            })
    }

    /// The stable IDs a Continue/Replay reuse would adopt: the original command
    /// id, a derived operation id, and the recorded idempotency key.
    fn reused_ids(&self) -> Result<ReusedIds, RecoveryError> {
        Ok(ReusedIds {
            command_id: self.command_id.clone(),
            operation_id: OperationId(format!("{OPERATION_PREFIX}{}", self.command_id.0)),
            idempotency_key: self.primary_key()?,
        })
    }
}

/// The IDs reused when a journal entry is continued or replayed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusedIds {
    pub command_id: CommandId,
    pub operation_id: OperationId,
    pub idempotency_key: IdempotencyKey,
}

/// What a recovery decision should do with a journal entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Reuse the original IDs and continue (same semantic digest).
    Continue { ids: ReusedIds },
    /// The stale prepared entry is eligible to be removed by the capacity
    /// policy.
    Prune,
    /// The entry cannot be reused, replayed or pruned: abandon it.
    Abandon,
    /// A server binding lookup found a committed result; expose it.
    Resolved { result: Box<OperationResult> },
    /// Both server bindings are absent; replay with the same IDs.
    Replay { ids: ReusedIds },
}

/// The composite key passed to the injected server binding lookup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingKey {
    pub command_id: CommandId,
    pub idempotency_key: IdempotencyKey,
}

/// Server binding lookup: maps a command/key onto a possibly committed
/// [`OperationResult`]. Injected so the CLI performs no real network I/O in
/// this module. The default mock reports an absent binding.
pub type BindingLookup = fn(&BindingKey) -> Result<Option<OperationResult>, RecoveryError>;

/// Default server binding lookup: reports no committed result. With both
/// bindings absent, recovery may replay with the same IDs.
fn mock_binding_lookup(_key: &BindingKey) -> Result<Option<OperationResult>, RecoveryError> {
    Ok(None)
}

/// Creates an [`OperationResult`] projecting a continued operation for
/// `command_id`. Used for Dispatching/Observed entries so a local observation
/// end is never misread as a Runtime cancel: the result carries
/// `operation_may_continue = true` and its `OperationRef` (operation id).
fn continuation_result(
    command_id: &CommandId,
    instance_id: &InstanceId,
    _key: &IdempotencyKey,
    status: &str,
    may_continue: bool,
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
            resolved_binding_digest: String::new(),
            owner_kind: String::new(),
            lease_until: None,
            last_progress: 0,
            reconciliation_policy: String::new(),
        },
        status: status.to_string(),
        committed_payload: None,
        error: None,
        operation_may_continue: may_continue,
    }
}

/// Recovery decision/projection entry point over a shared [`LocalJournal`].
#[derive(Debug, Clone)]
pub struct RecoveryManager {
    journal: Arc<Mutex<LocalJournal>>,
    binding_lookup: BindingLookup,
}

impl PartialEq for RecoveryManager {
    fn eq(&self, other: &Self) -> bool {
        // The journal is compared by shared ownership (its contents are not
        // `PartialEq`-comparable through the mutex); the binding lookup is a
        // `fn` pointer and compares by address.
        Arc::ptr_eq(&self.journal, &other.journal)
            && std::ptr::fn_addr_eq(self.binding_lookup, other.binding_lookup)
    }
}

impl RecoveryManager {
    /// Creates a recovery view over `journal` with the default (absent)
    /// server binding lookup.
    pub fn new(journal: Arc<Mutex<LocalJournal>>) -> Self {
        Self {
            journal,
            binding_lookup: mock_binding_lookup,
        }
    }

    /// Creates a recovery view with an injected server binding lookup (the
    /// hook for a mock or an authenticated binding producer; no real network
    /// occurs in this module).
    pub fn with_binding_lookup(
        journal: Arc<Mutex<LocalJournal>>,
        lookup: BindingLookup,
    ) -> Self {
        Self {
            journal,
            binding_lookup: lookup,
        }
    }

    /// Bounded scan of the journal, grouped into [`RecoveryEntry`] values.
    /// The underlying `LocalJournal::scan` is itself bounded (capacity-exceeded
    /// fails closed); this additionally applies a session-wide ceiling on the
    /// number of entries surfaced to a caller.
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
        for (command_id, series) in groups {
            entries.push(to_entry(CommandId(command_id), series));
        }
        Ok(entries)
    }

    /// Decides whether a `Prepared` entry can be reused by a new invocation.
    ///
    /// - Same semantic digest → reuse the original IDs and continue.
    /// - Stale/different digest → prune only when the entry is lock-free,
    ///   integrity-valid and retention-eligible; otherwise abandon.
    pub fn reuse_prepared(
        &self,
        entry: &RecoveryEntry,
    ) -> Result<RecoveryAction, RecoveryError> {
        let last = entry.records.last().ok_or_else(|| {
            RecoveryError::Corrupt(format!("entry {} has no journal records", entry.command_id.0))
        })?;
        if last.state != JournalState::Prepared {
            return Ok(RecoveryAction::Abandon);
        }
        let canonical_digest = last.request_digest.0.clone();
        if entry.semantic_digest == canonical_digest {
            // Same semantic digest: reuse the original IDs to continue.
            return Ok(RecoveryAction::Continue {
                ids: entry.reused_ids()?,
            });
        }
        // Stale/different digest: prune only if provably-unsent, lock-free,
        // integrity-valid and retention-eligible.
        let eligible = self.journal()?.is_prepared_eligible_for_prune(
            &entry.command_id,
            DEFAULT_PRUNE_MIN_RETENTION_SECONDS,
        );
        if eligible {
            Ok(RecoveryAction::Prune)
        } else {
            Ok(RecoveryAction::Abandon)
        }
    }

    /// Decides what to do with a `Dispatching`/`Observed` entry. Performs the
    /// server binding lookup first; never auto-replays when a binding exists
    /// or when the journal is uncertain/corrupt.
    pub fn lookup_dispatching(
        &self,
        entry: &RecoveryEntry,
    ) -> Result<RecoveryAction, RecoveryError> {
        if entry.state != JournalState::Dispatching
            && entry.state != JournalState::Observed
        {
            return Err(RecoveryError::NotDispatchable(entry.command_id.clone()));
        }
        let key = entry.primary_key()?;
        let binding = BindingKey {
            command_id: entry.command_id.clone(),
            idempotency_key: key.clone(),
        };
        match (self.binding_lookup)(&binding)? {
            // A stored binding exists: expose the committed result directly.
            Some(result) => Ok(RecoveryAction::Resolved { result: Box::new(result) }),
            // Both bindings absent: replay with the same IDs/digest/payload.
            None => Ok(RecoveryAction::Replay {
                ids: entry.reused_ids()?,
            }),
        }
    }

    /// Replays an operation from the journal for a `Dispatching`/`Observed`
    /// command, projecting the continued operation. Corrupt/unknown records
    /// fail chain validation before this point, so no auto replay/delete is
    /// possible from this path.
    pub fn replay_operation(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<OperationResult, RecoveryError> {
        let guard = self.journal()?;
        let records = match guard.lookup(command_id)? {
            Some(records) => records,
            None => return Err(RecoveryError::NotFound(command_id.clone())),
        };
        let last = records
            .last()
            .ok_or_else(|| RecoveryError::NotFound(command_id.clone()))?;
        let instance_id = last.instance_id.clone();
        let state = last.state;
        drop(guard);

        let (status, may_continue) = match state {
            JournalState::Dispatching | JournalState::Observed => ("in-flight", true),
            JournalState::Terminal => ("terminal", false),
            JournalState::Prepared | JournalState::Abandoned => {
                return Err(RecoveryError::NotDispatchable(command_id.clone()));
            }
        };
        Ok(continuation_result(
            command_id,
            &instance_id,
            key,
            status,
            may_continue,
        ))
    }

    /// Prunes stale, eligible `Prepared` commands under `policy` by delegating
    /// to the journal's bounded capacity policy. Returns the number pruned.
    pub fn prune_stale_prepared(
        &mut self,
        policy: &PrunePolicy,
    ) -> Result<usize, RecoveryError> {
        Ok(self.journal()?.prune_prepared(policy)?)
    }

    /// Locks the shared journal, recovering from a poisoned lock by taking its
    /// inner value: the state recorded before any panic is kept, never treated
    /// as loss.
    fn journal(&self) -> Result<MutexGuard<'_, LocalJournal>, RecoveryError> {
        Ok(self
            .journal
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()))
    }
}

/// Groups a validated record series into a recovery entry, taking the entry's
/// state and semantic digest from the most advanced record in the series.
fn to_entry(command_id: CommandId, records: Vec<JournalRecord>) -> RecoveryEntry {
    let last = records.last();
    let state = last.map(|r| r.state).unwrap_or(JournalState::Prepared);
    let semantic_digest = last.map(|r| r.request_digest.0.clone()).unwrap_or_default();
    RecoveryEntry {
        command_id,
        state,
        semantic_digest,
        records,
    }
}
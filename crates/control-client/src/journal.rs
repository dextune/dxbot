//! In-memory journal for the durable submission protocol.
//!
//! The [`JournalStore`] holds the ordered [`JournalRecord`] sequence for a
//! submission client instance. It models the durable state machine that a
//! real control client persists per-operation:
//!
//! `Prepared -> Dispatching -> Observed -> Terminal`
//!
//! Because the store is purely in-memory it is cheap to construct inside
//! tests, which lets the submission crash fixture inspect exactly which
//! records survived a crash at each [`CrashPoint`](crate::CrashPoint)
//! boundary without any filesystem dependency.

use std::collections::HashMap;

use dxbot_core::types::{CommandId, IdempotencyKey, JournalRecord, JournalState, OperationResult};

use crate::client::ClientError;

/// One persisted journal entry: a record plus its resolved terminal result.
///
/// The `result` is `None` until the operation has been observed/terminated,
/// which is how a crash after commit (but before response) is represented: a
/// committed binding with no resolved result yet.
#[derive(Debug, Clone, PartialEq)]
struct StoredEntry {
    record: JournalRecord,
    result: Option<OperationResult>,
}

/// An in-memory, sequence-ordered journal for one submission client instance.
#[derive(Debug, Default)]
pub struct JournalStore {
    entries: HashMap<CommandId, StoredEntry>,
    next_sequence: i64,
    previous_digest: String,
}

impl JournalStore {
    /// Creates an empty journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a `Prepared` record, assigning its sequence and digest.
    ///
    /// This is the durable write that happens *before* any dispatch or send;
    /// it defines the "commit" boundary of the submission protocol.
    pub fn prepare(&mut self, mut record: JournalRecord) -> Result<(), ClientError> {
        if self.entries.contains_key(&record.command_id) {
            return Err(ClientError::AlreadyExists(record.command_id));
        }
        record.state = JournalState::Prepared;
        record.sequence = self.next_sequence;
        self.next_sequence += 1;
        record.previous_digest = self.previous_digest.clone();
        record.record_digest = digest_of(&record);
        self.previous_digest = record.record_digest.clone();
        self.entries.insert(
            record.command_id.clone(),
            StoredEntry {
                record,
                result: None,
            },
        );
        Ok(())
    }

    /// Advances an existing record to `Dispatching` (fsync-before-send).
    pub fn dispatch(&mut self, command_id: &CommandId) -> Result<(), ClientError> {
        let entry = self
            .entries
            .get_mut(command_id)
            .ok_or_else(|| ClientError::NotCommitted(command_id.clone()))?;
        entry.record.state = JournalState::Dispatching;
        entry.record.record_digest = digest_of(&entry.record);
        Ok(())
    }

    /// Marks an existing record `Observed` and attaches the received result.
    pub fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        let entry = self
            .entries
            .get_mut(command_id)
            .ok_or_else(|| ClientError::NotCommitted(command_id.clone()))?;
        entry.record.state = JournalState::Observed;
        entry.record.record_digest = digest_of(&entry.record);
        entry.result = Some(result.clone());
        Ok(())
    }

    /// Marks an existing record `Terminal` and re-attaches the final result.
    pub fn terminate(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        let entry = self
            .entries
            .get_mut(command_id)
            .ok_or_else(|| ClientError::NotCommitted(command_id.clone()))?;
        entry.record.state = JournalState::Terminal;
        entry.record.record_digest = digest_of(&entry.record);
        entry.result = Some(result.clone());
        Ok(())
    }

    /// Returns the journal record for a command, if it has been committed.
    pub fn lookup(&self, command_id: &CommandId) -> Option<JournalRecord> {
        self.entries.get(command_id).map(|entry| entry.record.clone())
    }

    /// Returns the record and resolved result for a committed command.
    pub fn lookup_entry(
        &self,
        command_id: &CommandId,
    ) -> Option<(JournalRecord, Option<OperationResult>)> {
        self.entries
            .get(command_id)
            .map(|entry| (entry.record.clone(), entry.result.clone()))
    }

    /// Finds a committed binding matching both the command id and idempotency key.
    pub fn find_binding(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Option<(JournalRecord, Option<OperationResult>)> {
        self.entries.get(command_id).and_then(|entry| {
            if entry.record.idempotency_key == *key {
                Some((entry.record.clone(), entry.result.clone()))
            } else {
                None
            }
        })
    }

    /// Number of committed journal entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the journal has no committed entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Deterministic 64-bit FNV-1a over the stable, non-digest fields of a record.
fn digest_of(record: &JournalRecord) -> String {
    let canonical = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        state_str(&record.state),
        record.instance_id.0,
        record.command_id.0,
        record.idempotency_key.principal_ref.0,
        record.idempotency_key.key_digest,
        record.idempotency_key.expires_at,
        record.request_digest.0,
        record.sequence,
    );
    format!("{:016x}", fnv1a(canonical.as_bytes()))
}

fn state_str(state: &JournalState) -> &'static str {
    match state {
        JournalState::Prepared => "prepared",
        JournalState::Dispatching => "dispatching",
        JournalState::Observed => "observed",
        JournalState::Terminal => "terminal",
        JournalState::Abandoned => "abandoned",
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
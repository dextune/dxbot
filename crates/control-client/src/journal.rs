//! In-memory journal projection for the durable submission protocol.
//!
//! The fixture preserves the same replay identity as the file journal:
//! InstanceId + CommandId + OperationId + IdempotencyKey + RequestDigest.

use std::collections::HashMap;

use dxbot_core::types::{CommandId, IdempotencyKey, JournalRecord, JournalState, OperationResult};

use crate::client::ClientError;

#[derive(Debug, Clone, PartialEq)]
struct StoredEntry {
    record: JournalRecord,
    result: Option<OperationResult>,
}

#[derive(Debug)]
pub struct JournalStore {
    entries: HashMap<CommandId, StoredEntry>,
    next_sequence: i64,
}

impl Default for JournalStore {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_sequence: 1,
        }
    }
}

impl JournalStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn prepare(&mut self, mut record: JournalRecord) -> Result<(), ClientError> {
        if self.entries.contains_key(&record.command_id) {
            return Err(ClientError::AlreadyExists(record.command_id));
        }
        record.state = JournalState::Prepared;
        record.sequence = self.next_sequence;
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| ClientError::Storage("journal sequence exhausted".to_owned()))?;
        record.previous_digest.clear();
        record.record_digest = digest_of(&record);
        self.entries.insert(
            record.command_id.clone(),
            StoredEntry {
                record,
                result: None,
            },
        );
        Ok(())
    }

    pub fn dispatch(&mut self, command_id: &CommandId) -> Result<(), ClientError> {
        let entry = self
            .entries
            .get_mut(command_id)
            .ok_or_else(|| ClientError::NotCommitted(command_id.clone()))?;
        if entry.record.state != JournalState::Prepared {
            return Err(ClientError::Storage(format!(
                "cannot dispatch command {} from {:?}",
                command_id.0, entry.record.state
            )));
        }
        entry.record.state = JournalState::Dispatching;
        entry.record.record_digest = digest_of(&entry.record);
        Ok(())
    }

    pub fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        let entry = self
            .entries
            .get_mut(command_id)
            .ok_or_else(|| ClientError::NotCommitted(command_id.clone()))?;
        if entry.record.state != JournalState::Dispatching
            || result.command_id != entry.record.command_id
            || result.operation_id != entry.record.operation_id
            || result.instance_id != entry.record.instance_id
        {
            return Err(ClientError::Storage(format!(
                "observed result does not match dispatch identity for {}",
                command_id.0
            )));
        }
        entry.record.state = JournalState::Observed;
        entry.record.record_digest = digest_of(&entry.record);
        entry.result = Some(result.clone());
        Ok(())
    }

    pub fn terminate(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        let entry = self
            .entries
            .get_mut(command_id)
            .ok_or_else(|| ClientError::NotCommitted(command_id.clone()))?;
        if entry.record.state != JournalState::Observed
            || result.command_id != entry.record.command_id
            || result.operation_id != entry.record.operation_id
            || result.instance_id != entry.record.instance_id
        {
            return Err(ClientError::Storage(format!(
                "terminal result does not match observed identity for {}",
                command_id.0
            )));
        }
        entry.record.state = JournalState::Terminal;
        entry.record.record_digest = digest_of(&entry.record);
        entry.result = Some(result.clone());
        Ok(())
    }

    pub fn lookup(&self, command_id: &CommandId) -> Option<JournalRecord> {
        self.entries.get(command_id).map(|entry| entry.record.clone())
    }

    pub fn lookup_entry(
        &self,
        command_id: &CommandId,
    ) -> Option<(JournalRecord, Option<OperationResult>)> {
        self.entries
            .get(command_id)
            .map(|entry| (entry.record.clone(), entry.result.clone()))
    }

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

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn digest_of(record: &JournalRecord) -> String {
    let canonical = format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        state_str(&record.state),
        record.instance_id.0,
        record.command_id.0,
        record.operation_id.0,
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
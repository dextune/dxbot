//! Submission journal backend contract.
//!
//! `control-client` owns replay/submission state-machine semantics but does not
//! own the production storage implementation. The in-memory [`JournalStore`]
//! remains a deterministic fixture while the CLI can inject its file-backed
//! journal without duplicating the submission protocol.

use std::fmt::Debug;

use dxbot_core::types::{
    CommandId, IdempotencyKey, JournalRecord, OperationResult,
};

use crate::client::ClientError;
use crate::journal::JournalStore;

/// Minimal storage contract required by [`crate::SubmissionClient`].
///
/// Implementations must preserve the immutable replay identity carried by the
/// first Prepared record and must fail closed on corrupt or invalid state.
pub trait SubmissionJournal: Debug {
    fn prepare(&mut self, record: JournalRecord) -> Result<(), ClientError>;
    fn dispatch(&mut self, command_id: &CommandId) -> Result<(), ClientError>;
    fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError>;
    fn terminate(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError>;
    fn lookup(&self, command_id: &CommandId) -> Option<JournalRecord>;
    fn find_binding(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Option<(JournalRecord, Option<OperationResult>)>;
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl SubmissionJournal for JournalStore {
    fn prepare(&mut self, record: JournalRecord) -> Result<(), ClientError> {
        JournalStore::prepare(self, record)
    }

    fn dispatch(&mut self, command_id: &CommandId) -> Result<(), ClientError> {
        JournalStore::dispatch(self, command_id)
    }

    fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        JournalStore::observe(self, command_id, result)
    }

    fn terminate(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        JournalStore::terminate(self, command_id, result)
    }

    fn lookup(&self, command_id: &CommandId) -> Option<JournalRecord> {
        JournalStore::lookup(self, command_id)
    }

    fn find_binding(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Option<(JournalRecord, Option<OperationResult>)> {
        JournalStore::find_binding(self, command_id, key)
    }

    fn len(&self) -> usize {
        JournalStore::len(self)
    }
}

//! Production adapter from the CLI-owned file journal to the control-client
//! submission state machine.
//!
//! This keeps one durable owner for local Prepared/Dispatching/Observed/Terminal
//! state while `control-client` continues to own replay identity and ordering.

use std::collections::HashSet;

use control_client::{ClientError, SubmissionJournal};
use dxbot_core::types::{CommandId, IdempotencyKey, JournalRecord, OperationResult};

use crate::journal::{JournalError, LocalJournal};

impl SubmissionJournal for LocalJournal {
    fn prepare(&mut self, record: JournalRecord) -> Result<(), ClientError> {
        self.append_prepared(&record).map_err(map_error)
    }

    fn dispatch(&mut self, command_id: &CommandId) -> Result<(), ClientError> {
        LocalJournal::dispatch(self, command_id).map_err(map_error)
    }

    fn observe(
        &mut self,
        command_id: &CommandId,
        result: &OperationResult,
    ) -> Result<(), ClientError> {
        LocalJournal::observe(self, command_id, result).map_err(map_error)
    }

    fn terminate(
        &mut self,
        command_id: &CommandId,
        _result: &OperationResult,
    ) -> Result<(), ClientError> {
        LocalJournal::terminal(self, command_id).map_err(map_error)
    }

    fn lookup(&self, command_id: &CommandId) -> Result<Option<JournalRecord>, ClientError> {
        LocalJournal::lookup(self, command_id)
            .map(|records| records.and_then(|records| records.last().cloned()))
            .map_err(map_error)
    }

    fn find_binding(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<Option<(JournalRecord, Option<OperationResult>)>, ClientError> {
        let Some(records) = LocalJournal::lookup(self, command_id).map_err(map_error)? else {
            return Ok(None);
        };
        let Some(first) = records.first() else {
            return Err(ClientError::Storage(format!(
                "journal for command {} has no complete records",
                command_id.0
            )));
        };
        let Some(last) = records.last() else {
            return Err(ClientError::Storage(format!(
                "journal for command {} has no terminal record",
                command_id.0
            )));
        };
        if first.idempotency_key != *key {
            return Ok(None);
        }
        Ok(Some((last.clone(), None)))
    }

    fn len(&self) -> usize {
        LocalJournal::scan(self)
            .map(|records| {
                records
                    .into_iter()
                    .map(|record| record.command_id)
                    .collect::<HashSet<_>>()
                    .len()
            })
            .unwrap_or_default()
    }
}

fn map_error(error: JournalError) -> ClientError {
    match error {
        JournalError::AlreadyExists(command_id) => ClientError::AlreadyExists(command_id),
        JournalError::NotFound(command_id) => ClientError::NotCommitted(command_id),
        other => ClientError::Storage(format!("local journal: {other:?}")),
    }
}

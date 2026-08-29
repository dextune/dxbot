//! Durable CLI submission and recovery convergence.
//!
//! A fresh invocation may adopt exactly one non-terminal journal entry only when
//! the freshly materialized RequestDigest, Instance and authenticated Principal
//! all match. Terminal history never absorbs a new invocation. Dispatching or
//! Observed always performs canonical server binding lookup before any replay.

#![cfg(unix)]

use std::collections::BTreeMap;
use std::path::Path;

use application_contract::CliInput;
use control_client::{ClientError, LocalControlClient, SubmissionClient};
use dxbot_core::DxbotError;
use dxbot_core::types::{CommandId, CommandPayload, JournalRecord, JournalState, OperationResult};

use crate::identity::{build_operation_request, build_replay_request, request_digest_for_input};
use crate::journal::{JournalError, LocalJournal};

const MAX_RECOVERY_CANDIDATES: usize = 2;

#[derive(Debug)]
pub enum SubmissionFlowError {
    Contract(DxbotError),
    Journal(JournalError),
    Client(ClientError),
    AmbiguousRecovery(Vec<CommandId>),
    PreparedAlreadyBound(CommandId),
    ObservedBindingMissing(CommandId),
}

impl From<DxbotError> for SubmissionFlowError {
    fn from(error: DxbotError) -> Self {
        Self::Contract(error)
    }
}

impl From<JournalError> for SubmissionFlowError {
    fn from(error: JournalError) -> Self {
        Self::Journal(error)
    }
}

impl From<ClientError> for SubmissionFlowError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

pub fn submit_or_recover(
    input: &CliInput,
    payload: CommandPayload,
    journal_root: &Path,
    local_client: LocalControlClient,
) -> Result<OperationResult, SubmissionFlowError> {
    let digest = request_digest_for_input(input, &payload)?;
    let instance_id = payload.instance_id.clone();
    let principal = payload.principal_ref.clone();

    let scan_journal = LocalJournal::open(instance_id.clone(), journal_root)?;
    let records = scan_journal.scan()?;
    drop(scan_journal);
    let candidates = recovery_candidates(records, &digest.0, &principal.0);
    if candidates.len() > 1 {
        return Err(SubmissionFlowError::AmbiguousRecovery(
            candidates.iter().map(|record| record.command_id.clone()).collect(),
        ));
    }

    if let Some(record) = candidates.into_iter().next() {
        return recover_existing(input, payload, journal_root, local_client, record);
    }

    let request = build_operation_request(input, payload)?;
    let journal = LocalJournal::open(instance_id.clone(), journal_root)?;
    let client = SubmissionClient::builder(instance_id)
        .with_journal(journal)
        .with_fallible_transport(local_client.into_transport())
        .build();
    client.submit(&request).map_err(SubmissionFlowError::Client)
}

fn recover_existing(
    input: &CliInput,
    payload: CommandPayload,
    journal_root: &Path,
    local_client: LocalControlClient,
    record: JournalRecord,
) -> Result<OperationResult, SubmissionFlowError> {
    let remote = match record.state {
        JournalState::Prepared => None,
        JournalState::Dispatching | JournalState::Observed => local_client
            .lookup_binding(&record.command_id, &record.idempotency_key)?,
        JournalState::Terminal | JournalState::Abandoned => None,
    };

    if record.state == JournalState::Prepared && remote.is_some() {
        return Err(SubmissionFlowError::PreparedAlreadyBound(
            record.command_id,
        ));
    }
    if record.state == JournalState::Observed && remote.is_none() {
        return Err(SubmissionFlowError::ObservedBindingMissing(
            record.command_id,
        ));
    }

    let request = build_replay_request(input, payload, &record)?;
    let mut journal = LocalJournal::open(record.instance_id.clone(), journal_root)?;
    journal.takeover(&record.command_id)?;
    let client = SubmissionClient::builder(record.instance_id.clone())
        .with_journal(journal)
        .with_fallible_transport(local_client.into_transport())
        .build();

    if let Some(result) = remote {
        return client
            .finalize_recovered_result(&request, &result)
            .map_err(SubmissionFlowError::Client);
    }
    client
        .replay_operation(&request)
        .map_err(SubmissionFlowError::Client)
}

fn recovery_candidates(
    records: Vec<JournalRecord>,
    digest: &str,
    principal: &str,
) -> Vec<JournalRecord> {
    let mut latest = BTreeMap::<String, JournalRecord>::new();
    for record in records {
        latest.insert(record.command_id.0.clone(), record);
    }
    let mut candidates = Vec::new();
    for record in latest.into_values() {
        if record.request_digest.0 != digest
            || record.idempotency_key.principal_ref.0 != principal
            || matches!(record.state, JournalState::Terminal | JournalState::Abandoned)
        {
            continue;
        }
        candidates.push(record);
        if candidates.len() >= MAX_RECOVERY_CANDIDATES {
            break;
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use dxbot_core::types::{
        IdempotencyKey, InstanceId, OperationId, PrincipalRef, RequestDigest,
    };

    use super::*;

    fn record(command: &str, state: JournalState, digest: &str) -> JournalRecord {
        JournalRecord {
            state,
            instance_id: InstanceId("instance-a".to_owned()),
            command_id: CommandId(command.to_owned()),
            operation_id: OperationId(format!("operation-{command}")),
            idempotency_key: IdempotencyKey {
                principal_ref: PrincipalRef("principal-a".to_owned()),
                key_digest: format!("key-{command}"),
                expires_at: 10,
            },
            request_digest: RequestDigest(digest.to_owned()),
            sequence: 1,
            previous_digest: String::new(),
            record_digest: String::new(),
        }
    }

    #[test]
    fn terminal_history_never_absorbs_fresh_invocation() {
        let candidates = recovery_candidates(
            vec![record("old", JournalState::Terminal, "same")],
            "same",
            "principal-a",
        );
        assert!(candidates.is_empty());
    }

    #[test]
    fn only_matching_non_terminal_identity_is_recoverable() {
        let candidates = recovery_candidates(
            vec![
                record("a", JournalState::Dispatching, "same"),
                record("b", JournalState::Prepared, "other"),
            ],
            "same",
            "principal-a",
        );
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].command_id, CommandId("a".to_owned()));
    }
}

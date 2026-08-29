//! High-risk operation approvals and pending operation continuation.
//!
//! Approval state is canonical in runtime-security. Each decision advances the
//! approval revision so CLI CAS can fence stale approve/deny attempts.

use std::collections::HashMap;

use dxbot_core::types::{ApprovalId, OperationId, PrincipalRef};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    Approve,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    Pending,
    Approved,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalDecisionRecord {
    pub by: PrincipalRef,
    pub decision: ApprovalDecision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    pub id: ApprovalId,
    pub operation_id: OperationId,
    pub required_approvers: Vec<PrincipalRef>,
    pub decisions: Vec<ApprovalDecisionRecord>,
    pub state: ApprovalState,
    pub revision: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ApprovalManager {
    approvals: HashMap<String, ApprovalRecord>,
    next_seq: u64,
}

impl ApprovalManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_approval(
        &mut self,
        operation_id: OperationId,
        required_approvers: Vec<PrincipalRef>,
    ) -> Result<ApprovalId, Error> {
        let sequence = self.next_seq;
        self.next_seq = self.next_seq.checked_add(1).ok_or_else(|| {
            Error::ApprovalAlreadyDecided(ApprovalId("approval-id-space-exhausted".to_owned()))
        })?;
        let id = ApprovalId(format!("approval-{sequence}"));
        self.approvals.insert(
            id.0.clone(),
            ApprovalRecord {
                id: id.clone(),
                operation_id,
                required_approvers,
                decisions: Vec::new(),
                state: ApprovalState::Pending,
                revision: 1,
            },
        );
        Ok(id)
    }

    /// Compatibility API for internal callers that already hold the current
    /// record. Public CLI decisions should use `decide_approval_if_revision`.
    pub fn decide_approval(
        &mut self,
        approval_id: &ApprovalId,
        decision: ApprovalDecision,
        by: &PrincipalRef,
    ) -> Result<ApprovalState, Error> {
        let expected_revision = self.get_approval(approval_id)?.revision;
        self.decide_approval_if_revision(approval_id, expected_revision, decision, by)
            .map(|record| record.state)
    }

    pub fn decide_approval_if_revision(
        &mut self,
        approval_id: &ApprovalId,
        expected_revision: i64,
        decision: ApprovalDecision,
        by: &PrincipalRef,
    ) -> Result<ApprovalRecord, Error> {
        let record = self
            .approvals
            .get_mut(&approval_id.0)
            .ok_or_else(|| Error::UnknownApproval(approval_id.clone()))?;
        if record.revision != expected_revision || record.state != ApprovalState::Pending {
            return Err(Error::ApprovalAlreadyDecided(approval_id.clone()));
        }
        if !record.required_approvers.contains(by) {
            return Err(Error::InvalidApprover(by.clone()));
        }
        if record.decisions.iter().any(|existing| existing.by == *by) {
            return Err(Error::ApprovalAlreadyDecided(approval_id.clone()));
        }

        record.decisions.push(ApprovalDecisionRecord {
            by: by.clone(),
            decision,
        });
        match decision {
            ApprovalDecision::Deny => record.state = ApprovalState::Denied,
            ApprovalDecision::Approve => {
                if record.required_approvers.iter().all(|required| {
                    record.decisions.iter().any(|record| {
                        record.by == *required && record.decision == ApprovalDecision::Approve
                    })
                }) {
                    record.state = ApprovalState::Approved;
                }
            }
        }
        record.revision = record
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::ApprovalAlreadyDecided(approval_id.clone()))?;
        Ok(record.clone())
    }

    pub fn get_approval(&self, id: &ApprovalId) -> Result<ApprovalRecord, Error> {
        self.approvals
            .get(&id.0)
            .cloned()
            .ok_or_else(|| Error::UnknownApproval(id.clone()))
    }

    pub fn find_by_operation(&self, operation_id: &OperationId) -> Result<ApprovalRecord, Error> {
        self.approvals
            .values()
            .find(|record| record.operation_id == *operation_id)
            .cloned()
            .ok_or_else(|| Error::UnknownApproval(ApprovalId(operation_id.0.clone())))
    }

    pub fn list_approvals(&self) -> Vec<ApprovalRecord> {
        let mut records = self.approvals.values().cloned().collect::<Vec<_>>();
        records.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        records
    }

    pub fn continuation_ready(
        &self,
        approval_id: &ApprovalId,
    ) -> Result<Option<OperationId>, Error> {
        let record = self
            .approvals
            .get(&approval_id.0)
            .ok_or_else(|| Error::UnknownApproval(approval_id.clone()))?;
        Ok(match record.state {
            ApprovalState::Approved => Some(record.operation_id.clone()),
            ApprovalState::Pending | ApprovalState::Denied => None,
        })
    }
}

//! High-risk operation approvals and pending operation continuation.
//!
//! An approval is bound to a pending [`OperationId`]. Required approvers cast
//! decisions; the aggregate transitions `Pending -> Approved | Denied`. Once an
//! approval is `Approved`, the pending operation it guards may continue
//! ([`ApprovalManager::continuation_ready`]).

use std::collections::HashMap;

use dxbot_core::types::{ApprovalId, OperationId, PrincipalRef};

use crate::Error;

/// An approver's decision on an approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Approve the guarded operation so it may continue.
    Approve,
    /// Deny the guarded operation; it must not continue.
    Deny,
}

/// Aggregate state of an approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    /// Awaiting required approver decisions.
    Pending,
    /// All required approvers approved; the pending operation may continue.
    Approved,
    /// At least one required approver denied; the operation must not continue.
    Denied,
}

/// A single recorded approver decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalDecisionRecord {
    /// The principal that cast the decision.
    pub by: PrincipalRef,
    /// The decision that was cast.
    pub decision: ApprovalDecision,
}

/// Full record of an approval and its decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRecord {
    /// Canonical approval id.
    pub id: ApprovalId,
    /// The pending operation this approval guards.
    pub operation_id: OperationId,
    /// The required approvers who must approve for `Approved`.
    pub required_approvers: Vec<PrincipalRef>,
    /// Decisions cast so far, in order.
    pub decisions: Vec<ApprovalDecisionRecord>,
    /// Current aggregate state.
    pub state: ApprovalState,
}

/// In-memory store of approvals.
#[derive(Debug, Clone, Default)]
pub struct ApprovalManager {
    approvals: HashMap<String, ApprovalRecord>,
    next_seq: u64,
}

impl ApprovalManager {
    /// Create an empty approval store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new pending approval for `operation_id` requiring
    /// `required_approvers` to approve.
    pub fn create_approval(
        &mut self,
        operation_id: OperationId,
        required_approvers: Vec<PrincipalRef>,
    ) -> Result<ApprovalId, Error> {
        let id = ApprovalId(format!("approval-{}", self.next_seq));
        self.next_seq += 1;
        self.approvals.insert(
            id.0.clone(),
            ApprovalRecord {
                id: id.clone(),
                operation_id,
                required_approvers,
                decisions: Vec::new(),
                state: ApprovalState::Pending,
            },
        );
        Ok(id)
    }

    /// Record a decision by `by` on `approval_id`.
    ///
    /// Returns the resulting aggregate [`ApprovalState`]. Denying once denies
    /// the approval; approving every required approver approves it.
    pub fn decide_approval(
        &mut self,
        approval_id: &ApprovalId,
        decision: ApprovalDecision,
        by: &PrincipalRef,
    ) -> Result<ApprovalState, Error> {
        let record = self
            .approvals
            .get_mut(&approval_id.0)
            .ok_or_else(|| Error::UnknownApproval(approval_id.clone()))?;

        if record.state != ApprovalState::Pending {
            return Err(Error::ApprovalAlreadyDecided(approval_id.clone()));
        }
        if !record.required_approvers.contains(by) {
            return Err(Error::InvalidApprover(by.clone()));
        }

        record
            .decisions
            .push(ApprovalDecisionRecord {
                by: by.clone(),
                decision,
            });

        match decision {
            ApprovalDecision::Deny => record.state = ApprovalState::Denied,
            ApprovalDecision::Approve => {
                if record.required_approvers.iter().all(|required| {
                    record.decisions.iter().any(|d| {
                        d.by == *required && d.decision == ApprovalDecision::Approve
                    })
                }) {
                    record.state = ApprovalState::Approved;
                }
            }
        }

        Ok(record.state)
    }

    /// Retrieve an approval record.
    pub fn get_approval(&self, id: &ApprovalId) -> Result<ApprovalRecord, Error> {
        self.approvals
            .get(&id.0)
            .cloned()
            .ok_or_else(|| Error::UnknownApproval(id.clone()))
    }

    /// Gate for pending operation continuation.
    ///
    /// Returns `Ok(Some(operation_id))` once the approval is `Approved` (the
    /// guarded pending operation may continue), and `Ok(None)` while it remains
    /// `Pending` or is `Denied` (it must not continue). Unknown approvals are an
    /// error.
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
//! Canonical parked high-risk operation gating facts.
//!
//! When a high-risk operation is evaluated server-side it is *parked* behind a
//! bound Approval before any Application mutation. This module owns the security
//! side of that gate: the durable binding between the original [`OperationId`],
//! the [`ApprovalId`] that must clear, the exact request-identity digest, the
//! target CAS/policy generation the gate was created against, and the terminal
//! disposition (continued after approval, or denied and never continued).
//!
//! This is a security gating fact only. It intentionally does not hold Domain
//! rows: the replayable request payload is owned by the control-plane
//! coordinator. Runtime-security records *what was gated and why*, not the
//! Application state it guards.

use std::collections::BTreeMap;

use dxbot_core::types::{ApprovalId, OperationId};
use serde::{Deserialize, Serialize};

use crate::Error;

/// Terminal lifecycle of a parked operation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParkedGateState {
    /// Approval is still pending; the operation must not continue.
    Parked,
    /// Approval cleared and the original operation committed exactly once.
    Continued,
    /// Approval was denied; the operation must never continue.
    Denied,
    /// Approval cleared, but re-evaluation failed terminally (for example,
    /// because the original CAS became stale). The operation did not mutate.
    Failed,
}

/// Immutable identity/CAS the gate was created against. Continuation must
/// re-evaluate the *original* request against current CAS; these fields let the
/// coordinator prove the replayed request is byte-identical to what was parked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedGateBinding {
    /// Original OperationId allocated for the high-risk request.
    pub operation_id: OperationId,
    /// Approval whose terminal decision gates continuation.
    pub approval_id: ApprovalId,
    /// Exact request-identity digest (`RequestDigest`) parked at gate creation.
    pub request_digest: String,
    /// Command key the gate was raised for (audit/diagnostic binding).
    pub command_key: String,
    /// Target CAS snapshot label the gate was evaluated against.
    pub target: String,
    /// Policy generation the high-risk classification was decided under.
    pub policy_generation: i64,
}

/// A durable parked-operation gating record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParkedGateRecord {
    pub binding: ParkedGateBinding,
    pub state: ParkedGateState,
    /// Durable deny reason audit semantics (present iff `Denied`).
    pub deny_reason: Option<String>,
    /// Durable continuation failure detail (present iff `Failed`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    pub created_at: i64,
}

/// Canonical registry of parked high-risk operation gates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParkingManager {
    /// Keyed by ApprovalId so a decision resolves at most one gate.
    gates: BTreeMap<String, ParkedGateRecord>,
    /// Reverse index OperationId -> ApprovalId to fail duplicate parking closed.
    by_operation: BTreeMap<String, String>,
}

impl ParkingManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Park a high-risk operation behind a bound approval. Idempotent: replaying
    /// an identical park (same operation, approval, digest, target, generation)
    /// returns success; a conflicting re-park of the same OperationId fails
    /// closed.
    pub fn park(&mut self, binding: ParkedGateBinding, created_at: i64) -> Result<(), Error> {
        if binding.request_digest.trim().is_empty()
            || binding.command_key.trim().is_empty()
            || binding.target.trim().is_empty()
            || binding.policy_generation <= 0
        {
            return Err(Error::InvalidParkedGate(binding.operation_id));
        }
        if let Some(existing_approval) = self.by_operation.get(&binding.operation_id.0) {
            let existing = self
                .gates
                .get(existing_approval)
                .ok_or_else(|| Error::InvalidParkedGate(binding.operation_id.clone()))?;
            if existing.binding == binding {
                return Ok(());
            }
            return Err(Error::ParkedGateConflict(binding.operation_id));
        }
        if self.gates.contains_key(&binding.approval_id.0) {
            return Err(Error::ParkedGateConflict(binding.operation_id));
        }
        self.by_operation.insert(
            binding.operation_id.0.clone(),
            binding.approval_id.0.clone(),
        );
        self.gates.insert(
            binding.approval_id.0.clone(),
            ParkedGateRecord {
                binding,
                state: ParkedGateState::Parked,
                deny_reason: None,
                failure_reason: None,
                created_at,
            },
        );
        Ok(())
    }

    pub fn gate_for_approval(&self, approval_id: &ApprovalId) -> Option<ParkedGateRecord> {
        self.gates.get(&approval_id.0).cloned()
    }

    pub fn gate_for_operation(&self, operation_id: &OperationId) -> Option<ParkedGateRecord> {
        self.by_operation
            .get(&operation_id.0)
            .and_then(|approval| self.gates.get(approval))
            .cloned()
    }

    /// Records that the parked operation committed exactly once after approval.
    /// Idempotent for an already-continued gate; fails closed if the gate was
    /// denied (a denied operation must never continue).
    pub fn mark_continued(&mut self, approval_id: &ApprovalId) -> Result<(), Error> {
        let record = self
            .gates
            .get_mut(&approval_id.0)
            .ok_or_else(|| Error::UnknownParkedGate(approval_id.clone()))?;
        match record.state {
            ParkedGateState::Continued => Ok(()),
            ParkedGateState::Parked => {
                record.state = ParkedGateState::Continued;
                Ok(())
            }
            ParkedGateState::Denied => Err(Error::ParkedGateDenied(approval_id.clone())),
            ParkedGateState::Failed => Err(Error::ParkedGateConflict(
                record.binding.operation_id.clone(),
            )),
        }
    }

    /// Records a denial with durable reason audit semantics. Idempotent for an
    /// identical replayed denial; fails closed if the gate already continued.
    pub fn mark_denied(
        &mut self,
        approval_id: &ApprovalId,
        reason: Option<String>,
    ) -> Result<(), Error> {
        let record = self
            .gates
            .get_mut(&approval_id.0)
            .ok_or_else(|| Error::UnknownParkedGate(approval_id.clone()))?;
        match record.state {
            ParkedGateState::Denied => {
                if record.deny_reason == reason {
                    Ok(())
                } else {
                    Err(Error::ParkedGateConflict(
                        record.binding.operation_id.clone(),
                    ))
                }
            }
            ParkedGateState::Parked => {
                record.state = ParkedGateState::Denied;
                record.deny_reason = reason;
                Ok(())
            }
            ParkedGateState::Continued | ParkedGateState::Failed => Err(Error::ParkedGateConflict(
                record.binding.operation_id.clone(),
            )),
        }
    }

    /// Records a terminal continuation failure after approval. Replaying the
    /// same failure is idempotent; no terminal gate can be rewritten.
    pub fn mark_failed(&mut self, approval_id: &ApprovalId, reason: String) -> Result<(), Error> {
        if reason.trim().is_empty() {
            return Err(Error::UnknownParkedGate(approval_id.clone()));
        }
        let record = self
            .gates
            .get_mut(&approval_id.0)
            .ok_or_else(|| Error::UnknownParkedGate(approval_id.clone()))?;
        match record.state {
            ParkedGateState::Failed
                if record.failure_reason.as_deref() == Some(reason.as_str()) =>
            {
                Ok(())
            }
            ParkedGateState::Parked => {
                record.state = ParkedGateState::Failed;
                record.failure_reason = Some(reason);
                Ok(())
            }
            ParkedGateState::Continued | ParkedGateState::Denied | ParkedGateState::Failed => Err(
                Error::ParkedGateConflict(record.binding.operation_id.clone()),
            ),
        }
    }

    pub fn list_gates(&self) -> Vec<ParkedGateRecord> {
        self.gates.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn binding(operation: &str, approval: &str) -> ParkedGateBinding {
        ParkedGateBinding {
            operation_id: OperationId(operation.to_owned()),
            approval_id: ApprovalId(approval.to_owned()),
            request_digest: "digest-1".to_owned(),
            command_key: "project-member-set".to_owned(),
            target: "project:alpha".to_owned(),
            policy_generation: 3,
        }
    }

    #[test]
    fn park_is_idempotent_and_conflicts_fail_closed() {
        let mut manager = ParkingManager::new();
        manager
            .park(binding("op-a", "approval-0"), 1)
            .expect("park");
        manager
            .park(binding("op-a", "approval-0"), 1)
            .expect("idempotent replay");
        let mut conflicting = binding("op-a", "approval-1");
        conflicting.target = "project:beta".to_owned();
        assert!(matches!(
            manager.park(conflicting, 1),
            Err(Error::ParkedGateConflict(_))
        ));
    }

    #[test]
    fn denied_gate_can_never_continue() {
        let mut manager = ParkingManager::new();
        manager
            .park(binding("op-a", "approval-0"), 1)
            .expect("park");
        manager
            .mark_denied(
                &ApprovalId("approval-0".to_owned()),
                Some("policy".to_owned()),
            )
            .expect("deny");
        assert!(matches!(
            manager.mark_continued(&ApprovalId("approval-0".to_owned())),
            Err(Error::ParkedGateDenied(_))
        ));
        let gate = manager
            .gate_for_operation(&OperationId("op-a".to_owned()))
            .expect("gate");
        assert_eq!(gate.state, ParkedGateState::Denied);
        assert_eq!(gate.deny_reason.as_deref(), Some("policy"));
    }

    #[test]
    fn continue_is_idempotent() {
        let mut manager = ParkingManager::new();
        manager
            .park(binding("op-a", "approval-0"), 1)
            .expect("park");
        let id = ApprovalId("approval-0".to_owned());
        manager.mark_continued(&id).expect("continue");
        manager.mark_continued(&id).expect("idempotent continue");
        assert_eq!(
            manager.gate_for_approval(&id).expect("gate").state,
            ParkedGateState::Continued
        );
    }

    #[test]
    fn deny_reason_replay_must_match() {
        let mut manager = ParkingManager::new();
        manager
            .park(binding("op-a", "approval-0"), 1)
            .expect("park");
        let id = ApprovalId("approval-0".to_owned());
        manager
            .mark_denied(&id, Some("a".to_owned()))
            .expect("deny");
        manager
            .mark_denied(&id, Some("a".to_owned()))
            .expect("idempotent deny");
        assert!(matches!(
            manager.mark_denied(&id, Some("b".to_owned())),
            Err(Error::ParkedGateConflict(_))
        ));
    }
}

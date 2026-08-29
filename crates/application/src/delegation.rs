//! Typed multi-bot task delegation with membership and scope boundary.
//!
//! A delegation is a durable intent that one bot (`from_bot`) hands a task
//! (`task_id`) to another bot (`to_bot`) within a single scope. It is typed and
//! guarded: the delegating bot must be a member of—and hold a delegating role
//! within—the scope, and both the delegating and target bot must be members of
//! that same scope. Only the target bot may accept or reject a pending
//! delegation. [`DelegationRecord`]s live in the shared [`DomainState`] and are
//! listed as bounded, cursor-ordered [`DelegationSummary`] pages.

use std::sync::{Arc, Mutex};

use dxbot_core::types::{BotSelector, ScopeSelector, TaskId};

use crate::membership::MembershipManager;
use crate::mutation::AppError;
use crate::query::{Page, paginate};
use crate::state::DomainState;

/// Lifecycle of a task delegation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegationStatus {
    /// Created and awaiting the target bot's decision.
    Pending,
    /// Accepted by the target bot.
    Accepted,
    /// Rejected by the target bot.
    Rejected,
    /// No longer actionable (e.g. scope/task resolved).
    Expired,
}

/// A typed delegation of a task from one bot to another within a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationRecord {
    /// Stable unique delegation identity.
    pub id: String,
    /// The task being delegated.
    pub task_id: TaskId,
    /// The delegating bot (must hold a delegating role in the scope).
    pub from_bot: BotSelector,
    /// The receiving bot (the only bot that may accept or reject).
    pub to_bot: BotSelector,
    /// The scope the delegation lives within.
    pub scope: ScopeSelector,
    /// The role the delegating bot held when creating the delegation.
    pub role: String,
    /// Current status.
    pub status: DelegationStatus,
    /// Unix timestamp (milliseconds) at creation.
    pub created_at: i64,
    /// Unix timestamp (milliseconds) at which the delegation was resolved.
    pub resolved_at: Option<i64>,
}

/// Lightweight delegation summary returned by listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelegationSummary {
    pub id: String,
    pub task_id: TaskId,
    pub from_bot: BotSelector,
    pub to_bot: BotSelector,
    pub status: DelegationStatus,
}

/// Manages typed task delegation with membership and scope-boundary enforcement.
#[derive(Debug)]
pub struct DelegationManager {
    state: Arc<Mutex<DomainState>>,
}

impl DelegationManager {
    /// Create a delegation manager over shared in-memory domain state.
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    /// Create a typed delegation of `task_id` from `from_bot` to `to_bot`
    /// within `scope`.
    ///
    /// Validates that both bots are members of `scope` and that `from_bot`
    /// holds a role with delegation authority. The resulting record starts
    /// [`Pending`](DelegationStatus::Pending).
    pub fn delegate_task(
        &mut self,
        task_id: &TaskId,
        from_bot: &BotSelector,
        to_bot: &BotSelector,
        scope: &ScopeSelector,
    ) -> Result<DelegationRecord, AppError> {
        let membership = MembershipManager::new(self.state.clone());
        // Scope boundary: the delegating bot must be a member of the scope it
        // is delegating within, and hold a delegating role there.
        let role = membership.require_delegation_authority(scope, from_bot)?;
        // The target bot must also be a member of the *same* scope.
        membership.require_member(scope, to_bot)?;

        let mut guard = self.lock()?;
        let id = format!("deleg-{}", guard.delegations.len() + 1);
        let created_at = now_ms();
        let record = DelegationRecord {
            id,
            task_id: task_id.clone(),
            from_bot: from_bot.clone(),
            to_bot: to_bot.clone(),
            scope: scope.clone(),
            role,
            status: DelegationStatus::Pending,
            created_at,
            resolved_at: None,
        };
        guard.delegations.push(record.clone());
        Ok(record)
    }

    /// Accept a pending delegation as the target bot.
    ///
    /// Only `record.to_bot` may accept. Accepting a non-pending delegation is a
    /// conflict.
    pub fn accept_delegation(
        &mut self,
        delegation_id: &str,
        by_bot: &BotSelector,
    ) -> Result<DelegationRecord, AppError> {
        self.resolve(delegation_id, by_bot, DelegationStatus::Accepted)
    }

    /// Reject a pending delegation as the target bot.
    ///
    /// Only `record.to_bot` may reject. Rejecting a non-pending delegation is a
    /// conflict.
    pub fn reject_delegation(
        &mut self,
        delegation_id: &str,
        by_bot: &BotSelector,
    ) -> Result<DelegationRecord, AppError> {
        self.resolve(delegation_id, by_bot, DelegationStatus::Rejected)
    }

    /// Fetch a delegation by id, if present.
    pub fn get_delegation(&self, delegation_id: &str) -> Result<DelegationRecord, AppError> {
        let guard = self.lock()?;
        guard
            .delegations
            .iter()
            .find(|d| d.id == delegation_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("delegation {delegation_id} not found")))
    }

    /// List delegations within a scope as a bounded, cursor-ordered page.
    pub fn list_delegations(
        &self,
        scope: &ScopeSelector,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<DelegationSummary>, AppError> {
        let guard = self.lock()?;
        let delegations: Vec<DelegationRecord> = guard
            .delegations
            .iter()
            .filter(|d| d.scope == *scope)
            .cloned()
            .collect();
        let page = paginate(
            delegations,
            |d| d.id.clone(),
            to_summary,
            page_size,
            cursor.as_deref(),
        );
        Ok(page)
    }

    fn resolve(
        &mut self,
        delegation_id: &str,
        by_bot: &BotSelector,
        status: DelegationStatus,
    ) -> Result<DelegationRecord, AppError> {
        let mut guard = self.lock()?;
        let record = guard
            .delegations
            .iter_mut()
            .find(|d| d.id == delegation_id)
            .ok_or_else(|| AppError::NotFound(format!("delegation {delegation_id} not found")))?;
        if record.status != DelegationStatus::Pending {
            return Err(AppError::Conflict(format!(
                "delegation {} is already {:?}; only pending delegations can be resolved",
                delegation_id, record.status
            )));
        }
        if record.to_bot != *by_bot {
            return Err(AppError::PermissionDenied(format!(
                "bot {by_bot:?} is not the target of delegation {delegation_id}"
            )));
        }
        record.status = status;
        record.resolved_at = Some(now_ms());
        Ok(record.clone())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

fn to_summary(record: DelegationRecord) -> DelegationSummary {
    DelegationSummary {
        id: record.id,
        task_id: record.task_id,
        from_bot: record.from_bot,
        to_bot: record.to_bot,
        status: record.status,
    }
}

/// Current Unix timestamp (milliseconds).
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

//! Application membership records, generation CAS, and capability-local role
//! checks.
//!
//! This module owns membership facts only. Runtime AuthorityBinding is owned by
//! `runtime-security`; duplicating it here would create two canonical security
//! states that can diverge.

use std::sync::{Arc, Mutex};

use dxbot_core::types::{BotSelector, ChannelSelector, ProjectSelector, ScopeSelector};
use serde::{Deserialize, Serialize};

use crate::mutation::AppError;
use crate::query::{Page, paginate};
use crate::state::DomainState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRecord {
    pub id: String,
    pub scope: ScopeSelector,
    pub member_bot: BotSelector,
    pub role: String,
    pub generation: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipSummary {
    pub member_bot: BotSelector,
    pub role: String,
    pub generation: i64,
}

#[derive(Debug)]
pub struct MembershipManager {
    state: Arc<Mutex<DomainState>>,
}

impl MembershipManager {
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    pub fn set_membership(
        &mut self,
        scope: &ScopeSelector,
        member_bot: &BotSelector,
        role: &str,
        if_membership_generation: Option<i64>,
    ) -> Result<MembershipRecord, AppError> {
        if role.trim().is_empty() {
            return Err(AppError::Conflict(
                "membership role must not be empty".to_owned(),
            ));
        }
        let mut guard = self.lock()?;
        let key = membership_key(scope, member_bot);

        if let Some(want) = if_membership_generation {
            match guard.memberships.get(&key).map(|membership| membership.generation) {
                None => {
                    return Err(AppError::NotFound(format!(
                        "membership {key} does not exist but a generation precondition was set"
                    )));
                }
                Some(current) if current != want => {
                    return Err(AppError::Conflict(format!(
                        "membership {key} generation mismatch: expected {want}, current {current}"
                    )));
                }
                Some(_) => {}
            }
        }

        let now = now();
        let record = match guard.memberships.get(&key) {
            Some(existing) => MembershipRecord {
                id: existing.id.clone(),
                scope: scope.clone(),
                member_bot: member_bot.clone(),
                role: role.to_owned(),
                generation: existing.generation.checked_add(1).ok_or_else(|| {
                    AppError::Internal(format!("membership {key} generation exhausted"))
                })?,
                created_at: existing.created_at,
            },
            None => MembershipRecord {
                id: key.clone(),
                scope: scope.clone(),
                member_bot: member_bot.clone(),
                role: role.to_owned(),
                generation: 1,
                created_at: now,
            },
        };

        guard.memberships.insert(key, record.clone());
        Ok(record)
    }

    pub fn remove_membership(
        &mut self,
        scope: &ScopeSelector,
        member_bot: &BotSelector,
        if_membership_generation: i64,
    ) -> Result<(), AppError> {
        let mut guard = self.lock()?;
        let key = membership_key(scope, member_bot);
        match guard.memberships.get(&key).map(|membership| membership.generation) {
            None => Err(AppError::NotFound(format!("membership {key} does not exist"))),
            Some(current) if current != if_membership_generation => Err(AppError::Conflict(
                format!(
                    "membership {key} generation mismatch: expected {if_membership_generation}, current {current}"
                ),
            )),
            Some(_) => {
                guard.memberships.remove(&key);
                Ok(())
            }
        }
    }

    pub fn list_memberships(
        &self,
        scope: &ScopeSelector,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<MembershipSummary>, AppError> {
        let guard = self.lock()?;
        let rows = guard
            .memberships
            .values()
            .filter(|membership| membership.scope == *scope)
            .cloned()
            .collect::<Vec<_>>();
        Ok(paginate(
            rows,
            |membership| membership.id.clone(),
            |membership| MembershipSummary {
                member_bot: membership.member_bot,
                role: membership.role,
                generation: membership.generation,
            },
            page_size,
            cursor.as_deref(),
        ))
    }

    pub fn require_member(
        &self,
        scope: &ScopeSelector,
        bot: &BotSelector,
    ) -> Result<MembershipRecord, AppError> {
        let guard = self.lock()?;
        guard
            .memberships
            .get(&membership_key(scope, bot))
            .cloned()
            .ok_or_else(|| AppError::PermissionDenied("bot is not a scope member".to_owned()))
    }

    pub fn require_delegation_authority(
        &self,
        scope: &ScopeSelector,
        bot: &BotSelector,
    ) -> Result<String, AppError> {
        let membership = self.require_member(scope, bot)?;
        if matches!(membership.role.as_str(), "owner" | "admin" | "delegate") {
            Ok(membership.role)
        } else {
            Err(AppError::PermissionDenied(format!(
                "membership role {} cannot delegate",
                membership.role
            )))
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

fn membership_key(scope: &ScopeSelector, bot: &BotSelector) -> String {
    format!("{}|{}", scope_key(scope), bot_key(bot))
}

fn scope_key(scope: &ScopeSelector) -> String {
    match scope {
        ScopeSelector::Bot(selector) => format!("bot:{}", bot_key(selector)),
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => format!("project:{}", id.0),
        ScopeSelector::Project(ProjectSelector::VisibleExact(name)) => {
            format!("project:exact:{name}")
        }
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => format!("channel:{}", id.0),
        ScopeSelector::Channel(ChannelSelector::ProjectExact { project, name }) => {
            format!("channel:{project}:{name}")
        }
    }
}

fn bot_key(bot: &BotSelector) -> String {
    match bot {
        BotSelector::CanonicalId(id) => id.0.clone(),
        BotSelector::ScopedExact(name) => format!("exact:{name}"),
    }
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

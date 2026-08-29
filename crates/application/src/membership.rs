//! Application membership records, generation CAS, and capability-local role
//! checks.
//!
//! This module owns membership facts only. Runtime AuthorityBinding is owned by
//! `runtime-security`; duplicating it here would create two canonical security
//! states that can diverge. Removed rows remain inactive generation tombstones
//! so stale pre-removal CAS cannot recreate an ABA-equivalent membership.

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
    #[serde(default = "default_active")]
    pub active: bool,
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

    pub fn snapshot(&self) -> Result<DomainState, AppError> {
        self.lock().map(|state| state.clone())
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
                active: true,
                created_at: existing.created_at,
            },
            None => MembershipRecord {
                id: key.clone(),
                scope: scope.clone(),
                member_bot: member_bot.clone(),
                role: role.to_owned(),
                generation: 1,
                active: true,
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
        let membership = guard
            .memberships
            .get_mut(&key)
            .ok_or_else(|| AppError::NotFound(format!("membership {key} does not exist")))?;
        if !membership.active {
            return Err(AppError::NotFound(format!("membership {key} is not active")));
        }
        if membership.generation != if_membership_generation {
            return Err(AppError::Conflict(format!(
                "membership {key} generation mismatch: expected {if_membership_generation}, current {}",
                membership.generation
            )));
        }
        membership.generation = membership.generation.checked_add(1).ok_or_else(|| {
            AppError::Internal(format!("membership {key} generation exhausted"))
        })?;
        membership.active = false;
        Ok(())
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
            .filter(|membership| membership.active && membership.scope == *scope)
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
            .filter(|membership| membership.active)
            .cloned()
            .ok_or_else(|| AppError::PermissionDenied("bot is not a scope member".to_owned()))
    }

    pub fn require_delegation_authority(
        &self,
        scope: &ScopeSelector,
        bot: &BotSelector,
    ) -> Result<String, AppError> {
        let membership = self.require_member(scope, bot)?;
        if Self::can_delegate(&membership.role) {
            Ok(membership.role)
        } else {
            Err(AppError::PermissionDenied(format!(
                "membership role {} cannot delegate",
                membership.role
            )))
        }
    }

    pub fn can_delegate(role: &str) -> bool {
        let role = role.trim();
        role.eq_ignore_ascii_case("owner")
            || role.eq_ignore_ascii_case("admin")
            || role.eq_ignore_ascii_case("delegate")
            || role.eq_ignore_ascii_case("coordinator")
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

pub(crate) fn membership_key(scope: &ScopeSelector, bot: &BotSelector) -> String {
    let scope = scope_key(scope);
    let bot = bot_key(bot);
    format!("{}:{scope}{}:{bot}", scope.len(), bot.len())
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

fn default_active() -> bool {
    true
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use dxbot_core::types::{BotId, ProjectId};

    use super::*;

    fn fixture() -> (MembershipManager, ScopeSelector, BotSelector) {
        let state = Arc::new(Mutex::new(DomainState::new()));
        let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
            "project-a".to_owned(),
        )));
        let bot = BotSelector::CanonicalId(BotId("bot-a".to_owned()));
        (MembershipManager::new(state), scope, bot)
    }

    #[test]
    fn remove_and_readd_never_reuses_generation() {
        let (mut manager, scope, bot) = fixture();
        let created = manager
            .set_membership(&scope, &bot, "member", None)
            .expect("create membership");
        manager
            .remove_membership(&scope, &bot, created.generation)
            .expect("remove membership");
        let snapshot = manager.snapshot().expect("snapshot");
        let tombstone = snapshot
            .memberships
            .get(&membership_key(&scope, &bot))
            .expect("tombstone retained");
        assert!(!tombstone.active);
        assert_eq!(tombstone.generation, 2);

        let stale = manager.set_membership(&scope, &bot, "member", Some(1));
        assert!(matches!(stale, Err(AppError::Conflict(_))));

        let restored = manager
            .set_membership(&scope, &bot, "member", Some(2))
            .expect("re-add with current generation");
        assert!(restored.active);
        assert_eq!(restored.generation, 3);
    }
}

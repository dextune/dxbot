//! Application membership: membership records, generation-based CAS, and
//! atomically-bound authority within a scope.
//!
//! [`MembershipManager`] manages which bot is a member of which scope (Project
//! or Channel) and with which `role`. Every membership is guarded by a
//! monotonically increasing `generation` used for compare-and-set (CAS), and is
//! written *atomically* alongside an [`AuthorityBinding`] that grants the same
//! principal (the member bot) authority in that scope. Removing a membership
//! revokes the matching authority binding in the same operation.

use std::sync::{Arc, Mutex};

use dxbot_core::types::{BotSelector, ChannelSelector, ProjectSelector, ScopeSelector};

use crate::mutation::AppError;
use crate::query::{Page, paginate};
use crate::state::DomainState;

/// Canonical membership identity of a bot within a scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRecord {
    /// Stable, unique identity of the membership within the manager.
    pub id: String,
    /// The scope (Project or Channel) the membership applies to.
    pub scope: ScopeSelector,
    /// The bot that holds the membership.
    pub member_bot: BotSelector,
    /// The role the member holds within the scope (e.g. `admin`, `member`).
    pub role: String,
    /// Monotonic generation used for CAS; increments on every update.
    pub generation: i64,
    /// Unix timestamp (seconds) at which the membership was first created.
    pub created_at: i64,
}

/// Lightweight membership summary returned by listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipSummary {
    /// The member bot.
    pub member_bot: BotSelector,
    /// The role the member holds within the scope.
    pub role: String,
    /// The current membership generation.
    pub generation: i64,
}

/// Authority bound to a principal (member bot) within a scope.
///
/// Written atomically with membership: creating a membership also binds the
/// bot's authority, and removing the membership revokes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityBinding {
    /// The principal (member bot) that holds the authority.
    pub principal: BotSelector,
    /// The scope (Project or Channel) the authority applies to.
    pub scope: ScopeSelector,
    /// The role granted within the scope.
    pub role: String,
    /// Unix timestamp (seconds) at which the authority was bound.
    pub bound_at: i64,
}

/// Manages application membership with generation CAS and atomic authority
/// binding over a shared in-memory [`DomainState`].
#[derive(Debug)]
pub struct MembershipManager {
    state: Arc<Mutex<DomainState>>,
}

impl MembershipManager {
    /// Create a manager over a shared in-memory domain state store.
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    /// Set or update a bot's membership in a scope.
    ///
    /// * Creates the membership when it does not exist yet (generation `1`).
    /// * Updates the membership (generation `+ 1`) when it already exists.
    /// * When `if_membership_generation` is provided, the current generation
    ///   must match exactly, otherwise the call errors with a CAS conflict.
    ///
    /// The membership record and its [`AuthorityBinding`] are written
    /// atomically in one operation.
    pub fn set_membership(
        &mut self,
        scope: &ScopeSelector,
        member_bot: &BotSelector,
        role: &str,
        if_membership_generation: Option<i64>,
    ) -> Result<MembershipRecord, AppError> {
        let mut guard = self.lock()?;
        let key = membership_key(scope, member_bot);

        // CAS: verify the current generation matches when a precondition is given.
        if let Some(want) = if_membership_generation {
            let current = guard.memberships.get(&key).map(|m| m.generation);
            match current {
                None => {
                    return Err(AppError::NotFound(format!(
                        "membership {key} does not exist but a generation precondition was set"
                    )));
                }
                Some(cur) if cur != want => {
                    return Err(AppError::Conflict(format!(
                        "membership {key} generation mismatch: expected {want}, current {cur}"
                    )));
                }
                Some(_) => {}
            }
        }

        let now = now();
        let record = match guard.memberships.get(&key) {
            Some(existing) => {
                let generation = existing.generation + 1;
                MembershipRecord {
                    id: existing.id.clone(),
                    scope: scope.clone(),
                    member_bot: member_bot.clone(),
                    role: role.to_string(),
                    generation,
                    created_at: existing.created_at,
                }
            }
            None => MembershipRecord {
                id: key.clone(),
                scope: scope.clone(),
                member_bot: member_bot.clone(),
                role: role.to_string(),
                generation: 1,
                created_at: now,
            },
        };

        // Atomic: membership record and authority binding are written together.
        guard.memberships.insert(key.clone(), record.clone());
        guard.authority_bindings.insert(
            authority_key(scope, member_bot),
            AuthorityBinding {
                principal: member_bot.clone(),
                scope: scope.clone(),
                role: role.to_string(),
                bound_at: now,
            },
        );
        Ok(record)
    }

    /// Remove a bot's membership in a scope, requiring an exact generation match.
    ///
    /// The membership and its [`AuthorityBinding`] are removed in the same
    /// operation. A generation mismatch errors with a CAS conflict.
    pub fn remove_membership(
        &mut self,
        scope: &ScopeSelector,
        member_bot: &BotSelector,
        if_membership_generation: i64,
    ) -> Result<(), AppError> {
        let mut guard = self.lock()?;
        let key = membership_key(scope, member_bot);
        match guard.memberships.get(&key).map(|m| m.generation) {
            None => Err(AppError::NotFound(format!("membership {key} does not exist"))),
            Some(cur) if cur != if_membership_generation => Err(AppError::Conflict(format!(
                "membership {key} generation mismatch: expected {if_membership_generation}, \
                 current {cur}"
            ))),
            Some(_) => {
                guard.memberships.remove(&key);
                guard.authority_bindings.remove(&authority_key(scope, member_bot));
                Ok(())
            }
        }
    }

    /// List memberships in a scope as a bounded, cursor-ordered page.
    pub fn list_memberships(
        &self,
        scope: &ScopeSelector,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<MembershipSummary>, AppError> {
        let guard = self.lock()?;
        let members: Vec<MembershipRecord> = guard
            .memberships
            .values()
            .filter(|m| m.scope == *scope)
            .cloned()
            .collect();
        let page = paginate(
            members,
            |m| m.id.clone(),
            to_membership_summary,
            page_size,
            cursor.as_deref(),
        );
        Ok(page)
    }

    /// Whether `bot` is a current member of `scope`.
    pub fn is_member(&self, scope: &ScopeSelector, bot: &BotSelector) -> bool {
        self.role_of(scope, bot).is_some()
    }

    /// The role `bot` holds within `scope`, if it is a member.
    pub fn role_of(&self, scope: &ScopeSelector, bot: &BotSelector) -> Option<String> {
        self.state
            .lock()
            .ok()
            .and_then(|guard| guard.memberships.get(&membership_key(scope, bot)).map(|m| m.role.clone()))
    }

    /// Require that `bot` is a member of `scope`, else a permission error.
    ///
    /// Used by capability owners (e.g. delegation) to enforce the membership
    /// boundary before acting in a scope.
    pub fn require_member(
        &self,
        scope: &ScopeSelector,
        bot: &BotSelector,
    ) -> Result<MembershipRecord, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))?
            .memberships
            .get(&membership_key(scope, bot))
            .cloned()
            .ok_or_else(|| {
                AppError::PermissionDenied(format!(
                    "bot {bot:?} is not a member of scope {scope:?}"
                ))
            })
    }

    /// Require that `bot` is both a member of `scope` and holds a role that may
    /// originate a delegation there, returning the held role.
    pub fn require_delegation_authority(
        &self,
        scope: &ScopeSelector,
        bot: &BotSelector,
    ) -> Result<String, AppError> {
        let membership = self.require_member(scope, bot)?;
        if !Self::can_delegate(&membership.role) {
            return Err(AppError::PermissionDenied(format!(
                "bot {bot:?} with role {:?} is not authorized to delegate in scope {scope:?}",
                membership.role
            )));
        }
        Ok(membership.role)
    }

    /// Whether a role carries delegation authority.
    ///
    /// Role-based by design: only coordinating/owning roles may originate a
    /// delegation. `RoleRef` is not general authority; this is the
    /// role-to-capability map owned by the delegation capability.
    pub fn can_delegate(role: &str) -> bool {
        let normalized = role.to_lowercase();
        normalized.contains("owner")
            || normalized.contains("admin")
            || normalized.contains("manager")
            || normalized.contains("coordinator")
            || normalized == "lead"
    }

    /// Clone a snapshot of the current domain state.
    ///
    /// Returns [`AppError::Internal`] when the internal mutex is poisoned,
    /// indicating a prior panic in a critical section.
    pub fn snapshot(&self) -> Result<DomainState, AppError> {
        self.lock().map(|g| g.clone())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

/// Stable identity key for a membership `(scope, member_bot)`.
fn membership_key(scope: &ScopeSelector, member_bot: &BotSelector) -> String {
    format!("m:{}|{}", scope_key(scope), bot_key(member_bot))
}

/// Stable identity key for an authority binding `(scope, principal)`.
fn authority_key(scope: &ScopeSelector, principal: &BotSelector) -> String {
    format!("a:{}|{}", scope_key(scope), bot_key(principal))
}

/// A stable, human-readable key for a bot selector.
fn bot_key(bot: &BotSelector) -> String {
    match bot {
        BotSelector::CanonicalId(id) => format!("bot:{}", id.0),
        BotSelector::ScopedExact(name) => format!("exact:{name}"),
    }
}

/// A stable, human-readable key for a scope selector.
fn scope_key(scope: &ScopeSelector) -> String {
    match scope {
        ScopeSelector::Bot(b) => format!("bot-scope:{}", bot_key(b)),
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => format!("project:{}", id.0),
        ScopeSelector::Project(ProjectSelector::VisibleExact(name)) => {
            format!("project-exact:{name}")
        }
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => format!("channel:{}", id.0),
        ScopeSelector::Channel(ChannelSelector::ProjectExact { project, name }) => {
            format!("channel-project:{project}:{name}")
        }
    }
}

fn to_membership_summary(record: MembershipRecord) -> MembershipSummary {
    MembershipSummary {
        member_bot: record.member_bot,
        role: record.role,
        generation: record.generation,
    }
}

/// Current Unix timestamp (seconds), used for `created_at`/`bound_at`.
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
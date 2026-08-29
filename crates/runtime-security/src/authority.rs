//! Principal authority and membership-subject authority bindings.
//!
//! Authenticated Principal authority and Bot membership authority are distinct
//! canonical records. A BotId is never promoted to a PrincipalRef. Application
//! coordinates membership facts with the membership-subject binding generation.

use std::collections::{HashMap, HashSet};

use dxbot_core::types::{
    BotSelector, ChannelSelector, PrincipalRef, ProjectSelector, ScopeSelector,
};
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipAuthorityBinding {
    pub binding_id: String,
    pub scope: ScopeSelector,
    pub member_bot: BotSelector,
    pub role: String,
    pub generation: i64,
    pub active: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthorityManager {
    bindings: HashMap<String, HashMap<String, HashSet<String>>>,
    global_roles: HashMap<String, HashSet<String>>,
    membership_bindings: HashMap<String, MembershipAuthorityBinding>,
}

impl AuthorityManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_authority(
        &mut self,
        principal: &PrincipalRef,
        scope: &ScopeSelector,
        role: &str,
    ) -> Result<(), Error> {
        let key = scope_key(scope);
        self.bindings
            .entry(principal.0.clone())
            .or_default()
            .entry(key)
            .or_default()
            .insert(role.to_string());
        Ok(())
    }

    pub fn bind_global_authority(
        &mut self,
        principal: &PrincipalRef,
        role: &str,
    ) -> Result<(), Error> {
        self.global_roles
            .entry(principal.0.clone())
            .or_default()
            .insert(role.to_owned());
        Ok(())
    }

    pub fn check_authority(
        &self,
        principal: &PrincipalRef,
        scope: &ScopeSelector,
        required_role: &str,
    ) -> Result<bool, Error> {
        let key = scope_key(scope);
        Ok(self
            .bindings
            .get(&principal.0)
            .and_then(|by_principal| by_principal.get(&key))
            .is_some_and(|roles| roles.contains(required_role)))
    }

    pub fn check_global_authority(
        &self,
        principal: &PrincipalRef,
        required_role: &str,
    ) -> Result<bool, Error> {
        Ok(self
            .global_roles
            .get(&principal.0)
            .is_some_and(|roles| roles.contains(required_role)))
    }

    pub fn revoke_authority(
        &mut self,
        principal: &PrincipalRef,
        scope: &ScopeSelector,
    ) -> Result<(), Error> {
        let key = scope_key(scope);
        if let Some(by_principal) = self.bindings.get_mut(&principal.0) {
            by_principal.remove(&key);
            if by_principal.is_empty() {
                self.bindings.remove(&principal.0);
            }
        }
        Ok(())
    }

    pub fn revoke_global_authority(
        &mut self,
        principal: &PrincipalRef,
        role: &str,
    ) -> Result<(), Error> {
        if let Some(roles) = self.global_roles.get_mut(&principal.0) {
            roles.remove(role);
            if roles.is_empty() {
                self.global_roles.remove(&principal.0);
            }
        }
        Ok(())
    }

    pub fn apply_membership_binding(
        &mut self,
        binding: MembershipAuthorityBinding,
    ) -> Result<(), Error> {
        match self.membership_bindings.get(&binding.binding_id) {
            Some(existing) if existing == &binding => return Ok(()),
            Some(existing) if binding.generation <= existing.generation => {
                return Err(Error::StaleAuthorityBinding(binding.binding_id));
            }
            _ => {}
        }
        self.membership_bindings
            .insert(binding.binding_id.clone(), binding);
        Ok(())
    }

    /// Revokes an active membership using the caller's pre-removal generation.
    /// The canonical binding generation advances exactly once; replaying the
    /// same revocation is accepted only when the resulting tombstone already
    /// exists at `expected_generation + 1`.
    pub fn revoke_membership_binding(
        &mut self,
        binding_id: &str,
        expected_generation: i64,
    ) -> Result<(), Error> {
        let Some(existing) = self.membership_bindings.get_mut(binding_id) else {
            return Err(Error::UnknownAuthorityBinding(binding_id.to_owned()));
        };
        let tombstone_generation = expected_generation
            .checked_add(1)
            .ok_or_else(|| Error::StaleAuthorityBinding(binding_id.to_owned()))?;
        if !existing.active && existing.generation == tombstone_generation {
            return Ok(());
        }
        if !existing.active || existing.generation != expected_generation {
            return Err(Error::StaleAuthorityBinding(binding_id.to_owned()));
        }
        existing.generation = tombstone_generation;
        existing.active = false;
        Ok(())
    }

    pub fn membership_binding(&self, binding_id: &str) -> Option<MembershipAuthorityBinding> {
        self.membership_bindings.get(binding_id).cloned()
    }
}

fn scope_key(scope: &ScopeSelector) -> String {
    match scope {
        ScopeSelector::Bot(BotSelector::CanonicalId(id)) => format!("bot:{}", id.0),
        ScopeSelector::Bot(BotSelector::ScopedExact(value)) => format!("bot:exact:{value}"),
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => format!("project:{}", id.0),
        ScopeSelector::Project(ProjectSelector::VisibleExact(value)) => {
            format!("project:visible:{value}")
        }
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => format!("channel:{}", id.0),
        ScopeSelector::Channel(ChannelSelector::ProjectExact { project, name }) => {
            format!("channel:{project}:{name}")
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use dxbot_core::types::{BotId, ProjectId};

    use super::*;

    fn membership_binding(generation: i64, active: bool) -> MembershipAuthorityBinding {
        MembershipAuthorityBinding {
            binding_id: "membership-1".to_owned(),
            scope: ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
                "project-a".to_owned(),
            ))),
            member_bot: BotSelector::CanonicalId(BotId("bot-a".to_owned())),
            role: "member".to_owned(),
            generation,
            active,
        }
    }

    #[test]
    fn registration_does_not_imply_global_authority() {
        let manager = AuthorityManager::new();
        let principal = PrincipalRef("local:i:uid:1000".to_owned());
        assert!(
            !manager
                .check_global_authority(&principal, "operator")
                .unwrap()
        );
    }

    #[test]
    fn global_authority_is_explicit_and_revocable() {
        let mut manager = AuthorityManager::new();
        let principal = PrincipalRef("local:i:uid:1000".to_owned());
        manager
            .bind_global_authority(&principal, "operator")
            .unwrap();
        assert!(
            manager
                .check_global_authority(&principal, "operator")
                .unwrap()
        );
        manager
            .revoke_global_authority(&principal, "operator")
            .unwrap();
        assert!(
            !manager
                .check_global_authority(&principal, "operator")
                .unwrap()
        );
    }

    #[test]
    fn membership_binding_keeps_bot_subject_separate_from_principal() {
        let mut manager = AuthorityManager::new();
        let binding = membership_binding(1, true);
        manager.apply_membership_binding(binding.clone()).unwrap();
        assert_eq!(manager.membership_binding("membership-1"), Some(binding));
    }

    #[test]
    fn same_generation_cannot_change_membership_binding_state() {
        let mut manager = AuthorityManager::new();
        let binding = membership_binding(4, true);
        manager.apply_membership_binding(binding.clone()).unwrap();
        let mut stale = binding;
        stale.active = false;
        assert!(matches!(
            manager.apply_membership_binding(stale),
            Err(Error::StaleAuthorityBinding(_))
        ));
    }

    #[test]
    fn revocation_advances_generation_once_and_replay_is_idempotent() {
        let mut manager = AuthorityManager::new();
        manager
            .apply_membership_binding(membership_binding(4, true))
            .unwrap();
        manager
            .revoke_membership_binding("membership-1", 4)
            .unwrap();
        let tombstone = manager.membership_binding("membership-1").unwrap();
        assert!(!tombstone.active);
        assert_eq!(tombstone.generation, 5);
        manager
            .revoke_membership_binding("membership-1", 4)
            .unwrap();
        assert_eq!(
            manager.membership_binding("membership-1").unwrap(),
            tombstone
        );
    }
}

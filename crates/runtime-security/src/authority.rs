//! Principal authority bindings.
//!
//! [`AuthorityManager`] is the canonical owner of grants. Scope grants remain
//! exact and deny-unknown. A distinct instance-global role set exists for
//! operations whose canonical target has no Bot/Project/Channel scope (for
//! example instance creation or recovery). Global authority is always explicit;
//! principal registration alone still grants nothing.

use std::collections::{HashMap, HashSet};

use dxbot_core::types::{
    BotSelector, ChannelSelector, PrincipalRef, ProjectSelector, ScopeSelector,
};

use crate::Error;

/// Canonical in-memory store of authority bindings.
#[derive(Debug, Clone, Default)]
pub struct AuthorityManager {
    /// principal key -> scope key -> roles
    bindings: HashMap<String, HashMap<String, HashSet<String>>>,
    /// principal key -> explicit instance-global roles
    global_roles: HashMap<String, HashSet<String>>,
}

impl AuthorityManager {
    /// Create an empty authority store (deny-unknown).
    pub fn new() -> Self {
        Self::default()
    }

    /// Grant `role` to `principal` on `scope`.
    ///
    /// Granting is idempotent: re-granting the same role is a no-op.
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

    /// Explicitly grant an instance-global role.
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

    /// Check whether `principal` holds `required_role` on `scope`.
    ///
    /// Unknown principals or scopes are denied (`Ok(false)`), never an error.
    pub fn check_authority(
        &self,
        principal: &PrincipalRef,
        scope: &ScopeSelector,
        required_role: &str,
    ) -> Result<bool, Error> {
        let key = scope_key(scope);
        let held = self
            .bindings
            .get(&principal.0)
            .map(|by_principal| {
                by_principal
                    .get(&key)
                    .map(|roles| roles.contains(required_role))
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        Ok(held)
    }

    /// Check an explicit instance-global role. Registration alone never makes
    /// this return true.
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

    /// Revoke every role `principal` holds on `scope`.
    ///
    /// Revoking an absent binding is a safe no-op (idempotent).
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

    /// Revoke one instance-global role. Absent grants are a safe no-op.
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
}

/// Flatten a [`ScopeSelector`] into a stable canonical key.
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

    use super::*;

    #[test]
    fn registration_does_not_imply_global_authority() {
        let manager = AuthorityManager::new();
        let principal = PrincipalRef("local:i:uid:1000".to_owned());
        assert!(!manager.check_global_authority(&principal, "operator").unwrap());
    }

    #[test]
    fn global_authority_is_explicit_and_revocable() {
        let mut manager = AuthorityManager::new();
        let principal = PrincipalRef("local:i:uid:1000".to_owned());
        manager
            .bind_global_authority(&principal, "operator")
            .unwrap();
        assert!(manager.check_global_authority(&principal, "operator").unwrap());
        manager
            .revoke_global_authority(&principal, "operator")
            .unwrap();
        assert!(!manager.check_global_authority(&principal, "operator").unwrap());
    }
}

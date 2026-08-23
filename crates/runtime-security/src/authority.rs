//! Principal-to-scope authority bindings.
//!
//! [`AuthorityManager`] is the canonical owner of `AuthorityBinding` grants:
//! which principal holds which role on which scope. Default is deny-unknown:
//! an absent binding grants nothing, and a revoked binding grants nothing.
//!
//! Scopes are flattened to a deterministic canonical key because the network
//! `ScopeSelector` is not `Hash`; the key preserves scope identity exactly.

use std::collections::{HashMap, HashSet};

use dxbot_core::types::{
    BotSelector, PrincipalRef, ProjectSelector, ScopeSelector, {ChannelSelector},
};

use crate::Error;

/// Canonical in-memory store of authority bindings.
#[derive(Debug, Clone, Default)]
pub struct AuthorityManager {
    /// principal key -> scope key -> roles
    bindings: HashMap<String, HashMap<String, HashSet<String>>>,
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
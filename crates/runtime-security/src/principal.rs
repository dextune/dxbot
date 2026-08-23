//! Principal registry and lifecycle.
//!
//! A [`Principal`](dxbot_core::types::PrincipalRef) is derived server-side from
//! the authenticated peer, never from client hints or domain IDs. The
//! [`PrincipalManager`] owns the canonical registry: registration, resolution,
//! and lifecycle status. Authority is never inferred from registration alone.

use std::collections::HashMap;

use dxbot_core::types::PrincipalRef;

use crate::Error;

/// Lifecycle status of a registered principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalStatus {
    /// Registered and allowed to act.
    Active,
    /// Registered but temporarily not allowed to act.
    Suspended,
    /// Permanently revoked; must never grant authority or disclose state.
    Revoked,
}

/// Resolved identity state for a registered principal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrincipalState {
    /// The canonical principal reference.
    #[allow(clippy::struct_field_names)]
    pub ref_: PrincipalRef,
    /// Unix timestamp (seconds) when the principal was registered.
    pub registered_at: i64,
    /// Current lifecycle status.
    pub status: PrincipalStatus,
}

/// Canonical registry of authenticated principals.
#[derive(Debug, Clone, Default)]
pub struct PrincipalManager {
    principals: HashMap<String, PrincipalState>,
}

impl PrincipalManager {
    /// Create an empty principal registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a principal as [`PrincipalStatus::Active`].
    ///
    /// Registration is idempotent-denied: a second registration of the same
    /// principal is a [`Error::DuplicatePrincipal`] error, never a silent
    /// overwrite.
    pub fn register_principal(&mut self, principal: PrincipalRef) -> Result<(), Error> {
        if self.principals.contains_key(&principal.0) {
            return Err(Error::DuplicatePrincipal(principal));
        }
        self.principals.insert(
            principal.0.clone(),
            PrincipalState {
                ref_: principal,
                registered_at: now_secs(),
                status: PrincipalStatus::Active,
            },
        );
        Ok(())
    }

    /// Resolve a principal to its current state.
    ///
    /// An unknown principal is an [`Error::UnknownPrincipal`] error; callers
    /// must treat it as an authentication failure (fail-closed).
    pub fn resolve_principal(&self, ref_: &PrincipalRef) -> Result<PrincipalState, Error> {
        self.principals
            .get(&ref_.0)
            .cloned()
            .ok_or_else(|| Error::UnknownPrincipal(ref_.clone()))
    }

    /// Update the lifecycle status of a registered principal.
    pub fn set_status(
        &mut self,
        ref_: &PrincipalRef,
        status: PrincipalStatus,
    ) -> Result<(), Error> {
        let state = self
            .principals
            .get_mut(&ref_.0)
            .ok_or_else(|| Error::UnknownPrincipal(ref_.clone()))?;
        state.status = status;
        Ok(())
    }
}

/// Current Unix time in seconds since the epoch.
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
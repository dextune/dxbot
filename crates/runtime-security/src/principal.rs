//! Principal registry and lifecycle.
//!
//! A [`Principal`](dxbot_core::types::PrincipalRef) is derived server-side from
//! the authenticated peer, never from client hints or domain IDs. The
//! [`PrincipalManager`] owns the canonical registry: registration, resolution,
//! and lifecycle status. Authority is never inferred from registration alone.

use std::collections::HashMap;

use dxbot_core::types::PrincipalRef;
use serde::{Deserialize, Serialize};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrincipalStatus {
    Active,
    Suspended,
    Revoked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalState {
    #[allow(clippy::struct_field_names)]
    pub ref_: PrincipalRef,
    pub registered_at: i64,
    pub status: PrincipalStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PrincipalManager {
    principals: HashMap<String, PrincipalState>,
}

impl PrincipalManager {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn resolve_principal(&self, ref_: &PrincipalRef) -> Result<PrincipalState, Error> {
        self.principals
            .get(&ref_.0)
            .cloned()
            .ok_or_else(|| Error::UnknownPrincipal(ref_.clone()))
    }

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

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

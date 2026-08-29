//! Interactive journey preflight (`AT-CLI-INTERACTIVE-001`).
//!
//! The CLI materializes domain selectors only through a bounded, non-intrusive
//! preflight: resolve a selector to a canonical id and its observed
//! [`CasConditions`], materialize a [`CanonicalTarget`], and check the
//! caller-supplied CAS against it. A stale conflict is <em>never</em>
//! auto-retried — it must be surfaced for the user to re-materialize
//! intentionally.
//!
//! This module only depends on `dxbot-core` and standard types; it is at the
//! CLI boundary and does not import application or control-client crates.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;

use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::{BotId, CanonicalTarget, CasConditions};

/// Capabilities a ready production provider must expose.
pub const REQUIRED_PROVIDER_CAPABILITIES: &[&str] = &["llm-chat", "embeddings", "auth"];

/// The result of a bounded selector preflight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightResult {
    /// The canonical id for an exact, unambiguous resolution; `None` when the
    /// selector is ambiguous and the caller must disambiguate.
    pub canonical_id: Option<String>,
    /// The CAS conditions observed at preflight time (the revision the caller
    /// must match to make the operation conflict-free).
    pub required_cas: CasConditions,
    /// Visible candidates for an ambiguous selector, so the user can choose.
    pub visible_candidates: Vec<String>,
}

/// The outcome of a CAS conflict check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictStatus {
    /// The caller-supplied CAS matches the materialized target.
    Clean,
    /// A stale conflict: the target moved to `current_revision` since the
    /// caller captured their CAS. Never auto-retried.
    Stale {
        current_revision: i64,
        current_generation: Option<i64>,
    },
}

/// Provider availability surfaced to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderStatus {
    /// Whether the provider is currently available on the resolved instance.
    pub available: bool,
    /// The capabilities the provider is required to expose.
    pub required_capabilities: Vec<String>,
    /// A human-readable diagnostic explaining availability.
    pub diagnostic: String,
}

/// Errors produced by the interactive journey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractiveError {
    /// No canonical candidate matched the selector.
    NoCandidate { selector: String },
    /// A stale conflict was detected; the operation must not be auto-retried.
    Conflict {
        current_revision: i64,
        current_generation: Option<i64>,
    },
}

impl InteractiveError {
    /// Projects this error onto the canonical [`DxbotError`] surface.
    pub fn to_dxbot_error(&self) -> DxbotError {
        match self {
            Self::NoCandidate { selector } => DxbotError {
                code: ErrorCode::NotFound,
                category: ErrorCategory::Input,
                message: format!("no canonical candidate for selector {selector:?}"),
                retryable: false,
                operation_ref: None,
                target_refs: Vec::new(),
                field_violations: Vec::new(),
                current_revision: None,
                current_generation: None,
                resume_cursor: None,
                next_actions: Vec::new(),
            },
            Self::Conflict {
                current_revision,
                current_generation,
            } => DxbotError {
                code: ErrorCode::Conflict,
                category: ErrorCategory::Conflict,
                message: format!("stale CAS conflict (current revision {current_revision})"),
                retryable: false,
                operation_ref: None,
                target_refs: Vec::new(),
                field_violations: Vec::new(),
                current_revision: Some(*current_revision),
                current_generation: *current_generation,
                resume_cursor: None,
                next_actions: Vec::new(),
            },
        }
    }
}

/// The interactive journey: preflight → materialize → conflict check.
///
/// `InteractiveJourney` carries a small representative catalog used by the
/// bounded (non-network) preflight. In production the same surface is backed
/// by the discovery/catalog resolution; here is kept pure and deterministic so
/// the acceptance behavior is fully testable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractiveJourney {
    /// Scoped-exact name → canonical id.
    exact: HashMap<String, String>,
    /// Ambiguous name → visible canonical candidates.
    ambiguous: HashMap<String, Vec<String>>,
    /// Whether the reference provider is available for the resolved instance.
    provider_available: bool,
    /// Capabilities the provider is required to expose.
    required_capabilities: Vec<String>,
}

impl Default for InteractiveJourney {
    fn default() -> Self {
        let mut exact = HashMap::new();
        exact.insert("alpha".to_string(), "bot-alpha".to_string());
        exact.insert("beta".to_string(), "bot-beta".to_string());
        let mut ambiguous = HashMap::new();
        ambiguous.insert(
            "shared".to_string(),
            vec!["bot-alpha".to_string(), "bot-beta".to_string()],
        );
        Self {
            exact,
            ambiguous,
            provider_available: false,
            required_capabilities: REQUIRED_PROVIDER_CAPABILITIES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl InteractiveJourney {
    /// Resolves a domain selector to its canonical id plus the CAS conditions
    /// observed at preflight time.
    ///
    /// An exact, unambiguous selector resolves to a single canonical id. An
    /// ambiguous selector returns the visible candidates without resolving, so
    /// the user can disambiguate before anything is materialized.
    pub fn preflight_selector(&self, selector: &str) -> Result<PreflightResult, InteractiveError> {
        if let Some(canonical) = self.exact.get(selector) {
            return Ok(PreflightResult {
                canonical_id: Some(canonical.clone()),
                required_cas: cas_for_revision(0),
                visible_candidates: Vec::new(),
            });
        }
        if let Some(candidates) = self.ambiguous.get(selector) {
            return Ok(PreflightResult {
                canonical_id: None,
                required_cas: cas_for_revision(0),
                visible_candidates: candidates.clone(),
            });
        }
        Err(InteractiveError::NoCandidate {
            selector: selector.to_string(),
        })
    }

    /// Materializes the canonical target for a preflight result.
    ///
    /// Resolving the canonical id yields a concrete [`CanonicalTarget`]. An
    /// unresolved (ambiguous) result cannot be materialized.
    pub fn materialize_target(
        &self,
        result: &PreflightResult,
    ) -> Result<CanonicalTarget, InteractiveError> {
        let Some(canonical) = &result.canonical_id else {
            return Err(InteractiveError::NoCandidate {
                selector: "ambiguous (no canonical id)".to_string(),
            });
        };
        // Representative bot target; the canonical id is the bot's canonical id.
        Ok(CanonicalTarget::Bot {
            id: BotId(canonical.clone()),
            revision: 0,
        })
    }

    /// Checks the caller-supplied CAS conditions against a materialized target.
    ///
    /// A mismatch reports [`ConflictStatus::Stale`]; it must never be retried
    /// automatically (see [`InteractiveJourney::execute_guarded`]).
    pub fn check_conflict(
        &self,
        target: &CanonicalTarget,
        cas: &CasConditions,
    ) -> Result<ConflictStatus, InteractiveError> {
        let current = target_revision(target);
        if let Some(expected) = cas.if_revision {
            if expected != current {
                return Ok(ConflictStatus::Stale {
                    current_revision: current,
                    current_generation: target_generation(target),
                });
            }
        }
        Ok(ConflictStatus::Clean)
    }

    /// Runs the full guarded path once: preflight → materialize → conflict
    /// check.
    ///
    /// On a stale conflict this returns an error <em>without retrying</em>.
    /// The caller must observe the conflict and intentionally re-materialize —
    /// automatic retry is forbidden at the CLI boundary.
    pub fn execute_guarded(
        &self,
        selector: &str,
        cas: &CasConditions,
    ) -> Result<ConflictStatus, InteractiveError> {
        let preflight = self.preflight_selector(selector)?;
        let target = self.materialize_target(&preflight)?;
        match self.check_conflict(&target, cas)? {
            ConflictStatus::Clean => Ok(ConflictStatus::Clean),
            ConflictStatus::Stale {
                current_revision,
                current_generation,
            } => Err(InteractiveError::Conflict {
                current_revision,
                current_generation,
            }),
        }
    }

    /// Reports provider availability.
    pub fn provider_status(&self) -> ProviderStatus {
        let diagnostic = if self.provider_available {
            "provider available".to_string()
        } else {
            "provider unavailable: no ready reference provider found".to_string()
        };
        ProviderStatus {
            available: self.provider_available,
            required_capabilities: self.required_capabilities.clone(),
            diagnostic,
        }
    }
}

/// Builds CAS conditions asserting the given revision, for use as the
/// preflight-observed condition.
fn cas_for_revision(revision: i64) -> CasConditions {
    CasConditions {
        if_revision: Some(revision),
        if_generation: None,
        if_host_generation: None,
        if_execution_generation: None,
        if_source_revision: None,
        if_scope_revision: None,
        if_project_revision: None,
        if_channel_revision: None,
        if_membership_generation: None,
        if_proposal_revision: None,
        if_target_scope_revision: None,
        if_receipt_revision: None,
    }
}

/// The current revision of a materialized target.
fn target_revision(target: &CanonicalTarget) -> i64 {
    match target {
        CanonicalTarget::Instance(_) => 0,
        CanonicalTarget::Bot { revision, .. } => *revision,
        CanonicalTarget::Conversation { revision, .. } => *revision,
        CanonicalTarget::Thread { revision, .. } => *revision,
        CanonicalTarget::Task { revision, .. } => *revision,
        CanonicalTarget::Project { revision, .. } => *revision,
        CanonicalTarget::Channel { revision, .. } => *revision,
        CanonicalTarget::Operation { .. } => 0,
        CanonicalTarget::Approval { revision, .. } => *revision,
        CanonicalTarget::Memory { revision, .. } => *revision,
        CanonicalTarget::Provider { .. } => 0,
        CanonicalTarget::Process { .. } => 0,
        CanonicalTarget::SideEffect { revision, .. } => *revision,
        CanonicalTarget::Membership { .. } => 0,
    }
}

/// The current generation of a materialized target, if it has one.
fn target_generation(target: &CanonicalTarget) -> Option<i64> {
    match target {
        CanonicalTarget::Task {
            execution_generation,
            ..
        } => *execution_generation,
        CanonicalTarget::Provider { generation, .. } => {
            if *generation == 0 {
                None
            } else {
                Some(*generation)
            }
        }
        _ => None,
    }
}

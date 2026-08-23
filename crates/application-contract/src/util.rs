//! Internal error construction and small shared helpers.
//!
//! These are crate-internal (`pub(crate)`) and deliberately thin: they reuse the canonical
//! `DxbotError` type from `dxbot-core` and only fill the stable semantic fields that the
//! application-contract layer is responsible for.

use dxbot_core::error::{ErrorCategory, ErrorCode};
use dxbot_core::types::CasConditions;
use dxbot_core::DxbotError;

fn err(code: ErrorCode, category: ErrorCategory, message: String) -> DxbotError {
    DxbotError {
        code,
        category,
        message,
        retryable: false,
        operation_ref: None,
        target_refs: Vec::new(),
        field_violations: Vec::new(),
        current_revision: None,
        current_generation: None,
        resume_cursor: None,
        next_actions: Vec::new(),
    }
}

/// An input/usage error whose meaning is "the user-supplied CLI input is invalid".
pub(crate) fn input_error(message: impl Into<String>) -> DxbotError {
    err(ErrorCode::InvalidInput, ErrorCategory::Input, message.into())
}

/// An internal invariant violation (should never happen on well-formed inputs).
pub(crate) fn invariant(message: impl Into<String>) -> DxbotError {
    err(ErrorCode::InternalInvariant, ErrorCategory::Internal, message.into())
}

/// The fully-empty CAS condition: no optimistic-concurrency guard requested.
pub(crate) fn empty_cas() -> CasConditions {
    CasConditions {
        if_revision: None,
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
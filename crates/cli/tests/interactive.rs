//! Acceptance coverage for `AT-CLI-INTERACTIVE-001`: the interactive journey.
//!
//! Verifies bounded selector preflight, canonical materialization, CAS conflict
//! detection, provider status, and the no-auto-retry rule on conflict.
#![allow(clippy::unwrap_used)]

use cli::interactive::{ConflictStatus, InteractiveError, InteractiveJourney};
use dxbot_core::types::{BotId, CanonicalTarget, CasConditions};

fn journey() -> InteractiveJourney {
    InteractiveJourney::default()
}

/// Builds CAS conditions asserting only `if_revision`.
fn cas(if_revision: Option<i64>) -> CasConditions {
    CasConditions {
        if_revision,
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

#[test]
fn interactive_preflight_resolves_bot_selector_to_canonical_id() {
    let result = journey().preflight_selector("alpha").unwrap();
    // Scoped-exact (unambiguous) selector resolves to a single canonical id.
    assert_eq!(result.canonical_id.as_deref(), Some("bot-alpha"));
    assert!(result.visible_candidates.is_empty());
    // Preflight supplies the observed CAS conditions.
    assert_eq!(result.required_cas.if_revision, Some(0));
}

#[test]
fn interactive_preflight_returns_candidates_for_ambiguous() {
    let result = journey().preflight_selector("shared").unwrap();
    // An ambiguous selector returns candidates instead of resolving.
    assert_eq!(result.canonical_id, None);
    assert_eq!(result.visible_candidates, vec!["bot-alpha", "bot-beta"]);

    // An unknown selector fails without candidates.
    assert!(matches!(
        journey().preflight_selector("nope"),
        Err(InteractiveError::NoCandidate { .. })
    ));
}

#[test]
fn interactive_conflict_detection_reports_stale_revision() {
    let target = CanonicalTarget::Bot {
        id: BotId("bot-alpha".to_string()),
        revision: 5,
    };
    let stale_cas = cas(Some(3));
    let status = journey().check_conflict(&target, &stale_cas).unwrap();
    match status {
        ConflictStatus::Stale {
            current_revision,
            current_generation,
        } => {
            assert_eq!(current_revision, 5);
            assert_eq!(current_generation, None);
        }
        other => panic!("expected Stale conflict, got {other:?}"),
    }

    // Matching CAS is clean.
    let clean_cas = cas(Some(5));
    assert_eq!(
        journey().check_conflict(&target, &clean_cas).unwrap(),
        ConflictStatus::Clean
    );
}

#[test]
fn interactive_provider_status_reports_unavailable() {
    let status = journey().provider_status();
    // The reference provider is unavailable on the default journey.
    assert!(!status.available);
    assert!(
        !status.required_capabilities.is_empty(),
        "required capabilities must be reported"
    );
    assert!(
        status.diagnostic.to_lowercase().contains("unavailable"),
        "diagnostic must explain unavailability: {}",
        status.diagnostic
    );
}

#[test]
fn interactive_no_auto_retry_on_conflict() {
    let journey = journey();
    // The caller captured a revision that no longer matches the materialized
    // target (revision 0), so the guarded path surfaces the conflict.
    let stale_cas = cas(Some(7));

    // A stale conflict is surfaced as a conflict error — it is never silently
    // retried to a clean state and never auto-resolved.
    match journey.execute_guarded("alpha", &stale_cas) {
        Err(InteractiveError::Conflict {
            current_revision, ..
        }) => {
            assert_eq!(current_revision, 0);
        }
        other => panic!("expected Conflict (no auto-retry), got {other:?}"),
    }

    // The no-auto-retry rule is also reflected in the projected error surface:
    // a conflict is non-retryable.
    let err = InteractiveError::Conflict {
        current_revision: 0,
        current_generation: None,
    };
    assert!(!err.to_dxbot_error().retryable);

    // A clean CAS still proceeds without conflict.
    let clean_cas = cas(Some(0));
    assert_eq!(
        journey.execute_guarded("alpha", &clean_cas).unwrap(),
        ConflictStatus::Clean
    );
}
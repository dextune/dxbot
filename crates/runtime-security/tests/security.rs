//! Acceptance tests for AT-SECSTATE-001: Principal, Approval, Authority, and
//! pending operation continuation.
//!
//! All covered in-process against the in-memory managers (no external I/O).

use std::error::Error;

use dxbot_core::types::{
    ApprovalId, BotId, BotSelector, OperationId, PrincipalRef, ProjectId, ProjectSelector,
    ScopeSelector,
};
use runtime_security::{
    ApprovalDecision, ApprovalManager, ApprovalState, AuthorityManager, Error as SecurityError,
    PrincipalManager, PrincipalStatus,
};

#[test]
fn principal_registration_and_resolution() -> Result<(), Box<dyn Error>> {
    let mut manager = PrincipalManager::new();
    let alice = PrincipalRef("http:alice".into());

    manager.register_principal(alice.clone())?;

    let state = manager.resolve_principal(&alice)?;
    assert_eq!(state.ref_, alice);
    assert!(
        state.registered_at > 0,
        "registration timestamp must be populated"
    );
    assert_eq!(state.status, PrincipalStatus::Active);

    // Duplicate registration is rejected, never a silent overwrite.
    assert!(matches!(
        manager.register_principal(PrincipalRef("http:alice".into())),
        Err(SecurityError::DuplicatePrincipal(_))
    ));

    // Unknown principals fail resolution.
    assert!(manager
        .resolve_principal(&PrincipalRef("http:stranger".into()))
        .is_err());
    Ok(())
}

#[test]
fn approval_lifecycle_pending_to_approved() -> Result<(), Box<dyn Error>> {
    let mut manager = ApprovalManager::new();
    let a = PrincipalRef("http:approver-a".into());
    let b = PrincipalRef("http:approver-b".into());
    let operation = OperationId("op-flagged".into());

    let id = manager.create_approval(operation.clone(), vec![a.clone(), b.clone()])?;
    assert_eq!(manager.get_approval(&id)?.state, ApprovalState::Pending);
    // While pending, the guarded operation must not continue.
    assert_eq!(manager.continuation_ready(&id)?, None);

    // Partial approval keeps it pending.
    manager.decide_approval(&id, ApprovalDecision::Approve, &a)?;
    assert_eq!(manager.get_approval(&id)?.state, ApprovalState::Pending);

    // Full approval transitions to Approved and unlocks continuation.
    let state = manager.decide_approval(&id, ApprovalDecision::Approve, &b)?;
    assert_eq!(state, ApprovalState::Approved);
    assert_eq!(manager.continuation_ready(&id)?, Some(operation));
    Ok(())
}

#[test]
fn approval_lifecycle_pending_to_denied() -> Result<(), Box<dyn Error>> {
    let mut manager = ApprovalManager::new();
    let a = PrincipalRef("http:approver-a".into());
    let b = PrincipalRef("http:approver-b".into());
    let operation = OperationId("op-blocked".into());

    let id = manager.create_approval(operation, vec![a.clone(), b.clone()])?;

    // A single denial denies the approval regardless of later approvals.
    manager.decide_approval(&id, ApprovalDecision::Deny, &a)?;
    assert_eq!(manager.get_approval(&id)?.state, ApprovalState::Denied);

    // Re-deciding an already-decided approval is rejected.
    assert!(matches!(
        manager.decide_approval(&id, ApprovalDecision::Approve, &b),
        Err(SecurityError::ApprovalAlreadyDecided(_))
    ));

    // A denied approval never unlocks continuation.
    assert_eq!(manager.continuation_ready(&id)?, None);
    Ok(())
}

#[test]
fn authority_binding_and_check() -> Result<(), Box<dyn Error>> {
    let mut manager = AuthorityManager::new();
    let alice = PrincipalRef("http:alice".into());
    let scope = ScopeSelector::Bot(BotSelector::CanonicalId(BotId("bot-1".into())));

    manager.bind_authority(&alice, &scope, "mutator")?;

    assert!(manager.check_authority(&alice, &scope, "mutator")?);
    // A role that was not granted is denied.
    assert!(!manager.check_authority(&alice, &scope, "viewer")?);
    // A different scope is denied (deny-unknown).
    let other_scope = ScopeSelector::Bot(BotSelector::CanonicalId(BotId("bot-2".into())));
    assert!(!manager.check_authority(&alice, &other_scope, "mutator")?);
    Ok(())
}

#[test]
fn authority_revocation_denies_access() -> Result<(), Box<dyn Error>> {
    let mut manager = AuthorityManager::new();
    let alice = PrincipalRef("http:alice".into());
    let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId("proj-1".into())));

    manager.bind_authority(&alice, &scope, "mutator")?;
    assert!(manager.check_authority(&alice, &scope, "mutator")?);

    manager.revoke_authority(&alice, &scope)?;
    assert!(!manager.check_authority(&alice, &scope, "mutator")?);

    // Revoking an already-revoked binding is a safe no-op.
    manager.revoke_authority(&alice, &scope)?;
    assert!(!manager.check_authority(&alice, &scope, "mutator")?);
    Ok(())
}

#[test]
fn pending_operation_continuation_after_approval() -> Result<(), Box<dyn Error>> {
    let mut manager = ApprovalManager::new();
    let a = PrincipalRef("http:approver-a".into());
    let b = PrincipalRef("http:approver-b".into());
    let operation = OperationId("op-critical".into());
    let id: ApprovalId =
        manager.create_approval(operation.clone(), vec![a.clone(), b.clone()])?;

    // The operation is parked behind the pending approval: no continuation.
    assert_eq!(manager.continuation_ready(&id)?, None);

    manager.decide_approval(&id, ApprovalDecision::Approve, &a)?;
    assert_eq!(manager.continuation_ready(&id)?, None);

    // Once every required approver approves, the pending operation may continue.
    manager.decide_approval(&id, ApprovalDecision::Approve, &b)?;
    assert_eq!(manager.continuation_ready(&id)?, Some(operation));
    Ok(())
}
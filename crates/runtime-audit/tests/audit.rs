//! Acceptance tests for `AT-AUDIT-001` (required audit intent atomicity and
//! redacted observation).
//!
//! `cargo test -p runtime-audit` runs every test below.

use dxbot_core::types::{OperationId, PrincipalRef};
use runtime_audit::{
    AuditFilter, AuditLogger, Error as AuditError, EventType, RedactionLevel, SecurityEvent,
};

// ── fixtures ──

fn op(id: &str) -> OperationId {
    OperationId(id.to_owned())
}

fn principal(name: &str) -> PrincipalRef {
    PrincipalRef(name.to_owned())
}

fn logger() -> AuditLogger {
    AuditLogger::new()
}

// ── atomicity ──

#[test]
fn audit_operation_is_recorded_atomically() {
    let audit = logger();
    let operation_id = op("operation-1");

    let record = audit
        .log_operation(&operation_id, "command: bot.create")
        .expect("operation audit intent must record");

    // The record returned is the same one committed to the log.
    assert_eq!(record.operation_id, operation_id);
    assert_eq!(record.event_type, EventType::Operation);
    assert_eq!(record.payload, "command: bot.create");
    assert_eq!(record.id, "audit-0000000000000000");

    // Exactly one atomic append — no partial or duplicated records.
    assert_eq!(audit.record_count().expect("record count must succeed"), 1);
    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].operation_id, operation_id);
    assert_eq!(observed[0].payload, "command: bot.create");
    assert_eq!(observed[0].redaction_level, RedactionLevel::None);
}

// ── security events ──

#[test]
fn audit_security_event_is_recorded() {
    let audit = logger();

    let auth_ok = audit
        .log_security_event(SecurityEvent::AuthenticationSuccess)
        .expect("security event must record");
    let denied = audit
        .log_security_event(SecurityEvent::AuthorizationDenied)
        .expect("security event must record");

    assert_eq!(auth_ok.event_type, EventType::Security(SecurityEvent::AuthenticationSuccess));
    assert_eq!(
        denied.event_type,
        EventType::Security(SecurityEvent::AuthorizationDenied)
    );

    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 2);
    assert_eq!(observed[0].event_type, auth_ok.event_type);
    assert_eq!(observed[1].event_type, denied.event_type);
}

// ── filtering ──

#[test]
fn audit_query_filters_by_principal() {
    let audit = logger();

    audit
        .log_operation_as(&principal("alice"), &op("operation-a"), "command: a")
        .expect("record alice");
    audit
        .log_operation_as(&principal("bob"), &op("operation-b"), "command: b")
        .expect("record bob");
    audit
        .log_operation_as(&principal("alice"), &op("operation-c"), "command: c")
        .expect("record alice again");

    let filter = AuditFilter {
        principal_ref: Some(principal("alice")),
        ..Default::default()
    };
    let observed = audit
        .query_audit_log(&filter)
        .expect("query must succeed");

    assert_eq!(observed.len(), 2);
    for record in &observed {
        assert_eq!(record.principal_ref, principal("alice"));
    }
    assert_eq!(observed[0].operation_id, op("operation-a"));
    assert_eq!(observed[1].operation_id, op("operation-c"));
}

#[test]
fn audit_query_filters_by_event_type() {
    let audit = logger();

    audit
        .log_security_event(SecurityEvent::AuthenticationSuccess)
        .expect("record auth success");
    audit
        .log_security_event(SecurityEvent::AuthorizationDenied)
        .expect("record denial");
    audit.log_operation(&op("operation-1"), "command: read").expect("record operation");

    // Filter to a single security event kind.
    let filter = AuditFilter {
        event_type: Some(EventType::Security(SecurityEvent::AuthorizationDenied)),
        ..Default::default()
    };
    let observed = audit
        .query_audit_log(&filter)
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].event_type,
        EventType::Security(SecurityEvent::AuthorizationDenied)
    );

    // Filter to audit intents only.
    let filter = AuditFilter {
        event_type: Some(EventType::Operation),
        ..Default::default()
    };
    let observed = audit
        .query_audit_log(&filter)
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].event_type, EventType::Operation);
    assert_eq!(observed[0].operation_id, op("operation-1"));
}

// ── redacted observation ──

#[test]
fn audit_redacted_observation_hides_secrets() {
    let audit = logger();

    audit
        .log_operation(
            &op("operation-secret"),
            "password=supersecret deploy token=abc123 host=prod",
        )
        .expect("record secret-bearing operation");

    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);

    let record = &observed[0];
    // Redaction covered the secret tokens.
    assert!(!record.payload.contains("supersecret"), "secret value leaked: {}", record.payload);
    assert!(!record.payload.contains("abc123"), "token value leaked: {}", record.payload);
    // Non-secret content is preserved.
    assert!(record.payload.contains("host=prod"), "non-secret content lost: {}", record.payload);
    // The observation carries the redaction marker and classification.
    assert_eq!(record.redaction_level, RedactionLevel::Partial);

    // A fully secret payload is hidden entirely.
    audit
        .log_operation(&op("operation-only-secret"), "api_secret=s3cr3t")
        .expect("record fully secret operation");
    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    let full = &observed[1];
    assert_eq!(full.payload, "[REDACTED]");
    assert_eq!(full.redaction_level, RedactionLevel::Full);
}

#[test]
fn audit_redacted_observation_preserves_non_secret_fields() {
    let audit = logger();

    let operation_id = op("operation-clean");
    audit
        .log_operation_as(
            &principal("carol"),
            &operation_id,
            "command: bot.update note=clear-safe",
        )
        .expect("record clean operation");

    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);

    let record = &observed[0];
    // No secret-like tokens -> no redaction was applied.
    assert_eq!(record.payload, "command: bot.update note=clear-safe");
    assert_eq!(record.redaction_level, RedactionLevel::None);

    // Every non-secret identifying field survives verbatim.
    assert_eq!(record.operation_id, operation_id);
    assert_eq!(record.principal_ref, principal("carol"));
    assert_eq!(record.event_type, EventType::Operation);
    assert!(!record.id.is_empty());
    assert!(record.timestamp > 0, "timestamp must be present");
}

// ── fail closed on corrupt log ──

#[test]
fn audit_fail_closed_on_corrupt_log() {
    let audit = logger();

    audit
        .log_operation(&op("operation-1"), "command: read")
        .expect("record before corruption");

    // Simulate a corrupted/truncated journal via the fault-injection seam.
    audit.mark_corrupt();

    // Queries fail closed: no partial or unverifiable records are served.
    let error = audit
        .query_audit_log(&AuditFilter::default())
        .expect_err("querying a corrupt log must fail closed");
    assert_eq!(error, AuditError::CorruptLog);

    // Appends also fail closed.
    let error = audit
        .log_operation(&op("operation-2"), "command: write")
        .expect_err("appending to a corrupt log must fail closed");
    assert_eq!(error, AuditError::CorruptLog);
}
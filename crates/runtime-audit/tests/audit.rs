//! Acceptance tests for `AT-AUDIT-001`: required audit intent atomicity and
//! write-time redacted observation.

use std::sync::Arc;

use dxbot_core::types::{OperationId, PrincipalRef};
use runtime_audit::{
    AuditFilter, AuditLogger, Error as AuditError, EventType, RedactionLevel, SecurityEvent,
};

fn op(id: &str) -> OperationId {
    OperationId(id.to_owned())
}

fn principal(name: &str) -> PrincipalRef {
    PrincipalRef(name.to_owned())
}

fn logger() -> AuditLogger {
    AuditLogger::new()
}

#[test]
fn audit_operation_is_recorded_atomically() {
    let audit = logger();
    let operation_id = op("operation-1");

    let record = audit
        .log_operation(&operation_id, "command: bot.create")
        .expect("operation audit intent must record");

    assert_eq!(record.operation_id, operation_id);
    assert_eq!(record.event_type, EventType::Operation);
    assert_eq!(record.payload, "command: bot.create");
    assert_eq!(record.id, "audit-0000000000000000");

    assert_eq!(audit.record_count().expect("record count must succeed"), 1);
    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].operation_id, operation_id);
    assert_eq!(observed[0].payload, "command: bot.create");
    assert_eq!(observed[0].redaction_level, RedactionLevel::None);
}

#[test]
fn audit_ids_follow_atomic_append_order_under_concurrency() {
    let audit = Arc::new(logger());
    let mut workers = Vec::new();
    for index in 0..32 {
        let audit = Arc::clone(&audit);
        workers.push(std::thread::spawn(move || {
            audit
                .log_operation(&op(&format!("operation-{index}")), "command: concurrent")
                .expect("concurrent audit append")
        }));
    }
    for worker in workers {
        worker.join().expect("audit worker panicked");
    }

    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query concurrent audit log");
    assert_eq!(observed.len(), 32);
    for (index, record) in observed.iter().enumerate() {
        assert_eq!(record.id, format!("audit-{index:016x}"));
    }
}

#[test]
fn audit_security_event_is_recorded() {
    let audit = logger();

    let auth_ok = audit
        .log_security_event(SecurityEvent::AuthenticationSuccess)
        .expect("security event must record");
    let denied = audit
        .log_security_event(SecurityEvent::AuthorizationDenied)
        .expect("security event must record");

    assert_eq!(
        auth_ok.event_type,
        EventType::Security(SecurityEvent::AuthenticationSuccess)
    );
    assert_eq!(
        denied.event_type,
        EventType::Security(SecurityEvent::AuthorizationDenied)
    );

    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 2);
}

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
    let observed = audit.query_audit_log(&filter).expect("query must succeed");

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
    audit
        .log_operation(&op("operation-1"), "command: read")
        .expect("record operation");

    let filter = AuditFilter {
        event_type: Some(EventType::Security(SecurityEvent::AuthorizationDenied)),
        ..Default::default()
    };
    let observed = audit.query_audit_log(&filter).expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(
        observed[0].event_type,
        EventType::Security(SecurityEvent::AuthorizationDenied)
    );

    let filter = AuditFilter {
        event_type: Some(EventType::Operation),
        ..Default::default()
    };
    let observed = audit.query_audit_log(&filter).expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].operation_id, op("operation-1"));
}

#[test]
fn audit_rejects_inverted_time_range() {
    let audit = logger();
    let filter = AuditFilter {
        time_range: Some((10, 9)),
        ..Default::default()
    };
    assert!(matches!(
        audit.query_audit_log(&filter),
        Err(AuditError::InvalidInput(_))
    ));
}

#[test]
fn audit_redacts_before_storage_and_return() {
    let audit = logger();

    let committed = audit
        .log_operation(
            &op("operation-secret"),
            "password=supersecret deploy token=abc123 host=prod",
        )
        .expect("record secret-bearing operation");
    assert!(!committed.payload.contains("supersecret"));
    assert!(!committed.payload.contains("abc123"));
    assert!(committed.payload.contains("host=prod"));
    assert_eq!(committed.redaction_level, RedactionLevel::Partial);

    let observed = audit
        .query_audit_log(&AuditFilter::default())
        .expect("query must succeed");
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].payload, committed.payload);

    let full = audit
        .log_operation(&op("operation-only-secret"), "api_secret=s3cr3t")
        .expect("record fully secret operation");
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
    assert_eq!(record.payload, "command: bot.update note=clear-safe");
    assert_eq!(record.redaction_level, RedactionLevel::None);
    assert_eq!(record.operation_id, operation_id);
    assert_eq!(record.principal_ref, principal("carol"));
    assert_eq!(record.event_type, EventType::Operation);
    assert!(!record.id.is_empty());
    assert!(record.timestamp > 0, "timestamp must be present");
}

#[test]
fn audit_fail_closed_on_corrupt_log() {
    let audit = logger();

    audit
        .log_operation(&op("operation-1"), "command: read")
        .expect("record before corruption");

    audit.mark_corrupt();

    assert_eq!(
        audit
            .query_audit_log(&AuditFilter::default())
            .expect_err("querying a corrupt log must fail closed"),
        AuditError::CorruptLog
    );
    assert_eq!(
        audit
            .record_count()
            .expect_err("counting a corrupt log must fail closed"),
        AuditError::CorruptLog
    );
    assert_eq!(
        audit
            .log_operation(&op("operation-2"), "command: write")
            .expect_err("appending to a corrupt log must fail closed"),
        AuditError::CorruptLog
    );
}

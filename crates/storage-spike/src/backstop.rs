//! Bounded recovery backstop proof owned by the canonical storage boundary.
//!
//! This is not an automatic restore path and not a second canonical store.
//! Capture is a consistent, bounded, content-digested recovery cache. Restore
//! is exposed only as dry-run planning; apply authority remains outside this
//! isolated storage proof.

use std::fmt;

use runtime_audit::{
    DiagnosticComponent, DiagnosticFamily, DiagnosticReason, DiagnosticSeverity, DiagnosticSink,
    SafeAttribute, SafeAttributeKey,
};
use rusqlite::TransactionBehavior;
use sha2::{Digest, Sha256};

use crate::schema::verify_writer_fence;
use crate::{ReferenceStore, SpikeError};

const MAX_BACKSTOP_ITEMS: usize = 2_048;
const MAX_BACKSTOP_BYTES: usize = 256 * 1024;
const REVISION_ACCOUNTING_BYTES: usize = std::mem::size_of::<i64>();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackstopBudget {
    pub max_items: usize,
    pub max_bytes: usize,
    pub min_interval_secs: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackstopAggregate {
    pub aggregate_id: String,
    pub revision: i64,
    pub state_value: String,
    pub last_operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackstopSnapshot {
    pub snapshot_id: String,
    pub source_watermark: i64,
    pub schema_version: i64,
    pub captured_at: i64,
    pub content_digest: String,
    pub retained_bytes: usize,
    pub aggregates: Vec<BackstopAggregate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackstopCaptureOutcome {
    Captured(BackstopSnapshot),
    SkippedNoChange,
    SkippedDebounced,
    SkippedOverLimit,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackstopRestorePlan {
    pub snapshot_id: String,
    pub source_watermark: i64,
    pub aggregate_count: usize,
    pub requires_authorized_apply: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackstopError {
    Storage(String),
    InvalidBudget,
    SinkFailed,
    SchemaMismatch { snapshot: i64, current: i64 },
}

impl fmt::Display for BackstopError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "backstop storage error: {message}"),
            Self::InvalidBudget => formatter.write_str("invalid backstop budget"),
            Self::SinkFailed => formatter.write_str("backstop sink failed"),
            Self::SchemaMismatch { snapshot, current } => {
                write!(
                    formatter,
                    "backstop schema mismatch: snapshot={snapshot}, current={current}"
                )
            }
        }
    }
}

impl std::error::Error for BackstopError {}

impl From<SpikeError> for BackstopError {
    fn from(error: SpikeError) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<rusqlite::Error> for BackstopError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

pub trait BackstopSink: fmt::Debug {
    fn write_snapshot(&mut self, snapshot: &BackstopSnapshot) -> Result<(), BackstopError>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryBackstopSink {
    snapshots: Vec<BackstopSnapshot>,
    fail_writes: bool,
}

impl MemoryBackstopSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_fail_writes(&mut self, fail_writes: bool) {
        self.fail_writes = fail_writes;
    }

    pub fn snapshots(&self) -> &[BackstopSnapshot] {
        &self.snapshots
    }
}

impl BackstopSink for MemoryBackstopSink {
    fn write_snapshot(&mut self, snapshot: &BackstopSnapshot) -> Result<(), BackstopError> {
        if self.fail_writes {
            return Err(BackstopError::SinkFailed);
        }
        self.snapshots.push(snapshot.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineReason {
    SchemaMismatch,
    DigestMismatch,
    OperatorHold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantinedBackstop {
    pub snapshot_id: String,
    pub content_digest: String,
    pub reason: QuarantineReason,
}

#[derive(Debug, Clone, Default)]
pub struct QuarantineRegistry {
    entries: Vec<QuarantinedBackstop>,
}

impl QuarantineRegistry {
    pub fn quarantine(&mut self, snapshot: &BackstopSnapshot, reason: QuarantineReason) {
        self.entries.push(QuarantinedBackstop {
            snapshot_id: snapshot.snapshot_id.clone(),
            content_digest: snapshot.content_digest.clone(),
            reason,
        });
    }

    pub fn entries(&self) -> &[QuarantinedBackstop] {
        &self.entries
    }
}

#[derive(Debug, Clone, Default)]
pub struct BackstopCoordinator {
    last_source_watermark: Option<i64>,
    last_capture_at: Option<i64>,
    stopped: bool,
    diagnostics: Option<DiagnosticSink>,
}

impl BackstopCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_diagnostics(mut self, diagnostics: DiagnosticSink) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    pub fn stop(&mut self) {
        self.stopped = true;
    }

    pub fn capture_to_sink(
        &mut self,
        store: &mut ReferenceStore,
        budget: BackstopBudget,
        now: i64,
        sink: &mut dyn BackstopSink,
    ) -> Result<BackstopCaptureOutcome, BackstopError> {
        let outcome = self.capture(store, budget, now)?;
        if let BackstopCaptureOutcome::Captured(snapshot) = &outcome {
            if sink.write_snapshot(snapshot).is_err() {
                self.emit(
                    DiagnosticReason::SnapshotSinkFailed,
                    DiagnosticSeverity::Error,
                    Some(&snapshot.snapshot_id),
                );
                return Err(BackstopError::SinkFailed);
            }
        }
        Ok(outcome)
    }

    pub fn capture(
        &mut self,
        store: &mut ReferenceStore,
        budget: BackstopBudget,
        now: i64,
    ) -> Result<BackstopCaptureOutcome, BackstopError> {
        validate_budget(budget)?;
        if self.stopped {
            return Ok(BackstopCaptureOutcome::Stopped);
        }
        if self
            .last_capture_at
            .is_some_and(|captured_at| now.saturating_sub(captured_at) < budget.min_interval_secs)
        {
            return Ok(BackstopCaptureOutcome::SkippedDebounced);
        }

        let instance_id = store.instance_id.clone();
        let host_generation = store.host_generation;
        let transaction = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)?;
        verify_writer_fence(&transaction, &instance_id, host_generation)?;

        let source_watermark: i64 =
            transaction.query_row("SELECT COALESCE(MAX(sequence), 0) FROM events", [], |row| {
                row.get(0)
            })?;
        if self.last_source_watermark == Some(source_watermark) {
            transaction.commit()?;
            self.last_capture_at = Some(now);
            self.emit(
                DiagnosticReason::SnapshotNoChange,
                DiagnosticSeverity::Info,
                None,
            );
            return Ok(BackstopCaptureOutcome::SkippedNoChange);
        }

        let schema_version: i64 =
            transaction.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let (item_count, estimated_bytes): (i64, i64) = transaction.query_row(
            "SELECT COUNT(*), COALESCE(SUM(\
                 length(aggregate_id) + length(state_value) + length(last_operation_id) + 8\
             ), 0) FROM aggregate_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if item_count < 0 || estimated_bytes < 0 {
            return Err(BackstopError::Storage(
                "negative backstop preflight accounting".to_owned(),
            ));
        }
        let item_count = usize::try_from(item_count).map_err(|_| BackstopError::InvalidBudget)?;
        let estimated_bytes =
            usize::try_from(estimated_bytes).map_err(|_| BackstopError::InvalidBudget)?;
        if item_count > budget.max_items || estimated_bytes > budget.max_bytes {
            transaction.commit()?;
            self.last_capture_at = Some(now);
            self.emit(
                DiagnosticReason::SnapshotOverLimit,
                DiagnosticSeverity::Warning,
                None,
            );
            return Ok(BackstopCaptureOutcome::SkippedOverLimit);
        }

        let mut statement = transaction.prepare(
            "SELECT aggregate_id, revision, state_value, last_operation_id \
             FROM aggregate_state ORDER BY aggregate_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(BackstopAggregate {
                aggregate_id: row.get(0)?,
                revision: row.get(1)?,
                state_value: row.get(2)?,
                last_operation_id: row.get(3)?,
            })
        })?;
        let mut aggregates = Vec::with_capacity(item_count);
        let mut retained_bytes = 0_usize;
        for row in rows {
            let aggregate = row?;
            retained_bytes = retained_bytes
                .saturating_add(aggregate.aggregate_id.len())
                .saturating_add(aggregate.state_value.len())
                .saturating_add(aggregate.last_operation_id.len())
                .saturating_add(REVISION_ACCOUNTING_BYTES);
            if retained_bytes > budget.max_bytes || aggregates.len() >= budget.max_items {
                return Err(BackstopError::Storage(
                    "backstop changed beyond preflight budget during snapshot".to_owned(),
                ));
            }
            aggregates.push(aggregate);
        }
        drop(statement);

        let content_digest = digest_snapshot(schema_version, source_watermark, &aggregates);
        let digest_prefix = content_digest.get(..16).unwrap_or(&content_digest);
        let snapshot = BackstopSnapshot {
            snapshot_id: format!("backstop-{source_watermark}-{now}-{digest_prefix}"),
            source_watermark,
            schema_version,
            captured_at: now,
            content_digest,
            retained_bytes,
            aggregates,
        };
        transaction.commit()?;

        self.last_source_watermark = Some(source_watermark);
        self.last_capture_at = Some(now);
        self.emit(
            DiagnosticReason::SnapshotCaptured,
            DiagnosticSeverity::Info,
            Some(&snapshot.snapshot_id),
        );
        Ok(BackstopCaptureOutcome::Captured(snapshot))
    }

    fn emit(
        &self,
        reason: DiagnosticReason,
        severity: DiagnosticSeverity,
        snapshot_id: Option<&str>,
    ) {
        let Some(sink) = &self.diagnostics else {
            return;
        };
        let attributes = snapshot_id
            .map(|id| vec![SafeAttribute::new(SafeAttributeKey::ResourceKind, id)])
            .unwrap_or_default();
        let _ = sink.emit(
            DiagnosticComponent::Storage,
            "recovery-backstop",
            DiagnosticFamily::StorageRecovery,
            reason,
            severity,
            &attributes,
        );
    }
}

impl BackstopSnapshot {
    /// Produces a bounded restore plan only. No state is applied here.
    pub fn dry_run_restore(
        &self,
        current_schema: i64,
    ) -> Result<BackstopRestorePlan, BackstopError> {
        if self.schema_version != current_schema {
            return Err(BackstopError::SchemaMismatch {
                snapshot: self.schema_version,
                current: current_schema,
            });
        }
        Ok(BackstopRestorePlan {
            snapshot_id: self.snapshot_id.clone(),
            source_watermark: self.source_watermark,
            aggregate_count: self.aggregates.len(),
            requires_authorized_apply: true,
        })
    }
}

fn validate_budget(budget: BackstopBudget) -> Result<(), BackstopError> {
    if budget.max_items == 0
        || budget.max_items > MAX_BACKSTOP_ITEMS
        || budget.max_bytes == 0
        || budget.max_bytes > MAX_BACKSTOP_BYTES
        || budget.min_interval_secs < 0
    {
        return Err(BackstopError::InvalidBudget);
    }
    Ok(())
}

fn digest_snapshot(
    schema_version: i64,
    source_watermark: i64,
    aggregates: &[BackstopAggregate],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(schema_version.to_le_bytes());
    hasher.update(source_watermark.to_le_bytes());
    for aggregate in aggregates {
        hash_bytes(&mut hasher, aggregate.aggregate_id.as_bytes());
        hasher.update(aggregate.revision.to_le_bytes());
        hash_bytes(&mut hasher, aggregate.state_value.as_bytes());
        hash_bytes(&mut hasher, aggregate.last_operation_id.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(bytes.len().to_le_bytes());
    hasher.update(bytes);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("dxbot-{label}-{nanos}.sqlite"))
    }

    fn seed(store: &ReferenceStore, aggregate_id: &str, sequence_payload: &str) {
        store
            .connection
            .execute(
                "INSERT INTO aggregate_state(aggregate_id, revision, state_value, last_operation_id) \
                 VALUES (?1, 1, 'ready', 'operation-1')",
                [aggregate_id],
            )
            .expect("seed aggregate");
        store
            .connection
            .execute(
                "INSERT INTO events(operation_id, aggregate_id, payload) VALUES ('operation-1', ?1, ?2)",
                rusqlite::params![aggregate_id, sequence_payload],
            )
            .expect("seed event");
    }

    fn open(path: &Path) -> ReferenceStore {
        ReferenceStore::open_file(path, "instance-1", 3).expect("open backstop fixture store")
    }

    fn budget() -> BackstopBudget {
        BackstopBudget {
            max_items: 32,
            max_bytes: 16 * 1024,
            min_interval_secs: 0,
        }
    }

    #[test]
    fn capture_has_watermark_digest_and_dry_run_only_restore_plan() {
        let path = temp_path("backstop-capture");
        let mut store = open(&path);
        seed(&store, "bot-1", "created");
        let mut coordinator = BackstopCoordinator::new();
        let captured = coordinator.capture(&mut store, budget(), 100);
        assert!(captured.is_ok());
        let Ok(BackstopCaptureOutcome::Captured(snapshot)) = captured else {
            panic!("expected captured backstop");
        };
        assert_eq!(snapshot.source_watermark, 1);
        assert_eq!(snapshot.aggregates.len(), 1);
        assert_eq!(snapshot.content_digest.len(), 64);
        let plan = snapshot
            .dry_run_restore(store.schema_version().expect("schema version"))
            .expect("dry-run restore plan");
        assert!(plan.requires_authorized_apply);
        assert_eq!(plan.aggregate_count, 1);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn sink_failure_does_not_make_canonical_store_unwritable() {
        let path = temp_path("backstop-sink-failure");
        let mut store = open(&path);
        seed(&store, "bot-1", "created");
        let mut coordinator = BackstopCoordinator::new();
        let mut sink = MemoryBackstopSink::new();
        sink.set_fail_writes(true);
        assert_eq!(
            coordinator.capture_to_sink(&mut store, budget(), 100, &mut sink),
            Err(BackstopError::SinkFailed)
        );
        assert_eq!(
            store
                .delete_projection_row_for_fixture("bot-1")
                .expect("canonical store remains writable"),
            1
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn over_limit_and_stop_are_explicit_non_capture_outcomes() {
        let path = temp_path("backstop-limit");
        let mut store = open(&path);
        seed(&store, "very-long-aggregate-id", "created");
        let mut coordinator = BackstopCoordinator::new();
        let tiny = BackstopBudget {
            max_items: 1,
            max_bytes: 1,
            min_interval_secs: 0,
        };
        assert_eq!(
            coordinator.capture(&mut store, tiny, 100),
            Ok(BackstopCaptureOutcome::SkippedOverLimit)
        );
        coordinator.stop();
        assert_eq!(
            coordinator.capture(&mut store, budget(), 101),
            Ok(BackstopCaptureOutcome::Stopped)
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn quarantine_is_metadata_separate_from_snapshot_contents() {
        let snapshot = BackstopSnapshot {
            snapshot_id: "backstop-1".to_owned(),
            source_watermark: 4,
            schema_version: 2,
            captured_at: 100,
            content_digest: "a".repeat(64),
            retained_bytes: 0,
            aggregates: Vec::new(),
        };
        let mut quarantine = QuarantineRegistry::default();
        quarantine.quarantine(&snapshot, QuarantineReason::OperatorHold);
        assert_eq!(quarantine.entries().len(), 1);
        assert_eq!(quarantine.entries()[0].snapshot_id, "backstop-1");
    }
}

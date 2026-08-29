//! Atomic persistence adapter for Application canonical state.
//!
//! `DomainState` remains the sole in-process canonical owner. This adapter
//! serializes a versioned snapshot after a successful Application mutation and
//! restores it on Runtime restart. Subscription leases/retained event windows
//! are intentionally not persisted: they are observation projections and
//! restart requires explicit resync from canonical task/process state.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use dxbot_core::receipt::ReceiptRecord;
use dxbot_core::types::{CommandId, OperationId, OperationResult, RequestDigest};
use serde::{Deserialize, Serialize};

use crate::delegation::{DelegationRecord, DelegationStatus};
use crate::membership::MembershipRecord;
use crate::mutation::AppError;
use crate::state::{
    BotState, ChannelState, ConversationState, DomainState, ExecutionAuditIntent, ExecutionState,
    IdempotencyBindingState, MemoryState, MessageState, ProcessState, ProjectState,
    SideEffectState, TaskState, ThreadState,
};

// ProcessState is an additive v2 field. Keep the version stable and rely on
// serde(default) so existing v2 snapshots load with an empty process set.
const SNAPSHOT_VERSION: u32 = 2;
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;
const OWNER_DIRECTORY_MODE: u32 = 0o700;
const OWNER_FILE_MODE: u32 = 0o600;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationStateStore {
    path: PathBuf,
}

impl ApplicationStateStore {
    pub fn open(path: PathBuf) -> Result<(Self, DomainState), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
            harden_directory(parent)?;
        }
        let state = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                validate_snapshot_file(&path, &metadata)?;
                if metadata.len() > MAX_SNAPSHOT_BYTES as u64 {
                    return Err(AppError::ResourceExhausted(
                        "application snapshot exceeds 64 MiB capacity".to_owned(),
                    ));
                }
                let bytes = fs::read(&path).map_err(io_error)?;
                let snapshot: SnapshotV2 = serde_json::from_slice(&bytes).map_err(|error| {
                    AppError::Internal(format!("application snapshot is corrupt: {error}"))
                })?;
                snapshot.into_state()?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => DomainState::new(),
            Err(error) => return Err(io_error(error)),
        };
        Ok((Self { path }, state))
    }

    pub fn persist(&self, state: &DomainState) -> Result<(), AppError> {
        let parent = self.path.parent().ok_or_else(|| {
            AppError::Internal("application snapshot path has no parent directory".to_owned())
        })?;
        fs::create_dir_all(parent).map_err(io_error)?;
        harden_directory(parent)?;

        let bytes = serde_json::to_vec(&SnapshotV2::from_state(state)).map_err(|error| {
            AppError::Internal(format!("cannot serialize application snapshot: {error}"))
        })?;
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err(AppError::ResourceExhausted(
                "application snapshot exceeds 64 MiB capacity".to_owned(),
            ));
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(
            ".application-state.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(io_error)?;
        harden_file(&temp)?;
        if let Err(error) =
            write_snapshot_bytes(&mut file, &bytes, &self.path).and_then(|()| file.sync_all())
        {
            let _ = fs::remove_file(&temp);
            return Err(io_error(error));
        }
        drop(file);
        if let Err(error) = fs::rename(&temp, &self.path) {
            let _ = fs::remove_file(&temp);
            return Err(io_error(error));
        }
        harden_file(&self.path)?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(io_error)?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SnapshotV2 {
    version: u32,
    bots: Vec<BotState>,
    conversations: Vec<ConversationState>,
    messages: Vec<MessageState>,
    threads: Vec<ThreadState>,
    tasks: Vec<TaskState>,
    #[serde(default)]
    executions: Vec<ExecutionState>,
    #[serde(default)]
    processes: Vec<ProcessState>,
    projects: Vec<ProjectState>,
    channels: Vec<ChannelState>,
    memories: Vec<MemoryState>,
    side_effects: Vec<SideEffectState>,
    #[serde(default)]
    execution_audit_intents: Vec<ExecutionAuditIntent>,
    receipts: Vec<(OperationId, ReceiptRecord)>,
    results: Vec<(OperationId, OperationResult)>,
    command_bindings: Vec<(CommandId, OperationId)>,
    command_request_digests: Vec<(CommandId, RequestDigest)>,
    idempotency_bindings: Vec<IdempotencySnapshot>,
    memberships: Vec<(String, MembershipRecord)>,
    delegations: Vec<DelegationSnapshot>,
}

impl SnapshotV2 {
    fn from_state(state: &DomainState) -> Self {
        let mut bots: Vec<_> = state.bots.values().cloned().collect();
        bots.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut conversations: Vec<_> = state.conversations.values().cloned().collect();
        conversations.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut messages: Vec<_> = state.messages.values().cloned().collect();
        messages.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut threads: Vec<_> = state.threads.values().cloned().collect();
        threads.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut tasks: Vec<_> = state.tasks.values().cloned().collect();
        tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut executions: Vec<_> = state.executions.values().cloned().collect();
        executions.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut processes: Vec<_> = state.processes.values().cloned().collect();
        processes.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut projects: Vec<_> = state.projects.values().cloned().collect();
        projects.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut channels: Vec<_> = state.channels.values().cloned().collect();
        channels.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut memories: Vec<_> = state.memories.values().cloned().collect();
        memories.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut side_effects: Vec<_> = state.side_effects.values().cloned().collect();
        side_effects.sort_by(|left, right| left.id.cmp(&right.id));
        let mut execution_audit_intents: Vec<_> =
            state.execution_audit_intents.values().cloned().collect();
        execution_audit_intents.sort_by(|left, right| left.key.cmp(&right.key));

        let mut receipts: Vec<_> = state
            .receipts
            .iter()
            .map(|(id, receipt)| (id.clone(), receipt.clone()))
            .collect();
        receipts.sort_by(|left, right| left.0.0.cmp(&right.0.0));
        let mut results: Vec<_> = state
            .results
            .iter()
            .map(|(id, result)| (id.clone(), result.clone()))
            .collect();
        results.sort_by(|left, right| left.0.0.cmp(&right.0.0));
        let mut command_bindings: Vec<_> = state
            .command_bindings
            .iter()
            .map(|(command, operation)| (command.clone(), operation.clone()))
            .collect();
        command_bindings.sort_by(|left, right| left.0.0.cmp(&right.0.0));
        let mut command_request_digests: Vec<_> = state
            .command_request_digests
            .iter()
            .map(|(command, digest)| (command.clone(), digest.clone()))
            .collect();
        command_request_digests.sort_by(|left, right| left.0.0.cmp(&right.0.0));
        let mut idempotency_bindings: Vec<_> = state
            .idempotency_bindings
            .iter()
            .map(|((principal, key_digest), binding)| IdempotencySnapshot {
                principal: principal.clone(),
                key_digest: key_digest.clone(),
                command_id: binding.command_id.clone(),
                expires_at: binding.expires_at,
            })
            .collect();
        idempotency_bindings.sort_by(|left, right| {
            (&left.principal, &left.key_digest).cmp(&(&right.principal, &right.key_digest))
        });
        let mut memberships: Vec<_> = state
            .memberships
            .iter()
            .map(|(key, membership)| (key.clone(), membership.clone()))
            .collect();
        memberships.sort_by(|left, right| left.0.cmp(&right.0));
        let delegations = state
            .delegations
            .iter()
            .map(DelegationSnapshot::from)
            .collect();

        Self {
            version: SNAPSHOT_VERSION,
            bots,
            conversations,
            messages,
            threads,
            tasks,
            executions,
            processes,
            projects,
            channels,
            memories,
            side_effects,
            execution_audit_intents,
            receipts,
            results,
            command_bindings,
            command_request_digests,
            idempotency_bindings,
            memberships,
            delegations,
        }
    }

    fn into_state(self) -> Result<DomainState, AppError> {
        if self.version != SNAPSHOT_VERSION {
            return Err(AppError::Internal(format!(
                "unsupported application snapshot version {}",
                self.version
            )));
        }
        let mut state = DomainState::new();
        for row in self.bots {
            state.bots.insert(row.id.clone(), row);
        }
        for row in self.conversations {
            state.conversations.insert(row.id.clone(), row);
        }
        for row in self.messages {
            state.messages.insert(row.id.clone(), row);
        }
        for row in self.threads {
            state.threads.insert(row.id.clone(), row);
        }
        for row in self.tasks {
            state.tasks.insert(row.id.clone(), row);
        }
        for row in self.executions {
            state.executions.insert(row.id.clone(), row);
        }
        for row in self.processes {
            state.processes.insert(row.id.clone(), row);
        }
        for row in self.projects {
            state.projects.insert(row.id.clone(), row);
        }
        for row in self.channels {
            state.channels.insert(row.id.clone(), row);
        }
        for row in self.memories {
            state.memories.insert(row.id.clone(), row);
        }
        for row in self.side_effects {
            state.side_effects.insert(row.id.clone(), row);
        }
        for row in self.execution_audit_intents {
            state.execution_audit_intents.insert(row.key.clone(), row);
        }
        state.receipts.extend(self.receipts);
        state.results.extend(self.results);
        state.command_bindings.extend(self.command_bindings);
        state
            .command_request_digests
            .extend(self.command_request_digests);
        for row in self.idempotency_bindings {
            state.idempotency_bindings.insert(
                (row.principal, row.key_digest),
                IdempotencyBindingState {
                    command_id: row.command_id,
                    expires_at: row.expires_at,
                },
            );
        }
        state.memberships.extend(self.memberships);
        for row in self.delegations {
            state.delegations.push(row.into_record()?);
        }
        Ok(state)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IdempotencySnapshot {
    principal: String,
    key_digest: String,
    command_id: CommandId,
    expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DelegationSnapshot {
    id: String,
    task_id: dxbot_core::types::TaskId,
    from_bot: dxbot_core::types::BotSelector,
    to_bot: dxbot_core::types::BotSelector,
    scope: dxbot_core::types::ScopeSelector,
    role: String,
    status: String,
    created_at: i64,
    resolved_at: Option<i64>,
}

impl From<&DelegationRecord> for DelegationSnapshot {
    fn from(record: &DelegationRecord) -> Self {
        Self {
            id: record.id.clone(),
            task_id: record.task_id.clone(),
            from_bot: record.from_bot.clone(),
            to_bot: record.to_bot.clone(),
            scope: record.scope.clone(),
            role: record.role.clone(),
            status: delegation_status_name(record.status).to_owned(),
            created_at: record.created_at,
            resolved_at: record.resolved_at,
        }
    }
}

impl DelegationSnapshot {
    fn into_record(self) -> Result<DelegationRecord, AppError> {
        Ok(DelegationRecord {
            id: self.id,
            task_id: self.task_id,
            from_bot: self.from_bot,
            to_bot: self.to_bot,
            scope: self.scope,
            role: self.role,
            status: parse_delegation_status(&self.status)?,
            created_at: self.created_at,
            resolved_at: self.resolved_at,
        })
    }
}

fn delegation_status_name(value: DelegationStatus) -> &'static str {
    match value {
        DelegationStatus::Pending => "pending",
        DelegationStatus::Accepted => "accepted",
        DelegationStatus::Rejected => "rejected",
        DelegationStatus::Expired => "expired",
    }
}

fn write_snapshot_bytes(file: &mut File, bytes: &[u8], _target: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if test_storage_full_path()
        .lock()
        .is_ok_and(|path| path.as_ref() == Some(&_target.to_path_buf()))
    {
        let partial = bytes.len().min(32);
        file.write_all(&bytes[..partial])?;
        return Err(std::io::Error::new(
            std::io::ErrorKind::StorageFull,
            "injected production snapshot storage-full fault",
        ));
    }
    file.write_all(bytes)
}

#[cfg(test)]
fn test_storage_full_path() -> &'static Mutex<Option<PathBuf>> {
    static PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    PATH.get_or_init(|| Mutex::new(None))
}

fn parse_delegation_status(value: &str) -> Result<DelegationStatus, AppError> {
    match value {
        "pending" => Ok(DelegationStatus::Pending),
        "accepted" => Ok(DelegationStatus::Accepted),
        "rejected" => Ok(DelegationStatus::Rejected),
        "expired" => Ok(DelegationStatus::Expired),
        other => Err(AppError::Internal(format!(
            "unknown persisted delegation status: {other}"
        ))),
    }
}

fn validate_snapshot_file(path: &Path, metadata: &fs::Metadata) -> Result<(), AppError> {
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(AppError::Internal(format!(
            "application snapshot is not a direct regular file: {}",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(AppError::Internal(format!(
            "application snapshot permissions are not owner-only: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn harden_directory(path: &Path) -> Result<(), AppError> {
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_DIRECTORY_MODE)).map_err(io_error)
}

#[cfg(not(unix))]
fn harden_directory(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

#[cfg(unix)]
fn harden_file(path: &Path) -> Result<(), AppError> {
    fs::set_permissions(path, fs::Permissions::from_mode(OWNER_FILE_MODE)).map_err(io_error)
}

#[cfg(not(unix))]
fn harden_file(_path: &Path) -> Result<(), AppError> {
    Ok(())
}

fn io_error(error: std::io::Error) -> AppError {
    AppError::Internal(format!("application persistence I/O: {error}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn path() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "dxbot-application-snapshot-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).expect("private test root");
        root.join("application-state.json")
    }

    fn cleanup(path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn snapshot_round_trip_preserves_operation_bindings() {
        let path = path();
        let (store, mut state) = ApplicationStateStore::open(path.clone()).expect("store opens");
        state.command_bindings.insert(
            CommandId("command-a".to_owned()),
            OperationId("operation-a".to_owned()),
        );
        state.command_request_digests.insert(
            CommandId("command-a".to_owned()),
            RequestDigest("digest-a".to_owned()),
        );
        store.persist(&state).expect("snapshot persists");
        let (_, restored) = ApplicationStateStore::open(path.clone()).expect("snapshot restores");
        assert_eq!(
            restored
                .command_bindings
                .get(&CommandId("command-a".to_owned())),
            Some(&OperationId("operation-a".to_owned()))
        );
        cleanup(&path);
    }

    #[test]
    fn side_effect_operation_linkage_survives_snapshot_round_trip() {
        let path = path();
        let (store, mut state) = ApplicationStateStore::open(path.clone()).expect("store opens");
        state.side_effects.insert(
            "effect-a".to_owned(),
            SideEffectState {
                id: "effect-a".to_owned(),
                revision: 1,
                status: crate::state::SideEffectStatus::Dispatched,
                evidence: Vec::new(),
                operation_ref: Some(OperationId("operation-a".to_owned())),
                execution_ref: None,
                process_ref: None,
            },
        );
        store.persist(&state).expect("snapshot persists");
        let (_, restored) = ApplicationStateStore::open(path.clone()).expect("snapshot restores");
        assert_eq!(
            restored
                .side_effects
                .get("effect-a")
                .and_then(|effect| effect.operation_ref.as_ref()),
            Some(&OperationId("operation-a".to_owned()))
        );
        cleanup(&path);
    }

    #[test]
    fn legacy_v2_without_processes_still_loads() {
        let path = path();
        let legacy = serde_json::json!({
            "version": 2,
            "bots": [],
            "conversations": [],
            "messages": [],
            "threads": [],
            "tasks": [],
            "projects": [],
            "channels": [],
            "memories": [],
            "side_effects": [],
            "receipts": [],
            "results": [],
            "command_bindings": [],
            "command_request_digests": [],
            "idempotency_bindings": [],
            "memberships": [],
            "delegations": []
        });
        fs::write(&path, serde_json::to_vec(&legacy).expect("legacy encodes"))
            .expect("legacy writes");
        #[cfg(unix)]
        fs::set_permissions(&path, fs::Permissions::from_mode(OWNER_FILE_MODE)).expect("mode sets");
        let (_, restored) = ApplicationStateStore::open(path.clone()).expect("legacy loads");
        assert!(restored.processes.is_empty());
        cleanup(&path);
    }

    #[test]
    fn snapshot_capacity_failure_preserves_prior_durable_state() {
        let path = path();
        let (store, state) = ApplicationStateStore::open(path.clone()).expect("store opens");
        store.persist(&state).expect("baseline persists");
        let baseline = fs::read(&path).expect("baseline bytes");

        let mut oversized = state;
        oversized.command_request_digests.insert(
            CommandId("oversized-command".to_owned()),
            RequestDigest("x".repeat(MAX_SNAPSHOT_BYTES + 1)),
        );
        assert!(matches!(
            store.persist(&oversized),
            Err(AppError::ResourceExhausted(_))
        ));
        assert_eq!(fs::read(&path).expect("durable state"), baseline);
        let (_, restored) = ApplicationStateStore::open(path.clone()).expect("baseline restores");
        assert!(restored.command_request_digests.is_empty());
        cleanup(&path);
    }

    #[test]
    fn storage_full_during_temp_write_preserves_prior_snapshot() {
        let path = path();
        let (store, mut state) = ApplicationStateStore::open(path.clone()).expect("store opens");
        store.persist(&state).expect("baseline persists");
        let baseline = fs::read(&path).expect("baseline bytes");
        state.command_request_digests.insert(
            CommandId("new-command".to_owned()),
            RequestDigest("new-digest".to_owned()),
        );
        *test_storage_full_path().lock().expect("fault lock") = Some(path.clone());
        let error = store.persist(&state).expect_err("storage full");
        *test_storage_full_path().lock().expect("fault lock") = None;
        assert!(matches!(error, AppError::Internal(message) if message.contains("storage-full")));
        assert_eq!(fs::read(&path).expect("durable state"), baseline);
        let (_, restored) = ApplicationStateStore::open(path.clone()).expect("baseline restores");
        assert!(restored.command_request_digests.is_empty());
        cleanup(&path);
    }
}

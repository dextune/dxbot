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

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use dxbot_core::receipt::ReceiptRecord;
use dxbot_core::types::{
    BotId, BotSelector, CommandId, ConversationId, MessageId, OperationId, OperationResult,
    RequestDigest, ScopeSelector, TaskId, ThreadId,
};
use serde::{Deserialize, Serialize};

use crate::delegation::{DelegationRecord, DelegationStatus};
use crate::membership::MembershipRecord;
use crate::mutation::AppError;
use crate::state::{
    BotState, ConversationState, DomainState, IdempotencyBindingState, LifecycleState, TaskState,
    TaskStatus, ThreadState,
};

const SNAPSHOT_VERSION: u32 = 1;
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
                let bytes = fs::read(&path).map_err(io_error)?;
                let snapshot: SnapshotV1 = serde_json::from_slice(&bytes)
                    .map_err(|error| AppError::Internal(format!(
                        "application snapshot is corrupt: {error}"
                    )))?;
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

        let snapshot = SnapshotV1::from_state(state);
        let bytes = serde_json::to_vec(&snapshot).map_err(|error| {
            AppError::Internal(format!("cannot serialize application snapshot: {error}"))
        })?;
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
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
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
struct SnapshotV1 {
    version: u32,
    bots: Vec<BotSnapshot>,
    conversations: Vec<ConversationSnapshot>,
    threads: Vec<ThreadSnapshot>,
    tasks: Vec<TaskSnapshot>,
    receipts: Vec<(OperationId, ReceiptRecord)>,
    results: Vec<(OperationId, OperationResult)>,
    command_bindings: Vec<(CommandId, OperationId)>,
    command_request_digests: Vec<(CommandId, RequestDigest)>,
    idempotency_bindings: Vec<IdempotencySnapshot>,
    memberships: Vec<MembershipSnapshot>,
    delegations: Vec<DelegationSnapshot>,
}

impl SnapshotV1 {
    fn from_state(state: &DomainState) -> Self {
        let mut bots: Vec<_> = state.bots.values().map(BotSnapshot::from).collect();
        bots.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut conversations: Vec<_> = state
            .conversations
            .values()
            .map(ConversationSnapshot::from)
            .collect();
        conversations.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut threads: Vec<_> = state.threads.values().map(ThreadSnapshot::from).collect();
        threads.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        let mut tasks: Vec<_> = state.tasks.values().map(TaskSnapshot::from).collect();
        tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));

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
            .map(|(key, record)| MembershipSnapshot::from_pair(key, record))
            .collect();
        memberships.sort_by(|left, right| left.key.cmp(&right.key));
        let delegations = state
            .delegations
            .iter()
            .map(DelegationSnapshot::from)
            .collect();

        Self {
            version: SNAPSHOT_VERSION,
            bots,
            conversations,
            threads,
            tasks,
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
        for snapshot in self.bots {
            let record = snapshot.into_state()?;
            state.bots.insert(record.id.clone(), record);
        }
        for snapshot in self.conversations {
            let record = snapshot.into_state();
            state.conversations.insert(record.id.clone(), record);
        }
        for snapshot in self.threads {
            let record = snapshot.into_state();
            state.threads.insert(record.id.clone(), record);
        }
        for snapshot in self.tasks {
            let record = snapshot.into_state()?;
            state.tasks.insert(record.id.clone(), record);
        }
        state.receipts.extend(self.receipts);
        state.results.extend(self.results);
        state.command_bindings.extend(self.command_bindings);
        state
            .command_request_digests
            .extend(self.command_request_digests);
        for snapshot in self.idempotency_bindings {
            state.idempotency_bindings.insert(
                (snapshot.principal, snapshot.key_digest),
                IdempotencyBindingState {
                    command_id: snapshot.command_id,
                    expires_at: snapshot.expires_at,
                },
            );
        }
        for snapshot in self.memberships {
            let (key, record) = snapshot.into_pair();
            state.memberships.insert(key, record);
        }
        for snapshot in self.delegations {
            state.delegations.push(snapshot.into_record()?);
        }
        Ok(state)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BotSnapshot {
    id: BotId,
    name: String,
    revision: i64,
    lifecycle: String,
}

impl From<&BotState> for BotSnapshot {
    fn from(state: &BotState) -> Self {
        Self {
            id: state.id.clone(),
            name: state.name.clone(),
            revision: state.revision,
            lifecycle: lifecycle_name(state.lifecycle).to_owned(),
        }
    }
}

impl BotSnapshot {
    fn into_state(self) -> Result<BotState, AppError> {
        Ok(BotState {
            id: self.id,
            name: self.name,
            revision: self.revision,
            lifecycle: parse_lifecycle(&self.lifecycle)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationSnapshot {
    id: ConversationId,
    bot_id: BotId,
    revision: i64,
    messages: Vec<MessageId>,
}

impl From<&ConversationState> for ConversationSnapshot {
    fn from(state: &ConversationState) -> Self {
        Self {
            id: state.id.clone(),
            bot_id: state.bot_id.clone(),
            revision: state.revision,
            messages: state.messages.clone(),
        }
    }
}

impl ConversationSnapshot {
    fn into_state(self) -> ConversationState {
        ConversationState {
            id: self.id,
            bot_id: self.bot_id,
            revision: self.revision,
            messages: self.messages,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ThreadSnapshot {
    id: ThreadId,
    conversation_id: ConversationId,
    revision: i64,
    parent_message_id: Option<MessageId>,
}

impl From<&ThreadState> for ThreadSnapshot {
    fn from(state: &ThreadState) -> Self {
        Self {
            id: state.id.clone(),
            conversation_id: state.conversation_id.clone(),
            revision: state.revision,
            parent_message_id: state.parent_message_id.clone(),
        }
    }
}

impl ThreadSnapshot {
    fn into_state(self) -> ThreadState {
        ThreadState {
            id: self.id,
            conversation_id: self.conversation_id,
            revision: self.revision,
            parent_message_id: self.parent_message_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskSnapshot {
    id: TaskId,
    owner: String,
    revision: i64,
    execution_generation: i64,
    status: String,
}

impl From<&TaskState> for TaskSnapshot {
    fn from(state: &TaskState) -> Self {
        Self {
            id: state.id.clone(),
            owner: state.owner.clone(),
            revision: state.revision,
            execution_generation: state.execution_generation,
            status: task_status_name(state.status).to_owned(),
        }
    }
}

impl TaskSnapshot {
    fn into_state(self) -> Result<TaskState, AppError> {
        Ok(TaskState {
            id: self.id,
            owner: self.owner,
            revision: self.revision,
            execution_generation: self.execution_generation,
            status: parse_task_status(&self.status)?,
        })
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
struct MembershipSnapshot {
    key: String,
    id: String,
    scope: ScopeSelector,
    member_bot: BotSelector,
    role: String,
    generation: i64,
    created_at: i64,
}

impl MembershipSnapshot {
    fn from_pair(key: &str, record: &MembershipRecord) -> Self {
        Self {
            key: key.to_owned(),
            id: record.id.clone(),
            scope: record.scope.clone(),
            member_bot: record.member_bot.clone(),
            role: record.role.clone(),
            generation: record.generation,
            created_at: record.created_at,
        }
    }

    fn into_pair(self) -> (String, MembershipRecord) {
        let record = MembershipRecord {
            id: self.id,
            scope: self.scope,
            member_bot: self.member_bot,
            role: self.role,
            generation: self.generation,
            created_at: self.created_at,
        };
        (self.key, record)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DelegationSnapshot {
    id: String,
    task_id: TaskId,
    from_bot: BotSelector,
    to_bot: BotSelector,
    scope: ScopeSelector,
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

fn lifecycle_name(value: LifecycleState) -> &'static str {
    match value {
        LifecycleState::Active => "active",
        LifecycleState::Inactive => "inactive",
        LifecycleState::Degraded => "degraded",
        LifecycleState::Terminated => "terminated",
    }
}

fn parse_lifecycle(value: &str) -> Result<LifecycleState, AppError> {
    match value {
        "active" => Ok(LifecycleState::Active),
        "inactive" => Ok(LifecycleState::Inactive),
        "degraded" => Ok(LifecycleState::Degraded),
        "terminated" => Ok(LifecycleState::Terminated),
        other => Err(AppError::Internal(format!(
            "unknown persisted bot lifecycle: {other}"
        ))),
    }
}

fn task_status_name(value: TaskStatus) -> &'static str {
    match value {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Succeeded => "succeeded",
        TaskStatus::Failed => "failed",
        TaskStatus::Rejected => "rejected",
    }
}

fn parse_task_status(value: &str) -> Result<TaskStatus, AppError> {
    match value {
        "pending" => Ok(TaskStatus::Pending),
        "running" => Ok(TaskStatus::Running),
        "succeeded" => Ok(TaskStatus::Succeeded),
        "failed" => Ok(TaskStatus::Failed),
        "rejected" => Ok(TaskStatus::Rejected),
        other => Err(AppError::Internal(format!(
            "unknown persisted task status: {other}"
        ))),
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
        std::env::temp_dir().join(format!(
            "dxbot-application-snapshot-{}-{}.json",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
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
            restored.command_bindings.get(&CommandId("command-a".to_owned())),
            Some(&OperationId("operation-a".to_owned()))
        );
        let _ = fs::remove_file(path);
    }
}

//! Canonical in-process Application state.
//!
//! This store owns durable Domain facts and operation bindings. Runtime
//! security Authority/Approval and Provider Host registrations remain in their
//! own canonical owners and are deliberately not duplicated here. Durable
//! Process orchestration state lives here as an Application aggregate and only
//! stores refs/progress, never child Task/Memory copies.

use std::collections::HashMap;

use dxbot_core::receipt::ReceiptRecord;
use dxbot_core::types::{
    BotId, ChannelId, CommandId, ContentSource, ConversationId, MemoryId, MessageId, OperationId,
    OperationResult, OperationSelector, ProcessId, ProjectId, RequestDigest, ScopeSelector,
    SideEffectSelector, TaskId, ThreadId,
};
use serde::{Deserialize, Serialize};

use crate::delegation::DelegationRecord;
use crate::membership::MembershipRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LifecycleState {
    Active,
    Inactive,
    Degraded,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectLifecycle {
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Pending,
    Running,
    Suspended,
    Succeeded,
    Failed,
    Rejected,
    Deferred,
    Cancelled,
    RecoveryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessLifecycle {
    Created,
    Running,
    Waiting,
    Suspending,
    Suspended,
    Resuming,
    Completing,
    Cancelling,
    Completed,
    Cancelled,
    Failed,
    RecoveryRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MemoryAssertionStatus {
    Proposed,
    Accepted,
    Superseded,
    Retracted,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SideEffectStatus {
    Prepared,
    Dispatched,
    Confirmed,
    Failed,
    Unknown,
    Reconciling,
    ManualResolutionRequired,
}

/// Explicitly modeled Bot policy bindings.
///
/// This is the Application-owned binding of a Bot to the policy references it
/// was created with. The Application does not invent default policy
/// *generations* (that is the Runtime bootstrap owner's responsibility per
/// `DXB-RUN-035` / BF-CLI-026); it only durably records the explicit bindings
/// supplied at creation so the fields are never silently ignored. Fields are
/// modeled explicitly rather than kept as an opaque JSON blob.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BotPolicyBindings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brain_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_policy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_policy: Option<String>,
}

impl BotPolicyBindings {
    pub fn is_empty(&self) -> bool {
        self.brain_policy.is_none()
            && self.permission_policy.is_none()
            && self.resource_policy.is_none()
            && self.provider_policy.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotState {
    pub id: BotId,
    pub name: String,
    pub revision: i64,
    pub lifecycle: LifecycleState,
    /// Explicit policy bindings supplied at creation. Backward-compatible:
    /// snapshots written before this field default to empty bindings.
    #[serde(default)]
    pub policy_bindings: BotPolicyBindings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ConversationOwner {
    Bot {
        bot_id: BotId,
    },
    Channel {
        project_id: ProjectId,
        channel_id: ChannelId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationState {
    pub id: ConversationId,
    pub owner: ConversationOwner,
    pub revision: i64,
    pub messages: Vec<MessageId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageState {
    pub id: MessageId,
    pub content: ContentSource,
    pub sequence: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadState {
    pub id: ThreadId,
    pub conversation_id: ConversationId,
    pub revision: i64,
    pub parent_message_id: Option<MessageId>,
    pub title: String,
    pub messages: Vec<MessageId>,
}

/// Explicitly modeled task execution constraints supplied at submit time.
///
/// Fields are modeled individually rather than as an opaque JSON blob so that
/// `task-submit`'s `delegate_to_bot` / `requested_sender_bot` / `deadline` /
/// `budget` fields are durably recorded on the aggregate instead of being
/// silently ignored.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TaskExecutionConstraints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegate_to_bot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_sender_bot: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<String>,
}

impl TaskExecutionConstraints {
    pub fn is_empty(&self) -> bool {
        self.delegate_to_bot.is_none()
            && self.requested_sender_bot.is_none()
            && self.deadline.is_none()
            && self.budget.is_none()
    }
}

/// A durable, append-only record of a supervision directive applied to a task.
///
/// This gives `task-cancel` / `task-suspend`'s `reason` field a real audit home
/// rather than discarding it. The directive references the task revision the
/// control took effect at; it never copies task lifecycle into a second owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskControlDirective {
    pub command_key: String,
    pub applied_revision: i64,
    pub execution_generation: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskState {
    pub id: TaskId,
    pub owner: String,
    pub revision: i64,
    pub execution_generation: i64,
    pub status: TaskStatus,
    pub intent: Option<ContentSource>,
    pub result: Option<serde_json::Value>,
    /// Explicit submit-time execution constraints. Backward-compatible default.
    #[serde(default)]
    pub constraints: TaskExecutionConstraints,
    /// Append-only supervision/control audit trail. Backward-compatible default.
    #[serde(default)]
    pub control_history: Vec<TaskControlDirective>,
    /// Durable ref to the orchestration Process this task participates in.
    /// This is a *reference*, not a copy of Process state. Backward-compatible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessState {
    pub id: ProcessId,
    pub definition_id: String,
    pub definition_version: String,
    pub revision: i64,
    pub scope_ref: String,
    pub initiator_ref: String,
    pub lifecycle: ProcessLifecycle,
    pub current_step_ref: Option<String>,
    pub waiting_condition_ref: Option<String>,
    pub child_refs: Vec<String>,
    pub progress: i64,
    pub terminal_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectState {
    pub id: ProjectId,
    pub name: String,
    pub owner_bot: BotId,
    pub revision: i64,
    pub lifecycle: ProjectLifecycle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelState {
    pub id: ChannelId,
    pub project_id: ProjectId,
    pub name: String,
    pub revision: i64,
    pub conversation_id: ConversationId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryRevisionState {
    pub revision: i64,
    pub status: MemoryAssertionStatus,
    pub statement: ContentSource,
    pub evidence: Vec<String>,
}

/// Durable provenance of a memory declassification, recorded at promote time.
///
/// `memory-promote`'s `declassification_ref` names the information-label /
/// declassification authority the promotion relied on. The Application does not
/// own the label policy itself, but it durably records the reference and the
/// revision it was applied at so the field is auditable rather than silently
/// dropped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclassificationRecord {
    pub declassification_ref: String,
    pub applied_revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryState {
    pub id: MemoryId,
    pub scope_key: String,
    pub revision: i64,
    pub status: MemoryAssertionStatus,
    pub statement: ContentSource,
    pub evidence: Vec<String>,
    pub history: Vec<MemoryRevisionState>,
    /// Append-only declassification provenance. Backward-compatible default.
    #[serde(default)]
    pub declassifications: Vec<DeclassificationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectState {
    pub id: String,
    pub revision: i64,
    pub status: SideEffectStatus,
    pub evidence: Vec<String>,
    /// Durable linkage to the originating Operation, enabling
    /// `side-effect reconcile` selection by Operation. Backward-compatible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_ref: Option<OperationId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyBindingState {
    pub command_id: CommandId,
    pub expires_at: i64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomainState {
    pub bots: HashMap<BotId, BotState>,
    pub conversations: HashMap<ConversationId, ConversationState>,
    pub messages: HashMap<MessageId, MessageState>,
    pub threads: HashMap<ThreadId, ThreadState>,
    pub tasks: HashMap<TaskId, TaskState>,
    pub processes: HashMap<ProcessId, ProcessState>,
    pub projects: HashMap<ProjectId, ProjectState>,
    pub channels: HashMap<ChannelId, ChannelState>,
    pub memories: HashMap<MemoryId, MemoryState>,
    pub side_effects: HashMap<String, SideEffectState>,
    pub receipts: HashMap<OperationId, ReceiptRecord>,
    pub results: HashMap<OperationId, OperationResult>,
    pub command_bindings: HashMap<CommandId, OperationId>,
    pub command_request_digests: HashMap<CommandId, RequestDigest>,
    pub idempotency_bindings: HashMap<(String, String), IdempotencyBindingState>,
    pub memberships: HashMap<String, MembershipRecord>,
    pub delegations: Vec<DelegationRecord>,
    pub subscriptions: crate::subscription::SubscriptionRegistry,
}

impl DomainState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }

    pub fn conversation_count(&self) -> usize {
        self.conversations.len()
    }

    pub fn scope_exists(&self, scope: &ScopeSelector) -> bool {
        match scope {
            ScopeSelector::Bot(selector) => match selector {
                dxbot_core::types::BotSelector::CanonicalId(id) => self.bots.contains_key(id),
                dxbot_core::types::BotSelector::ScopedExact(name) => {
                    self.bots.values().any(|bot| bot.name == *name)
                }
            },
            ScopeSelector::Project(selector) => match selector {
                dxbot_core::types::ProjectSelector::CanonicalId(id) => {
                    self.projects.contains_key(id)
                }
                dxbot_core::types::ProjectSelector::VisibleExact(name) => {
                    self.projects.values().any(|project| project.name == *name)
                }
            },
            ScopeSelector::Channel(selector) => match selector {
                dxbot_core::types::ChannelSelector::CanonicalId(id) => {
                    self.channels.contains_key(id)
                }
                dxbot_core::types::ChannelSelector::ProjectExact { project, name } => self
                    .channels
                    .values()
                    .any(|channel| channel.project_id.0 == *project && channel.name == *name),
            },
        }
    }

    pub fn side_effect_id(selector: &SideEffectSelector) -> Option<&str> {
        match selector {
            SideEffectSelector::CanonicalId(id) => Some(id.as_str()),
            SideEffectSelector::Operation(_) => None,
        }
    }

    /// Resolve a [`SideEffectSelector`] against canonical state, including the
    /// Operation linkage index. Returns the canonical side-effect id.
    ///
    /// Selection by Operation is unambiguous because each side effect records
    /// at most one originating `operation_ref`; multiple side effects sharing an
    /// operation are reported as an ambiguity rather than silently picking one.
    pub fn resolve_side_effect_id(
        &self,
        selector: &SideEffectSelector,
    ) -> Result<String, SideEffectResolveError> {
        match selector {
            SideEffectSelector::CanonicalId(id) => {
                if self.side_effects.contains_key(id) {
                    Ok(id.clone())
                } else {
                    Err(SideEffectResolveError::NotFound(id.clone()))
                }
            }
            SideEffectSelector::Operation(operation) => {
                let operation_id = match operation {
                    OperationSelector::OperationId(id) => id.clone(),
                    OperationSelector::CommandIdKey { command_id, .. } => self
                        .command_bindings
                        .get(command_id)
                        .cloned()
                        .ok_or_else(|| SideEffectResolveError::NotFound(command_id.0.clone()))?,
                };
                let mut matches = self
                    .side_effects
                    .values()
                    .filter(|row| row.operation_ref.as_ref() == Some(&operation_id));
                let first = matches
                    .next()
                    .ok_or_else(|| SideEffectResolveError::NotFound(operation_id.0.clone()))?;
                if matches.next().is_some() {
                    return Err(SideEffectResolveError::Ambiguous(operation_id.0.clone()));
                }
                Ok(first.id.clone())
            }
        }
    }
}

/// Failure modes when resolving a [`SideEffectSelector`] to a canonical id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SideEffectResolveError {
    NotFound(String),
    Ambiguous(String),
}

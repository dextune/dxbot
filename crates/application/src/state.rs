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
    OperationResult, ProcessId, ProjectId, RequestDigest, ScopeSelector, SideEffectSelector, TaskId,
    ThreadId,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BotState {
    pub id: BotId,
    pub name: String,
    pub revision: i64,
    pub lifecycle: LifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ConversationOwner {
    Bot { bot_id: BotId },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskState {
    pub id: TaskId,
    pub owner: String,
    pub revision: i64,
    pub execution_generation: i64,
    pub status: TaskStatus,
    pub intent: Option<ContentSource>,
    pub result: Option<serde_json::Value>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryState {
    pub id: MemoryId,
    pub scope_key: String,
    pub revision: i64,
    pub status: MemoryAssertionStatus,
    pub statement: ContentSource,
    pub evidence: Vec<String>,
    pub history: Vec<MemoryRevisionState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SideEffectState {
    pub id: String,
    pub revision: i64,
    pub status: SideEffectStatus,
    pub evidence: Vec<String>,
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
}

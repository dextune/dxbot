//! In-memory domain state for the application fixture.

use std::collections::HashMap;

use dxbot_core::receipt::ReceiptRecord;
use dxbot_core::types::{
    BotId, CommandId, ConversationId, OperationId, RequestDigest, TaskId, ThreadId,
};

use crate::delegation::DelegationRecord;
use crate::membership::MembershipRecord;
use crate::subscription::SubscriptionRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Inactive,
    Degraded,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotState {
    pub id: BotId,
    pub name: String,
    pub revision: i64,
    pub lifecycle: LifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationState {
    pub id: ConversationId,
    pub bot_id: BotId,
    pub revision: i64,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadState {
    pub id: ThreadId,
    pub conversation_id: ConversationId,
    pub revision: i64,
    pub parent_message_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Suspended,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    pub id: TaskId,
    pub owner: String,
    pub status: TaskStatus,
    pub revision: i64,
    pub execution_generation: i64,
}

#[derive(Debug, Clone, Default)]
pub struct DomainState {
    pub bots: HashMap<BotId, BotState>,
    pub conversations: HashMap<ConversationId, ConversationState>,
    pub threads: HashMap<ThreadId, ThreadState>,
    pub tasks: HashMap<TaskId, TaskState>,
    pub receipts: HashMap<OperationId, ReceiptRecord>,
    pub results: HashMap<OperationId, dxbot_core::types::OperationResult>,
    pub command_bindings: HashMap<CommandId, OperationId>,
    pub command_request_digests: HashMap<CommandId, RequestDigest>,
    /// Principal/key_digest identify the lookup index; expiry metadata is kept
    /// in the value so changing it cannot silently mint a new binding.
    pub idempotency_bindings: HashMap<(String, String), (i64, CommandId)>,
    /// Membership owns membership facts only. Security authority remains owned
    /// by runtime-security::AuthorityManager.
    pub memberships: HashMap<String, MembershipRecord>,
    pub delegations: Vec<DelegationRecord>,
    pub subscriptions: SubscriptionRegistry,
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
}
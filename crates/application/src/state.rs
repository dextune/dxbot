//! In-memory domain state store.
//!
//! This is the plain state fixture backing [`crate::ApplicationMutator`]. It holds
//! canonical domain entities plus operation/binding state. Everything here is
//! in-memory only; there is no external I/O.

use std::collections::HashMap;

use dxbot_core::receipt::ReceiptRecord;
use dxbot_core::types::{
    BotId, CommandId, ConversationId, MessageId, OperationId, OperationResult, RequestDigest,
    TaskId, ThreadId,
};

use crate::delegation::DelegationRecord;
use crate::membership::MembershipRecord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Inactive,
    Degraded,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Rejected,
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
    pub messages: Vec<MessageId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadState {
    pub id: ThreadId,
    pub conversation_id: ConversationId,
    pub revision: i64,
    pub parent_message_id: Option<MessageId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    pub id: TaskId,
    pub owner: String,
    pub revision: i64,
    pub execution_generation: i64,
    pub status: TaskStatus,
}

/// Durable metadata attached to the principal/key-digest idempotency index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyBindingState {
    pub command_id: CommandId,
    pub expires_at: i64,
}

/// In-memory domain state store. Security AuthorityBinding deliberately does
/// not live here: runtime-security is its sole canonical owner.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomainState {
    pub bots: HashMap<BotId, BotState>,
    pub conversations: HashMap<ConversationId, ConversationState>,
    pub threads: HashMap<ThreadId, ThreadState>,
    pub tasks: HashMap<TaskId, TaskState>,
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
}

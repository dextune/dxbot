//! In-memory domain state store.
//!
//! This is the plain state fixture backing [`crate::ApplicationMutator`]. It holds
//! the canonical domain entities and the operation receipts produced by mutation.
//! Everything here is in-memory only; there is no external I/O.

use std::collections::HashMap;

use dxbot_core::receipt::ReceiptRecord;
use dxbot_core::types::{
    BotId, ConversationId, MessageId, OperationId, OperationResult, TaskId, ThreadId,
};

use crate::delegation::DelegationRecord;
use crate::membership::{AuthorityBinding, MembershipRecord};

/// Lifecycle of a bot identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Active,
    Inactive,
    Degraded,
    Terminated,
}

/// Execution status of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Rejected,
}

/// Canonical bot identity state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotState {
    pub id: BotId,
    pub name: String,
    pub revision: i64,
    pub lifecycle: LifecycleState,
}

/// Canonical conversation identity state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationState {
    pub id: ConversationId,
    pub bot_id: BotId,
    pub revision: i64,
    pub messages: Vec<MessageId>,
}

/// Canonical thread identity state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadState {
    pub id: ThreadId,
    pub conversation_id: ConversationId,
    pub revision: i64,
    pub parent_message_id: Option<MessageId>,
}

/// Canonical task identity state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskState {
    pub id: TaskId,
    pub owner: String,
    pub revision: i64,
    pub execution_generation: i64,
    pub status: TaskStatus,
}

/// In-memory domain state store.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DomainState {
    pub bots: HashMap<BotId, BotState>,
    pub conversations: HashMap<ConversationId, ConversationState>,
    pub threads: HashMap<ThreadId, ThreadState>,
    pub tasks: HashMap<TaskId, TaskState>,
    pub receipts: HashMap<OperationId, ReceiptRecord>,
    /// Completed operation results, keyed by `OperationId`, for idempotent replay.
    pub results: HashMap<OperationId, OperationResult>,
    /// `CommandId` -> `OperationId` binding index for same-command conflict detection.
    pub command_bindings: HashMap<dxbot_core::types::CommandId, OperationId>,
    /// Memberships keyed by stable `(scope, member_bot)` identity.
    pub memberships: HashMap<String, MembershipRecord>,
    /// Authority bindings keyed by stable `(scope, principal)` identity.
    pub authority_bindings: HashMap<String, AuthorityBinding>,
    /// Task delegation records in insertion order.
    pub delegations: Vec<DelegationRecord>,
    /// AT-APP-007 subscription stream registry (per-target retained events and
    /// subscription positions). Empty until a subscription is created.
    pub subscriptions: crate::subscription::SubscriptionRegistry,
}

impl DomainState {
    /// Create an empty domain state store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of bot entities currently stored.
    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }

    /// Number of conversation entities currently stored.
    pub fn conversation_count(&self) -> usize {
        self.conversations.len()
    }
}

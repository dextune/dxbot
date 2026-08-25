#![forbid(unsafe_code)]

/// Domain mutation: target materialization, receipt, and domain-outcome semantics.
pub mod mutation;
/// In-memory domain state store backing the mutator.
pub mod state;
/// Domain-outcome resolution semantics.
pub mod outcome;
/// Domain query: bounded pagination, cursor continuation, `--all` bounding and resync.
pub mod query;
/// Domain subscription: cursor-based, at-least-once event streams with reconnect,
/// gap detection and bounded buffers.
pub mod subscription;
/// Application membership facts and generation CAS. Runtime authority remains
/// owned by `runtime-security`.
pub mod membership;
/// Application delegation: typed multi-bot task delegation with membership/scope boundary.
pub mod delegation;

pub use delegation::{
    DelegationManager, DelegationRecord, DelegationStatus, DelegationSummary,
};
pub use membership::{MembershipManager, MembershipRecord, MembershipSummary};
pub use mutation::{AppError, ApplicationMutator};
pub use outcome::{DomainOutcome, cas_if_revision, resolve_outcome};
pub use query::{
    AllLoopResult, ApplicationQuery, BotSummary, ConversationSummary, Page, TaskSummary,
    ThreadSummary, all_loop, resync,
};
pub use state::{
    BotState, ConversationState, DomainState, IdempotencyBindingState, LifecycleState, TaskState,
    TaskStatus, ThreadState,
};
pub use subscription::{
    MAX_EVENTS_PER_STREAM, StreamEvent, Subscription, SubscriptionManager, TaskResult,
};

#![forbid(unsafe_code)]

pub mod mutation;
pub mod state;
pub mod persistence;
pub mod outcome;
pub mod query;
pub mod interface;
pub mod subscription;
pub mod membership;
pub mod delegation;

pub use delegation::{DelegationManager, DelegationRecord, DelegationStatus, DelegationSummary};
pub use interface::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};
pub use membership::{MembershipManager, MembershipRecord, MembershipSummary};
pub use mutation::{AppError, ApplicationMutator};
pub use outcome::{DomainOutcome, cas_if_revision, resolve_outcome};
pub use persistence::ApplicationStateStore;
pub use query::{
    AllLoopResult, ApplicationQuery, BotSummary, ConversationSummary, Page, TaskSummary,
    ThreadSummary, all_loop, resync,
};
pub use state::{
    BotState, ChannelState, ConversationOwner, ConversationState, DomainState,
    IdempotencyBindingState, LifecycleState, MemoryAssertionStatus, MemoryRevisionState,
    MemoryState, MessageState, ProcessLifecycle, ProcessState, ProjectLifecycle, ProjectState,
    SideEffectState, SideEffectStatus, TaskState, TaskStatus, ThreadState,
};
pub use subscription::{
    MAX_EVENTS_PER_STREAM, StreamEvent, Subscription, SubscriptionManager, TaskResult,
};

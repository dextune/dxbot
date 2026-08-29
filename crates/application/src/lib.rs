#![forbid(unsafe_code)]

pub mod delegation;
pub mod execution;
pub mod interface;
pub mod membership;
pub mod mutation;
pub mod outcome;
pub mod persistence;
pub mod query;
pub mod state;
pub mod subscription;
pub mod watch;

pub use delegation::{DelegationManager, DelegationRecord, DelegationStatus, DelegationSummary};
pub use execution::{
    ExecutionCandidate, ExecutionEvidenceInput, ExecutionResultInput, ExecutionRunFence,
    ExecutionWork,
};
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
    BotPolicyBindings, BotState, ChannelState, ContextMemoryRef, ContextPlan, ConversationOwner,
    ConversationState, DeclassificationRecord, DomainState, ExecutionAuditIntent,
    ExecutionAuditPhase, ExecutionEvidence, ExecutionState, ExecutionStatus,
    IdempotencyBindingState, LifecycleState, MemoryAssertionStatus, MemoryRevisionState,
    MemoryState, MessageState, ProcessLifecycle, ProcessState, ProjectLifecycle, ProjectState,
    ProviderBinding, SideEffectResolveError, SideEffectState, SideEffectStatus,
    TaskControlDirective, TaskExecutionConstraints, TaskState, TaskStatus, ThreadState,
};
pub use subscription::{
    MAX_EVENTS_PER_STREAM, StreamEvent, Subscription, SubscriptionManager, TaskResult,
};

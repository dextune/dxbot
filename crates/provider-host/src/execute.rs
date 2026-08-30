//! Canonical async Execute contract (`DXB-DEL-068` H2).
//!
//! Every provider implementation — the direct MiniMax/DeepSeek adapters, the
//! synthetic test canary, and the future DeepSeek Harness ACP adapter — drives
//! the same async SPI. The Common provider host owns lifecycle, generation
//! selection, deadline/cancellation, and output bounding; extensions map only
//! model-specific request/event shapes.
//!
//! The contract is intentionally free of any concrete transport, credential,
//! or protocol type. `ProviderHost` (see `harness.rs`) binds a registration's
//! immutable protocol/transport to an implementation and runs it on the
//! Runtime-owned Tokio handle. There is no host-owned runtime and no nested
//! `block_on`; the Runtime scheduler boundary is the sole synchronous bridge.

#![forbid(unsafe_code)]

use std::future::Future;
use std::pin::Pin;

use dxbot_core::types::ProviderId;

use crate::cancellation::{CancellationToken, UsageInfo};

/// Immutable request handed to an [`ExecuteProvider`].
///
/// The request carries the exact selected provider identity and generation so
/// an implementation can fence any late activity against the registration that
/// was captured at admission time. It never carries raw credentials, a mutable
/// Domain handle, or an external session identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteRequest {
    /// Provider selected by exact id + generation at admission.
    pub provider_id: ProviderId,
    /// Generation captured at admission; the host fences against replacement.
    pub provider_generation: i64,
    /// Bounded task intent.
    pub intent: String,
    /// Bounded provider-neutral context.
    pub context: String,
    /// Optional output budget in model tokens.
    pub budget: Option<u64>,
    /// Optional absolute wall-clock deadline (unix seconds).
    pub deadline: Option<i64>,
    /// Immutable Context Plan permission references propagated from the
    /// committed plan (`DXB-DEL-068` H10 Task 4). Bounded, opaque refs — never
    /// raw permission material. Empty when the plan carries none.
    pub permission_refs: Vec<String>,
    /// Immutable Context Plan resource references / budget ref propagated from
    /// the committed plan. Bounded, opaque refs — never raw grants.
    pub resource_refs: Vec<String>,
    /// The Application Side Effect ledger reference for this attempt, so the
    /// provider call can attribute per-tool effects to the durable side-effect
    /// record. `None` when the caller does not track side effects.
    pub side_effect_ref: Option<String>,
    /// The Core lease reference held for this attempt. Bounded, opaque.
    pub core_lease_ref: Option<String>,
    /// The request-scoped allow-once authority grant (`DXB-DEL-068` H10
    /// Task 7), resolved by the scheduler from an Approved runtime-security
    /// Approval bound to action `provider-tool-allow-once`, target exact
    /// execution, positive generation. `None` means allow-once is NOT
    /// authorized for this attempt and the provider must default to reject.
    /// The static `PermissionPolicy::AllowOnce` is only an upper bound; this
    /// grant is the necessary authority.
    pub allow_once_grant: Option<AllowOnceGrant>,
}

/// A request-scoped authority grant that a specific allow-once tool permission
/// is authorized for exactly one execution (`DXB-DEL-068` H10 Task 7).
///
/// The scheduler mints this only from an `Approved` runtime-security Approval
/// whose binding action is `provider-tool-allow-once`, whose target is the
/// exact execution reference, and whose policy generation is positive. The
/// provider never fabricates it and never treats a DSH/ambient approval as a
/// substitute. Its presence authorizes selecting a single `allow_once` option;
/// its absence forces the default reject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowOnceGrant {
    /// The Approval id that authorized this grant (bounded, opaque).
    pub approval_ref: String,
    /// The exact execution reference the grant is bound to.
    pub execution_ref: String,
    /// The positive policy generation the grant was minted under.
    pub policy_generation: i64,
}

impl AllowOnceGrant {
    /// Whether this grant is well-formed: non-empty refs and a positive
    /// generation. A malformed grant never authorizes an allow-once.
    pub fn is_valid(&self) -> bool {
        !self.approval_ref.trim().is_empty()
            && !self.execution_ref.trim().is_empty()
            && self.policy_generation > 0
    }
}

impl ExecuteRequest {
    /// Build a bounded request that carries no permission/resource/side-effect
    /// refs and no allow-once grant (the common case for direct HTTP adapters
    /// and tests). H10 callers set the additional fields explicitly.
    pub fn bounded(
        provider_id: ProviderId,
        provider_generation: i64,
        intent: String,
        context: String,
        budget: Option<u64>,
        deadline: Option<i64>,
    ) -> Self {
        Self {
            provider_id,
            provider_generation,
            intent,
            context,
            budget,
            deadline,
            permission_refs: Vec::new(),
            resource_refs: Vec::new(),
            side_effect_ref: None,
            core_lease_ref: None,
            allow_once_grant: None,
        }
    }
}

/// A single bounded, provider-neutral event in the ordered execution stream.
///
/// Raw protocol deltas stay inside the adapter/protocol layer. The stream is
/// monotonic in `sequence` and emits at most one terminal event; the host
/// enforces "no event after terminal".
///
/// The tool/permission/effect variants (`DXB-DEL-068` H10) carry only bounded,
/// provider-neutral references and dispositions. They never carry raw ACP/DSH
/// payloads, tool arguments, file contents, paths, or secrets, and they never
/// themselves grant authority: an allow-once decision is authorized only by a
/// DXBOT runtime-security Approval resolved by the scheduler, never by the
/// presence of one of these events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteEvent {
    /// Assistant-visible output text.
    AssistantOutput { sequence: u64, text: String },
    /// Provider reasoning trace (never treated as authority).
    Reasoning { sequence: u64, text: String },
    /// A provider-initiated tool call started. `correlation` is a bounded,
    /// adapter-local opaque id used to associate subsequent lifecycle/effect
    /// events; `tool` is a bounded, neutral tool-kind label (never raw args).
    ToolCallStarted {
        sequence: u64,
        correlation: String,
        tool: String,
    },
    /// A provider requested permission for a side-effecting action. This is a
    /// *request*, not a grant: the runtime default is reject, and any allow
    /// requires a DXBOT authority grant resolved out of band.
    PermissionRequested {
        sequence: u64,
        correlation: String,
        tool: String,
    },
    /// A tool side effect reached a bounded disposition. `correlation` ties it
    /// to the originating `ToolCallStarted`/`PermissionRequested`.
    ToolEffect {
        sequence: u64,
        correlation: String,
        disposition: ToolDisposition,
    },
    /// Terminal usage/accounting event; at most one per execution.
    Usage { usage: UsageInfo },
}

/// Bounded, provider-neutral disposition of a tool side effect (`DXB-DEL-068`
/// H10). No variant carries provider text, paths, or raw payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolDisposition {
    /// The effect was requested/prepared but not yet dispatched.
    Prepared,
    /// The effect was dispatched to the tool/environment.
    Dispatched,
    /// The effect was confirmed complete.
    Confirmed,
    /// The effect's completion is unknown (crash/timeout mid-effect); the
    /// caller must reconcile and must never blindly retry.
    Unknown,
    /// The effect was rejected (default, or no authority grant).
    Rejected,
}

/// Typed terminal disposition of an execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteOutcome {
    /// Bounded, non-empty final output plus provider-neutral evidence.
    Succeeded { output: String, usage: UsageInfo },
    /// Caller/durable cancellation observed; no failure retry implied.
    Cancelled,
}

/// Result of a completed execution attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecuteResult {
    pub provider_id: ProviderId,
    pub provider_generation: i64,
    pub outcome: ExecuteOutcome,
    /// Bounded, provider-neutral per-tool side-effect dispositions observed
    /// during the attempt (`DXB-DEL-068` H10 Task 8). Each entry carries only
    /// an adapter-local correlation id and a neutral disposition — never raw
    /// tool arguments, paths, or payloads. The scheduler folds these into the
    /// Application Side Effect ledger; an `Unknown` disposition drives
    /// RecoveryRequired with no blind retry.
    pub tool_effects: Vec<ToolEffectRecord>,
}

impl ExecuteResult {
    /// A result with no observed tool effects (the common case for direct HTTP
    /// adapters that surface no provider-initiated tool lifecycle).
    pub fn new(provider_id: ProviderId, provider_generation: i64, outcome: ExecuteOutcome) -> Self {
        Self {
            provider_id,
            provider_generation,
            outcome,
            tool_effects: Vec::new(),
        }
    }
}

/// A bounded, provider-neutral record of one tool side effect's disposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolEffectRecord {
    /// Adapter-local opaque correlation id (bounded). Never a raw external id.
    pub correlation: String,
    /// The neutral disposition the effect reached.
    pub disposition: ToolDisposition,
}

/// Stable, safe-to-surface execution failure categories.
///
/// No variant carries provider exception text, stderr, paths, prompts,
/// secrets, or raw protocol payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecuteError {
    /// The provider transport could not be reached.
    TransportUnavailable,
    /// The upstream returned a server-side failure status.
    UpstreamUnavailable { status: u16 },
    /// The upstream rate-limited the request.
    RateLimited { retry_after_seconds: Option<u64> },
    /// The request was rejected as invalid by the upstream.
    InvalidRequest { status: u16 },
    /// The response violated the expected protocol.
    ProtocolViolation,
    /// A bounded output/frame/item limit was exceeded.
    OutputExceeded,
    /// The absolute deadline elapsed.
    DeadlineExceeded,
    /// The provider stream ended without a terminal completion.
    IncompleteStream,
    /// A model-mapping or adapter-internal invariant failed.
    ExecutionFailed,
}

/// A boxed, owned future used by the object-safe [`ExecuteProvider`] trait.
pub type ExecuteFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ExecuteResult, ExecuteError>> + Send + 'a>>;

/// The single canonical async provider SPI (`DXB-DEL-068` H2).
///
/// Implementations own only model-specific request building and event mapping.
/// The host owns the immutable protocol/transport binding, deadline and
/// cancellation authority, and output bounding. Implementations must not spawn
/// their own runtime; they run on the Runtime-owned Tokio handle supplied by
/// the host.
pub trait ExecuteProvider: std::fmt::Debug + Send + Sync {
    /// Stable provider identity.
    fn id(&self) -> &ProviderId;
    /// Advertised capability class (e.g. `llm-chat`).
    fn capability(&self) -> &str;
    /// Immutable registration generation.
    fn generation(&self) -> i64;

    /// Execute one bounded attempt under the host-owned deadline/cancellation.
    fn execute<'a>(
        &'a self,
        request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a>;
}

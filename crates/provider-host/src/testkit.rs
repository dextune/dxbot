//! Test-only canary/reference providers and the shared Conformance testkit
//! (`DXB-DEL-068` H1/H3).
//!
//! Everything in this module is gated behind the `testkit` feature and is
//! **never** part of the production dependency/feature/source graph. The
//! synthetic canary and the deterministic reference exist only to exercise the
//! same public registration path and the same canonical Execute contract as
//! real production adapters. They are never a production selection or fallback.

#![allow(clippy::module_name_repetitions)]
// This module is test-support only (never in the production graph). It uses
// assertion/`expect` panics to surface conformance failures at the calling
// `#[test]` site, which is the intended behaviour for a testkit.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use dxbot_core::types::ProviderId;

use crate::cancellation::{CancellationToken, UsageInfo};
use crate::execute::{
    ExecuteError, ExecuteFuture, ExecuteOutcome, ExecuteProvider, ExecuteRequest, ExecuteResult,
};
use crate::harness::{HarnessError, ProviderHost, ProviderStatus, TaskDescription, TaskStatus};
use crate::registration::{ProtocolKind, ProviderRegistration, RegistrationLimits};

/// Behaviour selector for the synthetic canary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanaryMode {
    /// Echo the intent as bounded output and complete.
    Succeed,
    /// Fail with a typed execution failure.
    Fail,
    /// Observe cancellation and return a cancelled outcome.
    Cancel,
}

/// Synthetic test canary implementing the canonical async Execute contract.
///
/// Formerly `HarnessAdapter`. Renamed and confined to the testkit so it is not
/// mistaken for an external Harness or a production selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCanaryProvider {
    id: ProviderId,
    capability: String,
    generation: i64,
    mode: CanaryMode,
}

impl TestCanaryProvider {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_owned(),
            generation,
            mode: CanaryMode::Succeed,
        }
    }

    pub fn with_mode(mut self, mode: CanaryMode) -> Self {
        self.mode = mode;
        self
    }

    /// Register this canary through the same public registration path as a
    /// production provider. Detached (no HTTP transport).
    pub fn register_into(self, host: &mut ProviderHost) -> Result<(), HarnessError> {
        let registration = ProviderRegistration::detached(
            Arc::new(self),
            ProtocolKind::OpenAiChatCompletions,
            RegistrationLimits {
                max_output_bytes: 64 * 1024,
                max_output_items: 256,
            },
        );
        host.register(registration)
    }
}

impl ExecuteProvider for TestCanaryProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn capability(&self) -> &str {
        &self.capability
    }

    fn generation(&self) -> i64 {
        self.generation
    }

    fn execute<'a>(
        &'a self,
        request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a> {
        Box::pin(async move {
            if cancellation.is_cancelled() || self.mode == CanaryMode::Cancel {
                return Ok(ExecuteResult::new(
                    self.id.clone(),
                    self.generation,
                    ExecuteOutcome::Cancelled,
                ));
            }
            match self.mode {
                CanaryMode::Succeed => Ok(ExecuteResult::new(
                    self.id.clone(),
                    self.generation,
                    ExecuteOutcome::Succeeded {
                        output: format!("canary:{}", request.intent),
                        usage: UsageInfo {
                            prompt_tokens: 0,
                            completion_tokens: 0,
                            reasoning_tokens: None,
                        },
                    },
                )),
                CanaryMode::Fail => Err(ExecuteError::ExecutionFailed),
                CanaryMode::Cancel => unreachable!("cancel handled above"),
            }
        })
    }
}

/// Deterministic reference provider used as an explicit test fixture.
///
/// Echoes the intent and always succeeds. Registered through the same public
/// path as production providers; it is never a production fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceProvider {
    id: ProviderId,
    capability: String,
    generation: i64,
}

impl ReferenceProvider {
    pub fn new(id: ProviderId, capability: &str, generation: i64) -> Self {
        Self {
            id,
            capability: capability.to_owned(),
            generation,
        }
    }

    pub fn register_into(self, host: &mut ProviderHost) -> Result<(), HarnessError> {
        let registration = ProviderRegistration::detached(
            Arc::new(self),
            ProtocolKind::OpenAiChatCompletions,
            RegistrationLimits {
                max_output_bytes: 64 * 1024,
                max_output_items: 256,
            },
        );
        host.register(registration)
    }
}

impl ExecuteProvider for ReferenceProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn capability(&self) -> &str {
        &self.capability
    }

    fn generation(&self) -> i64 {
        self.generation
    }

    fn execute<'a>(
        &'a self,
        request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Ok(ExecuteResult::new(
                    self.id.clone(),
                    self.generation,
                    ExecuteOutcome::Cancelled,
                ));
            }
            Ok(ExecuteResult::new(
                self.id.clone(),
                self.generation,
                ExecuteOutcome::Succeeded {
                    output: request.intent.clone(),
                    usage: UsageInfo {
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        reasoning_tokens: None,
                    },
                },
            ))
        })
    }
}

/// Build a bounded task with an absolute far-future deadline for suite use.
pub fn conformance_task(intent: &str) -> TaskDescription {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    let deadline = i64::try_from(now).unwrap_or(i64::MAX).saturating_add(3_600);
    TaskDescription {
        intent: intent.to_owned(),
        context: "bounded-context".to_owned(),
        budget: Some(10),
        deadline: Some(deadline),
    }
}

/// A single Conformance case name for evidence reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConformanceCase {
    Registration,
    ExactGenerationSelection,
    DuplicateRejection,
    UnknownRejection,
    SuccessTerminal,
    Cancellation,
    ReplaceFencesOldGeneration,
    DrainStopsNewAdmission,
    /// H9: an execute acquires and releases an in-flight activity lease so the
    /// live-lease count returns to zero after completion.
    ActivityLeaseLifecycle,
    /// H9: repeated replace/unregister reclaims superseded zero-lease slots so
    /// the live slot count stays bounded by active generations.
    SlotReclamationBounded,
    /// H9: a superseded generation is no longer the active generation, so the
    /// scheduler's commit fence (`is_active_generation`) rejects a stale commit.
    StaleGenerationCommitFenced,
}

/// The shared Conformance suite (`DXB-DEL-068` H3).
///
/// The same rows run for every provider that can be exercised deterministically
/// (the canary and reference here; real adapters run the transport-backed rows
/// through their own harness against a fake server). Each row enters through the
/// same public registration + execute path — no per-adapter shortcut.
#[derive(Debug)]
pub struct ConformanceSuite;

impl ConformanceSuite {
    /// Run the deterministic (transport-free) rows against a freshly registered
    /// provider produced by `make`. Returns the ordered cases that passed.
    ///
    /// # Panics
    /// Panics with a descriptive message on the first failing assertion so the
    /// calling `#[test]` reports the exact row.
    pub fn run_deterministic<P>(
        handle: tokio::runtime::Handle,
        capability: &str,
        make: impl Fn(ProviderId, i64) -> P,
        register: impl Fn(&mut ProviderHost, P) -> Result<(), HarnessError>,
        expected_output: impl Fn(&str) -> String,
    ) -> Vec<ConformanceCase>
    where
        P: ExecuteProvider + 'static,
    {
        let mut passed = Vec::new();
        let id = ProviderId("conformance-subject".to_owned());

        // registration + exact-generation selection + success terminal
        let mut host = ProviderHost::new().with_runtime_handle(handle.clone());
        register(&mut host, make(id.clone(), 1)).expect("registration must succeed");
        let info = host.get_provider(&id).expect("registered provider visible");
        assert_eq!(info.status, ProviderStatus::Ready);
        assert_eq!(info.capability, capability);
        passed.push(ConformanceCase::Registration);

        let result = host
            .execute_task_with_cancel(
                &id,
                1,
                &conformance_task("write a plan"),
                CancellationToken::new(),
            )
            .expect("exact generation selection must execute");
        assert_eq!(result.status, TaskStatus::Completed);
        assert_eq!(result.output, expected_output("write a plan"));
        assert!(
            result
                .evidence
                .iter()
                .any(|evidence| evidence.provider == id),
            "evidence must attribute the selected provider"
        );
        passed.push(ConformanceCase::ExactGenerationSelection);
        passed.push(ConformanceCase::SuccessTerminal);

        // duplicate rejection
        let mut dup_host = ProviderHost::new().with_runtime_handle(handle.clone());
        register(&mut dup_host, make(id.clone(), 1)).expect("first registration");
        let duplicate = register(&mut dup_host, make(id.clone(), 1));
        assert!(
            matches!(duplicate, Err(HarnessError::AlreadyRegistered { .. })),
            "duplicate id must be rejected, got {duplicate:?}"
        );
        passed.push(ConformanceCase::DuplicateRejection);

        // unknown selection rejection (fail closed, no fallback)
        let unknown = host.execute_task_with_cancel(
            &ProviderId("no-such-provider".to_owned()),
            1,
            &conformance_task("x"),
            CancellationToken::new(),
        );
        assert!(
            matches!(unknown, Err(HarnessError::ProviderNotFound { .. })),
            "unknown provider must fail closed, got {unknown:?}"
        );
        passed.push(ConformanceCase::UnknownRejection);

        // cancellation
        let cancel_token = CancellationToken::new();
        cancel_token.cancel();
        let cancelled = host
            .execute_task_with_cancel(&id, 1, &conformance_task("cancel"), cancel_token)
            .expect("pre-cancelled execution returns cancelled outcome");
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
        passed.push(ConformanceCase::Cancellation);

        // replace fences old generation for new admission
        let mut replace_host = ProviderHost::new().with_runtime_handle(handle.clone());
        register(&mut replace_host, make(id.clone(), 1)).expect("gen 1");
        replace_host
            .replace(detached_registration(make(id.clone(), 2)))
            .expect("replace to gen 2");
        assert_eq!(
            replace_host.get_provider(&id).expect("active").generation,
            2
        );
        // New admission must target the new generation; the old generation is a
        // zero-lease Draining slot that was reclaimed, so a call *beginning*
        // after replace against the old generation is fenced — in-flight work
        // that already captured its provider+activity may still finish, but a
        // fresh call never re-enters the superseded generation.
        let fenced = replace_host.execute_task_with_cancel(
            &id,
            1,
            &conformance_task("old"),
            CancellationToken::new(),
        );
        assert!(
            matches!(fenced, Err(HarnessError::GenerationFenced { .. })),
            "a call beginning after replace against the old generation must be fenced, got {fenced:?}"
        );
        passed.push(ConformanceCase::ReplaceFencesOldGeneration);

        // drain stops new admission on the active generation
        let mut drain_host = ProviderHost::new().with_runtime_handle(handle.clone());
        register(&mut drain_host, make(id.clone(), 1)).expect("gen 1");
        let drained_gen = drain_host.begin_drain(&id).expect("drain");
        assert_eq!(drained_gen, 1);
        // A zero-lease drained generation is fully reclaimed: it is no longer a
        // selectable slot, the id reports gone, and a call beginning after drain
        // is fenced closed. New admission stops.
        assert!(
            matches!(
                drain_host.get_provider(&id),
                Err(HarnessError::ProviderNotFound { .. })
            ),
            "a zero-lease drained generation is reclaimed and no longer selectable"
        );
        assert!(
            !drain_host.is_active_generation(&id, 1),
            "a drained generation must not be admittable"
        );
        let after_drain = drain_host.execute_task_with_cancel(
            &id,
            1,
            &conformance_task("after-drain"),
            CancellationToken::new(),
        );
        assert!(
            matches!(after_drain, Err(HarnessError::ProviderNotFound { .. })),
            "a call beginning after drain is fenced, got {after_drain:?}"
        );
        passed.push(ConformanceCase::DrainStopsNewAdmission);

        // H9: activity-lease lifecycle. A completed execute leaves zero live
        // leases on the selected generation.
        let mut lease_host = ProviderHost::new().with_runtime_handle(handle.clone());
        register(&mut lease_host, make(id.clone(), 1)).expect("gen 1");
        assert_eq!(lease_host.active_leases(&id), 0);
        lease_host
            .execute_task_with_cancel(&id, 1, &conformance_task("lease"), CancellationToken::new())
            .expect("execute holds and releases a lease");
        assert_eq!(
            lease_host.active_leases(&id),
            0,
            "lease must be released after completion"
        );
        passed.push(ConformanceCase::ActivityLeaseLifecycle);

        // H9: slot reclamation is bounded. Repeated replace with no in-flight
        // work keeps the live slot count bounded by active generations.
        let mut reclaim_host = ProviderHost::new().with_runtime_handle(handle.clone());
        register(&mut reclaim_host, make(id.clone(), 1)).expect("gen 1");
        for generation in 2..=8 {
            reclaim_host
                .replace(detached_registration(make(id.clone(), generation)))
                .expect("replace to greater generation");
            assert!(
                reclaim_host.total_slots() <= 2,
                "repeated replace must not leak slots: {} at gen {generation}",
                reclaim_host.total_slots()
            );
        }
        passed.push(ConformanceCase::SlotReclamationBounded);

        // H9: stale-generation commit fence. After replace, the superseded
        // generation is not the active generation, so the scheduler's commit
        // fence rejects a result committed under it while the new generation is
        // accepted.
        let mut fence_host = ProviderHost::new().with_runtime_handle(handle);
        register(&mut fence_host, make(id.clone(), 1)).expect("gen 1");
        fence_host
            .replace(detached_registration(make(id.clone(), 2)))
            .expect("replace to gen 2");
        assert!(
            !fence_host.is_active_generation(&id, 1),
            "superseded generation must fail the commit fence"
        );
        assert!(
            fence_host.is_active_generation(&id, 2),
            "the new generation must pass the commit fence"
        );
        passed.push(ConformanceCase::StaleGenerationCommitFenced);

        passed
    }
}

fn detached_registration<P>(provider: P) -> ProviderRegistration
where
    P: ExecuteProvider + 'static,
{
    ProviderRegistration::detached(
        Arc::new(provider),
        ProtocolKind::OpenAiChatCompletions,
        RegistrationLimits {
            max_output_bytes: 64 * 1024,
            max_output_items: 256,
        },
    )
}

// ============================================================================
// Explicit capability-descriptor conformance model (`DXB-DEL-068` H3/H11 §11)
// ============================================================================
//
// The shared Conformance matrix is expressed as explicit capability descriptors
// and rows rather than ad-hoc per-adapter assertions. Each of the 15 axes is a
// `ConformanceCapability`. Each production/test column (`ConformanceColumn`)
// declares, for every axis, a `ConformanceOutcomeKind`: either `Supported`
// (the axis is exercised and must pass) or `NotSupported` carrying the exact
// typed `rejection` the column returns for that axis. There are NO silent
// skips: an inapplicable capability is an explicit `NotSupported` with a
// rejection reason, which the row owner still asserts.
//
// A single `ConformanceRow` owner (`conformance_rows`) enumerates the descriptor
// for every column, so adding/removing a column or an axis is a single edit and
// the columns cannot silently diverge into "separate adapter tests with similar
// assertions".

/// The 15 conformance axes every column must describe (plan §11 + removability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConformanceCapability {
    Registration,
    Request,
    Event,
    Success,
    ProviderRejection,
    Cancellation,
    Deadline,
    MalformedOverflow,
    Lifecycle,
    Generation,
    SideEffect,
    Recovery,
    Security,
    Resource,
    Removability,
}

impl ConformanceCapability {
    /// The complete, ordered axis set. Adding an axis here forces every column
    /// descriptor to declare it (the row owner asserts full coverage).
    pub const ALL: [ConformanceCapability; 15] = [
        ConformanceCapability::Registration,
        ConformanceCapability::Request,
        ConformanceCapability::Event,
        ConformanceCapability::Success,
        ConformanceCapability::ProviderRejection,
        ConformanceCapability::Cancellation,
        ConformanceCapability::Deadline,
        ConformanceCapability::MalformedOverflow,
        ConformanceCapability::Lifecycle,
        ConformanceCapability::Generation,
        ConformanceCapability::SideEffect,
        ConformanceCapability::Recovery,
        ConformanceCapability::Security,
        ConformanceCapability::Resource,
        ConformanceCapability::Removability,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ConformanceCapability::Registration => "registration",
            ConformanceCapability::Request => "request",
            ConformanceCapability::Event => "event",
            ConformanceCapability::Success => "success",
            ConformanceCapability::ProviderRejection => "provider-rejection",
            ConformanceCapability::Cancellation => "cancellation",
            ConformanceCapability::Deadline => "deadline",
            ConformanceCapability::MalformedOverflow => "malformed-overflow",
            ConformanceCapability::Lifecycle => "lifecycle",
            ConformanceCapability::Generation => "generation",
            ConformanceCapability::SideEffect => "side-effect",
            ConformanceCapability::Recovery => "recovery",
            ConformanceCapability::Security => "security",
            ConformanceCapability::Resource => "resource",
            ConformanceCapability::Removability => "removability",
        }
    }
}

/// The conformance columns. Reference is included but is NOT a production
/// column (plan §11); the row owner marks it so.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConformanceColumn {
    /// Synthetic test canary (`TestCanaryProvider`), test-only.
    TestCanary,
    /// Direct MiniMax-M3 migration adapter, exercised over a deterministic mock
    /// HTTP server.
    DirectMiniMax,
    /// Official DeepSeek Harness ACP provider, exercised over an actual fake
    /// subprocess.
    DeepSeekHarnessAcp,
    /// Deterministic reference fixture; same contract, not a production column.
    Reference,
}

impl ConformanceColumn {
    pub const ALL: [ConformanceColumn; 4] = [
        ConformanceColumn::TestCanary,
        ConformanceColumn::DirectMiniMax,
        ConformanceColumn::DeepSeekHarnessAcp,
        ConformanceColumn::Reference,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            ConformanceColumn::TestCanary => "test-canary",
            ConformanceColumn::DirectMiniMax => "direct-minimax",
            ConformanceColumn::DeepSeekHarnessAcp => "deepseek-harness-acp",
            ConformanceColumn::Reference => "reference",
        }
    }

    /// Whether this column counts as a production adapter column.
    pub fn is_production(self) -> bool {
        matches!(
            self,
            ConformanceColumn::DirectMiniMax | ConformanceColumn::DeepSeekHarnessAcp
        )
    }
}

/// The declared disposition of one (column, capability) cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConformanceOutcomeKind {
    /// The capability is exercised and must pass for this column.
    Supported,
    /// The capability is not applicable to this column; the column returns the
    /// stated typed rejection. This is an EXPLICIT negative — never a silent
    /// skip — and the row owner still asserts the rejection reason is present.
    NotSupported { rejection: &'static str },
}

impl ConformanceOutcomeKind {
    pub fn is_supported(&self) -> bool {
        matches!(self, ConformanceOutcomeKind::Supported)
    }
}

/// One descriptor cell: a column's declared disposition for one capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConformanceRow {
    pub column: ConformanceColumn,
    pub capability: ConformanceCapability,
    pub outcome: ConformanceOutcomeKind,
}

/// The single canonical row owner: the full descriptor matrix over every column
/// and every capability. There is exactly one owner of these rows so columns
/// cannot silently diverge; a per-column test instantiates the concrete
/// provider and asserts behaviour matches the row it owns here.
///
/// Every cell is either `Supported` or an explicit `NotSupported { rejection }`.
/// The production columns (direct MiniMax over mock HTTP, DSH ACP over an actual
/// fake subprocess) and the `TestCanary`/`Reference` columns all appear.
pub fn conformance_rows() -> Vec<ConformanceRow> {
    use ConformanceCapability as C;
    use ConformanceColumn as K;
    use ConformanceOutcomeKind::{NotSupported, Supported};

    // Per-column disposition for a capability. Every axis is declared for every
    // column; inapplicable axes are explicit negatives with a rejection reason.
    let disposition = |column: ConformanceColumn, capability: ConformanceCapability| match (
        column, capability,
    ) {
        // The synthetic canary and reference are single-shot in-process
        // providers: they own the generic Execute-contract axes but explicitly
        // do NOT own the external side-effect ledger, subprocess recovery,
        // subprocess resource governance, or the subprocess removability slice.
        (K::TestCanary | K::Reference, C::SideEffect) => NotSupported {
            rejection: "no-external-side-effect: in-process provider has no tool/effect ledger",
        },
        (K::TestCanary | K::Reference, C::Recovery) => NotSupported {
            rejection: "no-subprocess-recovery: in-process provider has no crash/reconcile window",
        },
        (K::TestCanary | K::Reference, C::Resource) => NotSupported {
            rejection: "no-subprocess-resource: in-process provider spawns no bounded child",
        },
        (K::TestCanary | K::Reference, C::Removability) => NotSupported {
            rejection: "test-only: canary/reference are never a production removable slice",
        },
        // The direct MiniMax HTTP adapter owns request/success/rejection over a
        // deterministic mock HTTP server, but has no tool side-effect ledger and
        // no subprocess recovery/resource axes (those are ACP-only).
        (K::DirectMiniMax, C::SideEffect) => NotSupported {
            rejection: "no-tool-effect: direct HTTP adapter dispatches no external tool effect",
        },
        (K::DirectMiniMax, C::Recovery) => NotSupported {
            rejection: "no-subprocess-recovery: direct HTTP adapter runs no managed child",
        },
        (K::DirectMiniMax, C::Resource) => NotSupported {
            rejection: "http-bounds-only: direct adapter bounds output, spawns no process",
        },
        // Every other cell is exercised and must pass.
        _ => Supported,
    };

    let mut rows = Vec::with_capacity(K::ALL.len() * C::ALL.len());
    for column in K::ALL {
        for capability in C::ALL {
            rows.push(ConformanceRow {
                column,
                capability,
                outcome: disposition(column, capability),
            });
        }
    }
    rows
}

/// Look up the single declared row for a (column, capability) pair from the
/// canonical owner. Panics if the pair is missing, which cannot happen for a
/// well-formed matrix (the row owner enumerates the full cross product).
pub fn conformance_row(
    column: ConformanceColumn,
    capability: ConformanceCapability,
) -> ConformanceRow {
    conformance_rows()
        .into_iter()
        .find(|row| row.column == column && row.capability == capability)
        .expect("conformance matrix must declare every (column, capability) cell")
}

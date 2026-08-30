//! Provider host: immutable registration owner and Common execution authority
//! (`DXB-DEL-068` H1/H2/H4/H9).
//!
//! The host owns provider registration/lifecycle and drives the canonical
//! async [`ExecuteProvider`](crate::execute) contract. It does **not** own a
//! Tokio runtime: the Runtime Host supplies a [`tokio::runtime::Handle`] at the
//! scheduler boundary, so cancellation, timeout, and shutdown propagate on the
//! Runtime task tree instead of a nested adapter runtime.
//!
//! Selection is by exact `ProviderId` + generation. There is no production
//! reference/canary fallback: an unavailable, replaced, or absent provider is a
//! typed failure, never a silent substitution.
//!
//! Concurrency (`DXB-DEL-068` H9). The host is designed to be shared as a
//! single `Arc<ProviderHost>` across the Runtime scheduler, the control server,
//! and lifecycle callers. Every registration/replace/drain/unregister/reclaim
//! API takes `&self` and mutates a private synchronized slot registry behind an
//! [`RwLock`]. Execute admission observes `Ready` and acquires a per-slot RAII
//! activity lease **atomically under the registry lock**, then clones the
//! provider and releases the lock before the (blocking) `Handle::block_on` — no
//! host or slot lock is ever held across the provider call. A replace/drain/
//! unregister beginning after that admission fences new work while the captured
//! in-flight call, which already holds provider + activity, is allowed to
//! finish under its old generation.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use dxbot_core::types::ProviderId;

use crate::cancellation::CancellationToken;
use crate::execute::{ExecuteError, ExecuteOutcome, ExecuteRequest, ExecuteResult};
use crate::registration::ProviderRegistration;

/// Poll cadence used by [`ProviderHost::drain_to_quiescence`] while waiting for
/// live in-flight leases to return to zero. Bounded and small so drain latency
/// is dominated by the caller-supplied deadline, never by this interval.
const DRAIN_POLL_INTERVAL: Duration = Duration::from_millis(5);

/// A bounded task description executed by a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDescription {
    pub intent: String,
    pub context: String,
    pub budget: Option<u64>,
    pub deadline: Option<i64>,
}

/// A piece of provider-produced evidence attached to a [`TaskResult`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub provider: ProviderId,
    pub observation: String,
}

/// Terminal execution status of a bounded task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Completed,
    Failed,
    Cancelled,
}

/// The result of executing a [`TaskDescription`] through a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskResult {
    pub output: String,
    pub evidence: Vec<Evidence>,
    pub status: TaskStatus,
    /// Bounded, provider-neutral per-tool side-effect dispositions observed
    /// during the attempt (`DXB-DEL-068` H10). The Runtime scheduler folds
    /// these into the Application Side Effect ledger; an `Unknown` disposition
    /// forces RecoveryRequired with no blind retry.
    pub tool_effects: Vec<crate::execute::ToolEffectRecord>,
}

/// Availability of a registered provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Ready,
    Draining,
    Unavailable,
}

/// Read-only registration record for a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderInfo {
    pub id: ProviderId,
    pub capability: String,
    pub generation: i64,
    pub status: ProviderStatus,
}

/// Errors produced by the provider host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HarnessError {
    AlreadyRegistered {
        id: ProviderId,
    },
    InvalidProvider {
        reason: String,
    },
    ProviderUnavailable {
        id: ProviderId,
    },
    ProviderNotFound {
        id: ProviderId,
    },
    /// The captured registration generation no longer matches (replaced/drained).
    GenerationFenced {
        id: ProviderId,
    },
    DeadlineExceeded,
    RateLimited {
        id: ProviderId,
        retry_after_seconds: Option<u64>,
    },
    UpstreamUnavailable {
        id: ProviderId,
        status: u16,
    },
    TransportUnavailable {
        id: ProviderId,
    },
    InvalidRequest {
        id: ProviderId,
        status: u16,
    },
    ProtocolViolation {
        id: ProviderId,
    },
    OutputExceeded {
        id: ProviderId,
    },
    Cancelled {
        id: ProviderId,
    },
    ExecutionFailed {
        id: ProviderId,
        reason: String,
    },
    /// No provider is configured for execution (fail closed, never fall back).
    NoProviderConfigured,
}

/// Lifecycle state of a slot in the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotState {
    Ready,
    Draining,
}

/// The set of live in-flight cancellation tokens tracked for a slot, keyed by a
/// process-unique activity id. Each admitted execution inserts its token under
/// a fresh id; the matching RAII [`ProviderActivityRef`] removes exactly its
/// own entry on drop. A drain/unregister that needs to force quiescence cancels
/// every token still present here (`DXB-DEL-068` H9).
type TokenRegistry = Arc<Mutex<HashMap<u64, CancellationToken>>>;

/// A shared slot: the immutable registration, its lifecycle state, a live
/// in-flight activity lease count, and the set of in-flight cancellation tokens
/// tracked per activity id.
///
/// The lease count is the number of executions that captured this exact
/// `(id, generation)` slot and have not yet released. A `Draining` slot with a
/// zero lease count is reclaimable; a `Draining` slot with a positive lease
/// count is retained so its captured in-flight calls can complete under the old
/// generation fence, but it never admits new activity. The counter and token
/// map are shared via `Arc` so a lease can be released — and its token removed
/// — from the drop of a guard that outlives the registry lock that acquired it.
#[derive(Debug)]
struct Slot {
    registration: ProviderRegistration,
    state: SlotState,
    leases: Arc<AtomicUsize>,
    tokens: TokenRegistry,
}

impl Slot {
    fn new(registration: ProviderRegistration, state: SlotState) -> Self {
        Self {
            registration,
            state,
            leases: Arc::new(AtomicUsize::new(0)),
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Current live in-flight lease count for this slot.
    fn live_leases(&self) -> usize {
        self.leases.load(Ordering::Acquire)
    }

    /// Cancel every in-flight token tracked for this slot. Used by the bounded
    /// drain/unregister timeout path so a parked call is asked to unwind.
    fn cancel_all_tokens(&self) {
        if let Ok(tokens) = self.tokens.lock() {
            for token in tokens.values() {
                token.cancel();
            }
        }
    }
}

/// A RAII in-flight activity lease held for the life of one execution against a
/// captured `(id, generation)` slot.
///
/// Acquiring a lease increments the slot's atomic counter and registers the
/// call's [`CancellationToken`] under a process-unique activity id. Dropping it
/// (on any path — success, failure, panic-unwind, or cancellation) decrements
/// the counter and removes exactly that token. A slot is only reclaimed once
/// its lease count returns to zero, so a `Draining` generation whose in-flight
/// call is still running can never be disposed out from under that call.
#[derive(Debug)]
pub struct ProviderActivityRef {
    id: ProviderId,
    generation: i64,
    activity_id: u64,
    leases: Arc<AtomicUsize>,
    tokens: TokenRegistry,
    released: bool,
}

impl ProviderActivityRef {
    /// Acquire a lease atomically: bump the lease counter and record the call's
    /// cancellation token under `activity_id`. Must be called while the registry
    /// write lock is held so admission and lease acquisition are one atomic step.
    fn acquire(
        id: ProviderId,
        generation: i64,
        activity_id: u64,
        leases: &Arc<AtomicUsize>,
        tokens: &TokenRegistry,
        cancellation: &CancellationToken,
    ) -> Self {
        leases.fetch_add(1, Ordering::AcqRel);
        if let Ok(mut guard) = tokens.lock() {
            guard.insert(activity_id, cancellation.clone());
        }
        Self {
            id,
            generation,
            activity_id,
            leases: Arc::clone(leases),
            tokens: Arc::clone(tokens),
            released: false,
        }
    }

    /// The provider id this activity is fenced to.
    pub fn provider_id(&self) -> &ProviderId {
        &self.id
    }

    /// The captured generation this activity is fenced to. A result committed
    /// against a superseded generation must be rejected by the caller.
    pub fn generation(&self) -> i64 {
        self.generation
    }
}

impl Drop for ProviderActivityRef {
    fn drop(&mut self) {
        if !self.released {
            if let Ok(mut guard) = self.tokens.lock() {
                guard.remove(&self.activity_id);
            }
            self.leases.fetch_sub(1, Ordering::AcqRel);
            self.released = true;
        }
    }
}

/// The private synchronized slot registry. All mutation of the provider set
/// happens here under a single [`RwLock`], so concurrent lifecycle operations
/// on a shared `Arc<ProviderHost>` are serialized and never observe a torn view.
#[derive(Debug, Default)]
struct Registry {
    slots: Vec<Slot>,
}

/// Provider host and Common execution authority.
///
/// Registrations are immutable once published. Replacement publishes a strictly
/// greater `Ready` generation and transitions every previous `Ready` generation
/// to `Draining` (no-new-admission) under one write lock; a zero-lease draining
/// generation is reclaimed immediately, so no superseded generation stays
/// selectable. Unregister uses the same drain path and removes every zero-lease
/// generation for the id. In-flight work that already holds a provider clone and
/// an activity lease may complete under its captured generation; any call
/// beginning after the transition is fenced. Late results are fenced by
/// generation at commit time via [`ProviderHost::is_active_generation`].
///
/// The host is safe to share as one `Arc<ProviderHost>`: the registry is behind
/// an `RwLock`, the runtime handle behind an `RwLock`, and every public method
/// takes `&self`.
#[derive(Debug)]
pub struct ProviderHost {
    handle: RwLock<Option<tokio::runtime::Handle>>,
    registry: RwLock<Registry>,
    next_activity_id: AtomicU64,
}

impl Default for ProviderHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderHost {
    /// Create a host without a bound runtime handle. Execution requires a handle
    /// to be installed via [`ProviderHost::with_runtime_handle`]; registration
    /// and inspection work without one.
    pub fn new() -> Self {
        Self {
            handle: RwLock::new(None),
            registry: RwLock::new(Registry::default()),
            next_activity_id: AtomicU64::new(1),
        }
    }

    /// Install the Runtime-owned Tokio handle used to drive async execution.
    #[must_use]
    pub fn with_runtime_handle(self, handle: tokio::runtime::Handle) -> Self {
        if let Ok(mut guard) = self.handle.write() {
            *guard = Some(handle);
        }
        self
    }

    /// Install or replace the Runtime-owned Tokio handle in place. Takes `&self`
    /// so a shared host can have its handle installed after construction.
    pub fn set_runtime_handle(&self, handle: tokio::runtime::Handle) {
        if let Ok(mut guard) = self.handle.write() {
            *guard = Some(handle);
        }
    }

    /// Register an immutable provider registration. Fails closed on duplicate
    /// id or invalid descriptor.
    pub fn register(&self, registration: ProviderRegistration) -> Result<(), HarnessError> {
        let provider = registration.provider();
        let id = provider.id().clone();
        validate_provider(&id, provider.capability(), provider.generation())?;
        let mut registry = self.registry_write();
        if active_slot(&registry.slots, &id).is_some() {
            return Err(HarnessError::AlreadyRegistered { id });
        }
        registry
            .slots
            .push(Slot::new(registration, SlotState::Ready));
        Ok(())
    }

    /// Atomically replace the active registration for an id with a strictly
    /// greater generation, under one registry write lock.
    ///
    /// Every currently-`Ready` generation for the id transitions to `Draining`
    /// (no new admission) and the new `Ready` registration is published in the
    /// same critical section, so from any caller's perspective the swap is
    /// atomic. A `Draining` generation with live in-flight leases is retained so
    /// its captured calls complete under the old fence; every zero-lease
    /// `Draining` generation is reclaimed immediately, so no superseded
    /// generation remains selectable and repeated replacement is bounded by the
    /// number of `Ready` plus actually-leased draining generations.
    pub fn replace(&self, registration: ProviderRegistration) -> Result<(), HarnessError> {
        let provider = registration.provider();
        let id = provider.id().clone();
        let new_generation = provider.generation();
        validate_provider(&id, provider.capability(), new_generation)?;
        let mut registry = self.registry_write();
        let Some(current) = active_slot(&registry.slots, &id) else {
            return Err(HarnessError::ProviderNotFound { id });
        };
        if new_generation <= current.registration.provider().generation() {
            return Err(HarnessError::InvalidProvider {
                reason: "replacement generation must be strictly greater".to_owned(),
            });
        }
        for slot in &mut registry.slots {
            if slot.registration.provider().id() == &id && slot.state == SlotState::Ready {
                slot.state = SlotState::Draining;
            }
        }
        registry
            .slots
            .push(Slot::new(registration, SlotState::Ready));
        reclaim(&mut registry.slots);
        Ok(())
    }

    /// Begin bounded drain of the active registration for an id: it stops
    /// admitting new activity (`Draining`) but any captured in-flight generation
    /// may still complete under its old fence. A zero-lease drained generation is
    /// reclaimed immediately so it is no longer selectable for new admission.
    pub fn begin_drain(&self, id: &ProviderId) -> Result<i64, HarnessError> {
        let mut registry = self.registry_write();
        let generation = {
            let slot = active_slot_mut(&mut registry.slots, id)
                .ok_or_else(|| HarnessError::ProviderNotFound { id: id.clone() })?;
            slot.state = SlotState::Draining;
            slot.registration.provider().generation()
        };
        reclaim(&mut registry.slots);
        Ok(generation)
    }

    /// Unregister a provider using the drain path. Every generation for the id
    /// stops admitting new activity immediately; zero-lease generations are
    /// removed at once, while a generation with in-flight leases is retained in
    /// `Draining` so its captured calls complete under the old fence and is
    /// reclaimed once its leases return to zero. Returns the captured active
    /// generation that was removed so callers can fence late results.
    pub fn unregister(&self, id: &ProviderId) -> Result<i64, HarnessError> {
        let mut registry = self.registry_write();
        let captured = active_slot(&registry.slots, id)
            .map(|slot| slot.registration.provider().generation())
            .ok_or_else(|| HarnessError::ProviderNotFound { id: id.clone() })?;
        for slot in &mut registry.slots {
            if slot.registration.provider().id() == id {
                slot.state = SlotState::Draining;
            }
        }
        // Unregister fully removes the provider: every zero-lease generation for
        // this id is disposed immediately (no selectable Draining generation is
        // kept). Slots with live in-flight leases are retained until their
        // captured calls complete, then reclaimed by `reclaim_drained`.
        registry
            .slots
            .retain(|slot| slot.registration.provider().id() != id || slot.live_leases() != 0);
        Ok(captured)
    }

    /// Reclaim `Draining` slots that can no longer be legitimately selected.
    ///
    /// A slot is retained iff it is `Ready` OR it has live in-flight leases (its
    /// captured calls must finish under the old fence). Every zero-lease
    /// `Draining` generation is disposed — no superseded generation is kept
    /// selectable. This bounds the live slot count per id to the number of
    /// `Ready` generations plus the number of draining generations that still
    /// hold at least one live lease, regardless of how many times a provider is
    /// replaced, drained, or unregistered.
    pub fn reclaim_drained(&self) {
        let mut registry = self.registry_write();
        reclaim(&mut registry.slots);
    }

    /// Total number of live slots across all ids and generations (Ready plus
    /// retained-because-leased Draining). Used to assert that repeated
    /// replace/unregister do not leak slots: it is bounded by the number of
    /// active generations, never by the number of lifecycle operations.
    pub fn total_slots(&self) -> usize {
        self.registry_read().slots.len()
    }

    /// Total live in-flight activity leases across every slot for an id (both
    /// Ready and retained Draining generations). Zero means no captured call is
    /// running.
    pub fn active_leases(&self, id: &ProviderId) -> usize {
        self.registry_read()
            .slots
            .iter()
            .filter(|slot| slot.registration.provider().id() == id)
            .map(Slot::live_leases)
            .sum()
    }

    /// Drive a bounded drain of a provider to quiescence: stop admission, then
    /// wait up to `deadline` for live leases to drain, reclaiming disposed slots
    /// as they do. Returns `true` if the provider reached zero live leases (all
    /// captured calls completed) within the deadline.
    ///
    /// On timeout this cancels every in-flight token still tracked for the id's
    /// slots (`DXB-DEL-068` H9) and returns `false`; it never admits new
    /// activity and never blocks unboundedly. The caller re-runs this (or the
    /// all-provider shutdown drain) to reap the cancelled calls. Slots that
    /// remain leased are kept `Draining` (fenced) and reclaimed on a later pass
    /// once their cancelled calls unwind.
    pub fn drain_to_quiescence(&self, id: &ProviderId, deadline: Duration) -> bool {
        {
            let mut registry = self.registry_write();
            for slot in &mut registry.slots {
                if slot.registration.provider().id() == id {
                    slot.state = SlotState::Draining;
                }
            }
            reclaim(&mut registry.slots);
        }
        let start = Instant::now();
        loop {
            self.reclaim_drained();
            if self.active_leases(id) == 0 {
                return true;
            }
            if start.elapsed() >= deadline {
                // Bounded-deadline exhausted: cancel every tracked token for the
                // id so parked calls are asked to unwind, then report not-yet
                // quiescent. Never wait unboundedly for a wedged call.
                self.cancel_tracked_tokens(Some(id));
                return false;
            }
            std::thread::sleep(DRAIN_POLL_INTERVAL.min(deadline));
        }
    }

    /// Drain every registered provider to quiescence within a single bounded
    /// deadline shared across all ids (`DXB-DEL-068` H8/H9 all-provider shutdown
    /// drain). Stops admission on all slots, waits for leases to reach zero,
    /// cancels every tracked in-flight token at the deadline, and reclaims
    /// disposed slots. Returns `true` iff every provider reached zero live
    /// leases within the deadline. Never blocks unboundedly.
    pub fn drain_all_to_quiescence(&self, deadline: Duration) -> bool {
        {
            let mut registry = self.registry_write();
            for slot in &mut registry.slots {
                slot.state = SlotState::Draining;
            }
            reclaim(&mut registry.slots);
        }
        let start = Instant::now();
        loop {
            self.reclaim_drained();
            if self.total_live_leases() == 0 {
                return true;
            }
            if start.elapsed() >= deadline {
                self.cancel_tracked_tokens(None);
                return false;
            }
            std::thread::sleep(DRAIN_POLL_INTERVAL.min(deadline));
        }
    }

    /// Cancel every in-flight cancellation token tracked across slots, optionally
    /// restricted to a single id. Used by the bounded drain timeout paths so a
    /// host-triggered shutdown/drain can force parked provider calls to observe
    /// cancellation and release their leases.
    fn cancel_tracked_tokens(&self, id: Option<&ProviderId>) {
        let registry = self.registry_read();
        for slot in &registry.slots {
            if id.is_none_or(|wanted| slot.registration.provider().id() == wanted) {
                slot.cancel_all_tokens();
            }
        }
    }

    fn total_live_leases(&self) -> usize {
        self.registry_read()
            .slots
            .iter()
            .map(Slot::live_leases)
            .sum()
    }

    /// Execute a bounded task against the exact selected provider id + generation.
    ///
    /// Requires the exact `provider_id` and `provider_generation` on the
    /// [`ExecuteRequest`]; there is no fallback to a different provider.
    pub fn execute(
        &self,
        request: &ExecuteRequest,
        cancellation: CancellationToken,
    ) -> Result<TaskResult, HarnessError> {
        self.execute_with_activity(request, cancellation)
            .map(|(result, _activity)| result)
    }

    /// Execute a bounded task and also return the [`ProviderActivityRef`] that
    /// was held for the call.
    ///
    /// Admission is atomic: while the registry write lock is held, the host
    /// observes an exact `(id, generation)` slot that is still `Ready`, acquires
    /// the in-flight lease (bumping the counter and tracking the call's
    /// cancellation token under a unique activity id), and clones the provider
    /// `Arc`. The lock is then released **before** the blocking
    /// `Handle::block_on`, so no host or slot lock is held across the provider
    /// call. Because the slot was `Ready` at admission and the lease was taken
    /// under the same lock, a concurrent replace/drain/unregister either ran
    /// before admission (this call is fenced with `GenerationFenced`) or after
    /// it (the slot becomes `Draining` but is retained until this lease drops).
    ///
    /// The Runtime scheduler uses the returned ref to fence a stale-generation
    /// result at commit time: if the ref's generation is no longer the active
    /// generation, the result must not be committed.
    pub fn execute_with_activity(
        &self,
        request: &ExecuteRequest,
        cancellation: CancellationToken,
    ) -> Result<(TaskResult, ProviderActivityRef), HarnessError> {
        let handle = self
            .handle_snapshot()
            .ok_or(HarnessError::NoProviderConfigured)?;
        // Atomic admission under the registry write lock: observe Ready + exact
        // generation, acquire the lease and track the token, clone the provider,
        // then drop the lock before executing.
        let (provider, activity) = {
            let registry = self.registry_write();
            let slot = admissible_slot(
                &registry.slots,
                &request.provider_id,
                request.provider_generation,
            )
            .ok_or_else(|| selection_error(&registry.slots, request))?;
            let activity_id = self.next_activity_id.fetch_add(1, Ordering::Relaxed);
            let activity = ProviderActivityRef::acquire(
                request.provider_id.clone(),
                request.provider_generation,
                activity_id,
                &slot.leases,
                &slot.tokens,
                &cancellation,
            );
            let provider = slot.registration.provider().clone();
            (provider, activity)
        };
        let result = handle.block_on(async { provider.execute(request, &cancellation).await });
        let task_result = self.to_task_result(&request.provider_id, result)?;
        Ok((task_result, activity))
    }

    /// Convenience execution over a [`TaskDescription`] against an exact
    /// provider id + generation. Used by the Runtime scheduler which owns the
    /// committed Context Plan binding.
    pub fn execute_task_with_cancel(
        &self,
        provider_id: &ProviderId,
        provider_generation: i64,
        task: &TaskDescription,
        cancellation: CancellationToken,
    ) -> Result<TaskResult, HarnessError> {
        let request = ExecuteRequest::bounded(
            provider_id.clone(),
            provider_generation,
            task.intent.clone(),
            task.context.clone(),
            task.budget,
            task.deadline,
        );
        self.execute(&request, cancellation)
    }

    /// List active (non-drained) registrations, optionally filtered by capability.
    pub fn list_providers(&self, capability: Option<&str>) -> Vec<ProviderInfo> {
        let registry = self.registry_read();
        let mut out = Vec::new();
        for id in active_ids(&registry.slots) {
            if let Some(slot) = active_slot(&registry.slots, &id) {
                let provider = slot.registration.provider();
                if capability.is_none_or(|requested| provider.capability() == requested) {
                    out.push(slot_info(slot));
                }
            }
        }
        out
    }

    /// Look up the active registration for an id.
    pub fn get_provider(&self, provider_id: &ProviderId) -> Result<ProviderInfo, HarnessError> {
        let registry = self.registry_read();
        active_slot(&registry.slots, provider_id)
            .map(slot_info)
            .ok_or_else(|| HarnessError::ProviderNotFound {
                id: provider_id.clone(),
            })
    }

    /// Whether `generation` is still the active (Ready, highest) generation for
    /// `id`. The Runtime scheduler uses this at commit time: a result produced
    /// under a generation that has since been superseded (replaced or
    /// unregistered) must be fenced and never committed as a fresh result.
    pub fn is_active_generation(&self, id: &ProviderId, generation: i64) -> bool {
        let registry = self.registry_read();
        active_slot(&registry.slots, id).is_some_and(|slot| {
            slot.state == SlotState::Ready
                && slot.registration.provider().generation() == generation
        })
    }

    fn registry_read(&self) -> std::sync::RwLockReadGuard<'_, Registry> {
        self.registry
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn registry_write(&self) -> std::sync::RwLockWriteGuard<'_, Registry> {
        self.registry
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn handle_snapshot(&self) -> Option<tokio::runtime::Handle> {
        self.handle
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn to_task_result(
        &self,
        provider_id: &ProviderId,
        result: Result<ExecuteResult, ExecuteError>,
    ) -> Result<TaskResult, HarnessError> {
        match result {
            Ok(execute_result) => {
                let tool_effects = execute_result.tool_effects.clone();
                match execute_result.outcome {
                    ExecuteOutcome::Succeeded { output, .. } => Ok(TaskResult {
                        output,
                        evidence: vec![Evidence {
                            provider: execute_result.provider_id.clone(),
                            observation: format!(
                                "provider:{}:generation:{}",
                                execute_result.provider_id.0, execute_result.provider_generation
                            ),
                        }],
                        status: TaskStatus::Completed,
                        tool_effects,
                    }),
                    ExecuteOutcome::Cancelled => Ok(TaskResult {
                        output: String::new(),
                        evidence: Vec::new(),
                        status: TaskStatus::Cancelled,
                        tool_effects,
                    }),
                }
            }
            Err(error) => Err(map_execute_error(provider_id, error)),
        }
    }
}

/// The active (Ready-or-Draining, highest-generation) slot for an id.
fn active_slot<'a>(slots: &'a [Slot], id: &ProviderId) -> Option<&'a Slot> {
    slots
        .iter()
        .filter(|slot| slot.registration.provider().id() == id)
        .max_by_key(|slot| slot.registration.provider().generation())
}

fn active_slot_mut<'a>(slots: &'a mut [Slot], id: &ProviderId) -> Option<&'a mut Slot> {
    slots
        .iter_mut()
        .filter(|slot| slot.registration.provider().id() == id)
        .max_by_key(|slot| slot.registration.provider().generation())
}

/// A slot admissible for a *new* execution: exact id + generation and still
/// `Ready`. A `Draining` slot is never admissible — a call beginning after
/// drain/replace/unregister is fenced even if it targets the old generation.
/// In-flight work that was admitted while the slot was `Ready` keeps its own
/// provider clone and activity lease and is unaffected.
fn admissible_slot<'a>(slots: &'a [Slot], id: &ProviderId, generation: i64) -> Option<&'a Slot> {
    slots.iter().find(|slot| {
        slot.state == SlotState::Ready
            && slot.registration.provider().id() == id
            && slot.registration.provider().generation() == generation
    })
}

fn selection_error(slots: &[Slot], request: &ExecuteRequest) -> HarnessError {
    match active_slot(slots, &request.provider_id) {
        Some(_) => HarnessError::GenerationFenced {
            id: request.provider_id.clone(),
        },
        None => HarnessError::ProviderNotFound {
            id: request.provider_id.clone(),
        },
    }
}

fn active_ids(slots: &[Slot]) -> Vec<ProviderId> {
    let mut ids: Vec<ProviderId> = Vec::new();
    for slot in slots {
        let id = slot.registration.provider().id().clone();
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    ids
}

/// Dispose every zero-lease `Draining` slot. A slot survives iff it is `Ready`
/// or still holds at least one live in-flight lease.
fn reclaim(slots: &mut Vec<Slot>) {
    slots.retain(|slot| slot.state != SlotState::Draining || slot.live_leases() != 0);
}

fn slot_info(slot: &Slot) -> ProviderInfo {
    let provider = slot.registration.provider();
    ProviderInfo {
        id: provider.id().clone(),
        capability: provider.capability().to_owned(),
        generation: provider.generation(),
        status: match slot.state {
            SlotState::Ready => ProviderStatus::Ready,
            SlotState::Draining => ProviderStatus::Draining,
        },
    }
}

fn map_execute_error(provider_id: &ProviderId, error: ExecuteError) -> HarnessError {
    match error {
        ExecuteError::TransportUnavailable => HarnessError::TransportUnavailable {
            id: provider_id.clone(),
        },
        ExecuteError::UpstreamUnavailable { status } => HarnessError::UpstreamUnavailable {
            id: provider_id.clone(),
            status,
        },
        ExecuteError::RateLimited {
            retry_after_seconds,
        } => HarnessError::RateLimited {
            id: provider_id.clone(),
            retry_after_seconds,
        },
        ExecuteError::InvalidRequest { status } => HarnessError::InvalidRequest {
            id: provider_id.clone(),
            status,
        },
        ExecuteError::ProtocolViolation => HarnessError::ProtocolViolation {
            id: provider_id.clone(),
        },
        ExecuteError::OutputExceeded => HarnessError::OutputExceeded {
            id: provider_id.clone(),
        },
        ExecuteError::DeadlineExceeded => HarnessError::DeadlineExceeded,
        ExecuteError::IncompleteStream => HarnessError::ExecutionFailed {
            id: provider_id.clone(),
            reason: "provider stream ended without completion".to_owned(),
        },
        ExecuteError::ExecutionFailed => HarnessError::ExecutionFailed {
            id: provider_id.clone(),
            reason: "provider execution failed".to_owned(),
        },
    }
}

fn validate_provider(
    id: &ProviderId,
    capability: &str,
    generation: i64,
) -> Result<(), HarnessError> {
    if id.0.trim().is_empty() {
        return Err(HarnessError::InvalidProvider {
            reason: "provider id must not be empty".to_owned(),
        });
    }
    if capability.trim().is_empty() {
        return Err(HarnessError::InvalidProvider {
            reason: "capability must not be empty".to_owned(),
        });
    }
    if generation < 0 {
        return Err(HarnessError::InvalidProvider {
            reason: "generation must not be negative".to_owned(),
        });
    }
    Ok(())
}

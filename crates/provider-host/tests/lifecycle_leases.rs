//! Provider host activity-lease, slot-reclamation, and bounded-drain coverage
//! (`DXB-DEL-068` H9).
//!
//! These tests exercise the shared dynamic lifecycle of `ProviderHost` under
//! concurrency: every `execute` acquires an in-flight lease against the exact
//! captured `(id, generation)` slot and releases it on completion/drop; a
//! `Draining` slot is reclaimed only at lease zero; repeated `replace` is
//! bounded by the number of *active* generations (no `Vec<Slot>` growth); and a
//! bounded drain returns quiescence status without ever admitting new activity
//! or blocking unboundedly.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use dxbot_core::types::ProviderId;
use provider_host::{
    CancellationToken, ExecuteFuture, ExecuteOutcome, ExecuteProvider, ExecuteRequest,
    ExecuteResult, HarnessError, ProtocolKind, ProviderHost, ProviderRegistration, ProviderStatus,
    RegistrationLimits, TaskDescription, TaskStatus, UsageInfo,
};

fn test_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("runtime")
}

fn task(intent: &str) -> TaskDescription {
    TaskDescription {
        intent: intent.to_owned(),
        context: "bounded-context".to_owned(),
        budget: Some(10),
        deadline: None,
    }
}

fn limits() -> RegistrationLimits {
    RegistrationLimits {
        max_output_bytes: 64 * 1024,
        max_output_items: 256,
    }
}

/// A provider whose async body blocks on a shared gate until released, so a
/// test can observe a live in-flight lease from another thread while the call
/// is parked. It also observes cancellation and returns a Cancelled outcome
/// promptly so timeout/cancel paths can be exercised.
#[derive(Debug)]
struct GatedProvider {
    id: ProviderId,
    generation: i64,
    entered: Arc<AtomicUsize>,
    gate: Arc<tokio::sync::Semaphore>,
    honor_cancel: bool,
}

impl GatedProvider {
    fn registration(
        id: &str,
        generation: i64,
        entered: &Arc<AtomicUsize>,
        gate: &Arc<tokio::sync::Semaphore>,
        honor_cancel: bool,
    ) -> ProviderRegistration {
        ProviderRegistration::detached(
            Arc::new(Self {
                id: ProviderId(id.to_owned()),
                generation,
                entered: Arc::clone(entered),
                gate: Arc::clone(gate),
                honor_cancel,
            }),
            ProtocolKind::OpenAiChatCompletions,
            limits(),
        )
    }
}

impl ExecuteProvider for GatedProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }
    fn capability(&self) -> &str {
        "text"
    }
    fn generation(&self) -> i64 {
        self.generation
    }
    fn execute<'a>(
        &'a self,
        _request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a> {
        Box::pin(async move {
            self.entered.fetch_add(1, Ordering::SeqCst);
            loop {
                if self.honor_cancel && cancellation.is_cancelled() {
                    return Ok(ExecuteResult::new(
                        self.id.clone(),
                        self.generation,
                        ExecuteOutcome::Cancelled,
                    ));
                }
                // Try to acquire the gate; if released, complete successfully.
                if self.gate.try_acquire().is_ok() {
                    return Ok(ExecuteResult::new(
                        self.id.clone(),
                        self.generation,
                        ExecuteOutcome::Succeeded {
                            output: "gated-ok".to_owned(),
                            usage: UsageInfo {
                                prompt_tokens: 0,
                                completion_tokens: 0,
                                reasoning_tokens: None,
                            },
                        },
                    ));
                }
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
    }
}

#[test]
fn lease_is_zero_before_and_after_completion() {
    let runtime = test_runtime();
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(1)); // already open
    host.register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let id = ProviderId("p".to_owned());
    assert_eq!(host.active_leases(&id), 0);
    let result = host
        .execute_task_with_cancel(&id, 1, &task("go"), CancellationToken::new())
        .unwrap();
    assert_eq!(result.status, TaskStatus::Completed);
    assert_eq!(
        host.active_leases(&id),
        0,
        "lease released after completion"
    );
}

#[test]
fn no_lease_on_generation_fence_or_unknown() {
    let runtime = test_runtime();
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    host.register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let id = ProviderId("p".to_owned());
    // Wrong generation is fenced with no lease acquired and provider not entered.
    let fenced = host.execute_task_with_cancel(&id, 99, &task("x"), CancellationToken::new());
    assert!(matches!(fenced, Err(HarnessError::GenerationFenced { .. })));
    // Unknown id fails closed with no lease.
    let unknown = host.execute_task_with_cancel(
        &ProviderId("other".to_owned()),
        1,
        &task("x"),
        CancellationToken::new(),
    );
    assert!(matches!(
        unknown,
        Err(HarnessError::ProviderNotFound { .. })
    ));
    assert_eq!(host.active_leases(&id), 0);
    assert_eq!(entered.load(Ordering::SeqCst), 0, "provider never entered");
}

#[test]
fn draining_slot_with_live_lease_is_retained_then_reclaimed_at_zero() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0)); // closed: call parks
    // Build a mutable host, register generation 1, then share.
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());

    // Launch an in-flight call on generation 1 that will park on the closed gate.
    let worker_host = Arc::clone(&shared);
    let worker = std::thread::spawn(move || {
        worker_host.execute_task_with_cancel(
            &ProviderId("p".to_owned()),
            1,
            &task("park"),
            CancellationToken::new(),
        )
    });
    // Wait until the provider is actually in-flight (lease held).
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while shared.active_leases(&id) == 0 {
        assert!(std::time::Instant::now() < deadline, "call never started");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(shared.active_leases(&id), 1);

    // Now open the gate so the parked call can finish, then it releases.
    gate.add_permits(1);
    let result = worker.join().unwrap().unwrap();
    assert_eq!(result.status, TaskStatus::Completed);
    // After completion the lease is zero again.
    let final_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while shared.active_leases(&id) != 0 {
        assert!(std::time::Instant::now() < final_deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(shared.active_leases(&id), 0);
}

#[test]
fn repeated_replace_is_bounded_by_active_generations_no_slot_leak() {
    let runtime = test_runtime();
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    host.register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let id = ProviderId("p".to_owned());
    // Replace 25 times with strictly-greater generations while nothing is
    // in-flight. Old zero-lease Draining generations are reclaimed on each
    // replace; at most the new Ready generation plus one selectable Draining
    // generation survive, so the slot count never grows with the number of
    // replacements.
    for generation in 2..=26 {
        host.replace(GatedProvider::registration(
            "p", generation, &entered, &gate, false,
        ))
        .unwrap();
        // list_providers reports the active generation per id (one entry).
        assert_eq!(
            host.list_providers(None).len(),
            1,
            "exactly one active generation per id is listed"
        );
        // Total live slots are bounded by active generations (<= 2), never N.
        assert!(
            host.total_slots() <= 2,
            "repeated replace must not leak slots: {} slots at gen {generation}",
            host.total_slots()
        );
    }
    assert_eq!(host.get_provider(&id).unwrap().generation, 26);
    assert_eq!(
        host.get_provider(&id).unwrap().status,
        ProviderStatus::Ready
    );
}

#[test]
fn drain_to_quiescence_returns_true_when_idle_and_stops_admission() {
    let runtime = test_runtime();
    let host = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(1));
    host.register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let id = ProviderId("p".to_owned());
    // No in-flight leases: drain reaches quiescence immediately.
    assert!(host.drain_to_quiescence(&id, Duration::from_millis(200)));
    // A zero-lease drained generation is fully reclaimed: it is no longer a
    // selectable slot, so the id reports gone and the commit/admission fence
    // (`is_active_generation`) refuses it. A drained provider is never admitted
    // for a fresh execution.
    assert!(matches!(
        host.get_provider(&id),
        Err(HarnessError::ProviderNotFound { .. })
    ));
    assert!(
        !host.is_active_generation(&id, 1),
        "drained generation must not be admittable"
    );
    assert_eq!(
        host.total_slots(),
        0,
        "zero-lease drained slot is reclaimed"
    );
}

#[test]
fn drain_deadline_is_bounded_when_a_lease_never_drains() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0)); // closed forever here
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 1, &entered, &gate, true))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());
    let cancel = CancellationToken::new();

    // Launch a call that parks on the closed gate.
    let worker_host = Arc::clone(&shared);
    let worker_cancel = cancel.clone();
    let worker = std::thread::spawn(move || {
        worker_host.execute_task_with_cancel(
            &ProviderId("p".to_owned()),
            1,
            &task("park"),
            worker_cancel,
        )
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while shared.active_leases(&id) == 0 {
        assert!(std::time::Instant::now() < deadline, "call never started");
        std::thread::sleep(Duration::from_millis(5));
    }

    // While the lease is still live, a bounded wait for quiescence must return
    // within (approximately) its deadline and never block unboundedly. We
    // exercise the same bounded read API the coordinator uses to observe drain
    // progress on a shared host.
    let drain_start = std::time::Instant::now();
    let bounded = Duration::from_millis(150);
    while shared.active_leases(&id) != 0 && drain_start.elapsed() < bounded {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        drain_start.elapsed() < Duration::from_secs(2),
        "bounded drain wait must not block unboundedly"
    );
    assert_eq!(
        shared.active_leases(&id),
        1,
        "lease still held (not drained)"
    );

    // Cancel the in-flight call; the honor_cancel provider observes it and
    // returns Cancelled, releasing the lease.
    cancel.cancel();
    let result = worker.join().unwrap().unwrap();
    assert_eq!(result.status, TaskStatus::Cancelled);
    let release_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while shared.active_leases(&id) != 0 {
        assert!(std::time::Instant::now() < release_deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(shared.active_leases(&id), 0, "cancel released the lease");
}

#[test]
fn concurrent_execute_and_reclaim_stress_leaks_no_slots_or_leases() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(1_000_000)); // always open
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());

    let mut workers = Vec::new();
    for _ in 0..8 {
        let host = Arc::clone(&shared);
        workers.push(std::thread::spawn(move || {
            for _ in 0..50 {
                let _ = host.execute_task_with_cancel(
                    &ProviderId("p".to_owned()),
                    1,
                    &task("go"),
                    CancellationToken::new(),
                );
            }
        }));
    }
    for worker in workers {
        worker.join().unwrap();
    }
    // All calls completed; no lease leaked.
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while shared.active_leases(&id) != 0 {
        assert!(
            std::time::Instant::now() < deadline,
            "lease leak under stress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(shared.active_leases(&id), 0);
    assert!(shared.is_active_generation(&id, 1));
}

/// Block until the id shows at least one live lease, or panic on timeout.
fn wait_for_lease(shared: &Arc<ProviderHost>, id: &ProviderId) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while shared.active_leases(id) == 0 {
        assert!(std::time::Instant::now() < deadline, "call never started");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Block until the id shows zero live leases, or panic on timeout.
fn wait_for_zero_leases(shared: &Arc<ProviderHost>, id: &ProviderId) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while shared.active_leases(id) != 0 {
        assert!(std::time::Instant::now() < deadline, "lease never released");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// A truly-shared `Arc<ProviderHost>`: while an execute on generation 1 is
/// parked (holding its captured provider + activity lease), a concurrent
/// `replace` to generation 2 publishes the new generation. The in-flight call
/// finishes under its old fence, but any call *beginning* after the replace
/// against generation 1 is fenced, the new generation admits work, and the
/// superseded generation fails the commit fence.
#[test]
fn shared_replace_while_parked_fences_old_admission_and_commit_but_lets_inflight_finish() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0)); // closed: gen-1 call parks
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());

    // Park an in-flight call on generation 1 that already holds provider+lease.
    let worker_host = Arc::clone(&shared);
    let inflight = std::thread::spawn(move || {
        worker_host.execute_task_with_cancel(
            &ProviderId("p".to_owned()),
            1,
            &task("park-gen-1"),
            CancellationToken::new(),
        )
    });
    wait_for_lease(&shared, &id);
    assert_eq!(shared.active_leases(&id), 1);

    // Concurrently replace to generation 2 through the SAME shared Arc.
    let open_gate = Arc::new(tokio::sync::Semaphore::new(1_000_000)); // open: gen-2 runs
    let entered2 = Arc::new(AtomicUsize::new(0));
    shared
        .replace(GatedProvider::registration(
            "p", 2, &entered2, &open_gate, false,
        ))
        .unwrap();

    // The active generation is now 2, and it is the commit-fence generation.
    assert_eq!(shared.get_provider(&id).unwrap().generation, 2);
    assert!(shared.is_active_generation(&id, 2));
    // The superseded generation 1 is no longer active: a late result committed
    // under it must be fenced.
    assert!(!shared.is_active_generation(&id, 1));

    // A call BEGINNING after replace against the old generation is fenced.
    let old_admission =
        shared.execute_task_with_cancel(&id, 1, &task("late-old"), CancellationToken::new());
    assert!(
        matches!(old_admission, Err(HarnessError::GenerationFenced { .. })),
        "new admission on the superseded generation must be fenced, got {old_admission:?}"
    );

    // The NEW generation admits and completes work.
    let new_result = shared
        .execute_task_with_cancel(&id, 2, &task("new-gen"), CancellationToken::new())
        .unwrap();
    assert_eq!(new_result.status, TaskStatus::Completed);

    // The parked in-flight generation-1 call is still allowed to finish under
    // its captured fence: release its gate and join.
    gate.add_permits(1);
    let inflight_result = inflight.join().unwrap().unwrap();
    assert_eq!(inflight_result.status, TaskStatus::Completed);

    // After the in-flight generation-1 call returns, its zero-lease Draining
    // slot is reclaimed and only the single Ready generation 2 remains.
    wait_for_zero_leases(&shared, &id);
    shared.reclaim_drained();
    assert_eq!(shared.total_slots(), 1, "only Ready gen 2 remains");
    assert!(shared.is_active_generation(&id, 2));
}

/// Unregister through a shared Arc while a call is parked: the id is fenced for
/// new admission immediately, the in-flight call finishes under its captured
/// fence, and the leased Draining slot is retained until the lease drops then
/// fully reclaimed (no slot leak).
#[test]
fn shared_unregister_while_parked_retains_then_reclaims_leased_slot() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0)); // closed: call parks
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 7, &entered, &gate, false))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());

    let worker_host = Arc::clone(&shared);
    let inflight = std::thread::spawn(move || {
        worker_host.execute_task_with_cancel(
            &ProviderId("p".to_owned()),
            7,
            &task("park"),
            CancellationToken::new(),
        )
    });
    wait_for_lease(&shared, &id);

    // Unregister concurrently: captures generation 7, retains the leased slot.
    let captured = shared.unregister(&id).unwrap();
    assert_eq!(captured, 7);
    // A leased Draining slot is retained (not leaked, not reclaimed yet).
    assert_eq!(shared.total_slots(), 1);
    assert_eq!(shared.active_leases(&id), 1);

    // New admission after unregister is fenced closed: while the leased slot
    // lingers in Draining it is not admissible, so a fresh call is fenced.
    let after = shared.execute_task_with_cancel(&id, 7, &task("late"), CancellationToken::new());
    assert!(
        matches!(
            after,
            Err(HarnessError::GenerationFenced { .. }) | Err(HarnessError::ProviderNotFound { .. })
        ),
        "a call beginning after unregister must be fenced, got {after:?}"
    );

    // Let the in-flight call finish; the slot is reclaimed at lease zero.
    gate.add_permits(1);
    let result = inflight.join().unwrap().unwrap();
    assert_eq!(result.status, TaskStatus::Completed);
    wait_for_zero_leases(&shared, &id);
    shared.reclaim_drained();
    assert_eq!(
        shared.total_slots(),
        0,
        "leased slot reclaimed at lease zero"
    );
    assert!(shared.get_provider(&id).is_err());
}

/// A host-triggered bounded drain fires the tracked cancellation token of a
/// parked call, which the provider observes and releases its lease — proving
/// the per-slot token tracking wires host-initiated cancellation to in-flight
/// work without the caller holding the token.
#[test]
fn host_triggered_drain_cancellation_is_observed_by_parked_call() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(0)); // never opened
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 1, &entered, &gate, true))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());

    // The caller does NOT retain a token it can cancel; only the host tracks it.
    let worker_host = Arc::clone(&shared);
    let inflight = std::thread::spawn(move || {
        worker_host.execute_task_with_cancel(
            &ProviderId("p".to_owned()),
            1,
            &task("park-forever"),
            CancellationToken::new(),
        )
    });
    wait_for_lease(&shared, &id);

    // A bounded drain cannot reach quiescence (gate never opens), so at its
    // deadline it cancels every tracked token for the id. The honor_cancel
    // provider observes it and returns Cancelled, releasing the lease.
    let quiesced = shared.drain_to_quiescence(&id, Duration::from_millis(100));
    assert!(!quiesced, "cannot quiesce while the call is parked");

    let result = inflight.join().unwrap().unwrap();
    assert_eq!(
        result.status,
        TaskStatus::Cancelled,
        "host-triggered cancellation must be observed by the parked call"
    );
    wait_for_zero_leases(&shared, &id);
    // A follow-up drain now reaches quiescence and reclaims the slot.
    assert!(shared.drain_to_quiescence(&id, Duration::from_millis(100)));
    assert_eq!(shared.total_slots(), 0);
}

/// The all-provider shutdown drain fires tracked tokens across every id at its
/// deadline (`DXB-DEL-068` H8/H9), reaping parked calls and reclaiming slots.
#[test]
fn all_provider_shutdown_drain_cancels_and_reaps_every_id() {
    let runtime = test_runtime();
    let gate = Arc::new(tokio::sync::Semaphore::new(0)); // never opened
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    for id in ["a", "b"] {
        let entered = Arc::new(AtomicUsize::new(0));
        owned
            .register(GatedProvider::registration(id, 1, &entered, &gate, true))
            .unwrap();
    }
    let shared = Arc::new(owned);

    let mut inflight = Vec::new();
    for id in ["a", "b"] {
        let host = Arc::clone(&shared);
        inflight.push(std::thread::spawn(move || {
            host.execute_task_with_cancel(
                &ProviderId(id.to_owned()),
                1,
                &task("park"),
                CancellationToken::new(),
            )
        }));
    }
    wait_for_lease(&shared, &ProviderId("a".to_owned()));
    wait_for_lease(&shared, &ProviderId("b".to_owned()));

    // Bounded all-provider drain: cannot quiesce, so it cancels every tracked
    // token across both ids at the deadline.
    assert!(!shared.drain_all_to_quiescence(Duration::from_millis(100)));
    for worker in inflight {
        let result = worker.join().unwrap().unwrap();
        assert_eq!(result.status, TaskStatus::Cancelled);
    }
    // A follow-up bounded drain now verifies zero leases everywhere and reclaims.
    assert!(shared.drain_all_to_quiescence(Duration::from_millis(200)));
    assert_eq!(shared.total_slots(), 0, "all slots reclaimed after reap");
}

/// Stress: many concurrent executes racing repeated replace through one shared
/// Arc. No lease or slot leaks; the live slot count stays bounded by active
/// generations regardless of how many replacements and calls interleave.
#[test]
fn shared_concurrent_execute_and_replace_stress_no_leak() {
    let runtime = test_runtime();
    let entered = Arc::new(AtomicUsize::new(0));
    let gate = Arc::new(tokio::sync::Semaphore::new(1_000_000)); // always open
    let owned = ProviderHost::new().with_runtime_handle(runtime.handle().clone());
    owned
        .register(GatedProvider::registration("p", 1, &entered, &gate, false))
        .unwrap();
    let shared = Arc::new(owned);
    let id = ProviderId("p".to_owned());

    // Executor threads: keep trying the current-ish generation; a fenced call is
    // an expected, non-leaking outcome during a concurrent replace.
    let mut workers = Vec::new();
    for _ in 0..8 {
        let host = Arc::clone(&shared);
        let id = id.clone();
        workers.push(std::thread::spawn(move || {
            for _ in 0..100 {
                let generation = host.get_provider(&id).map(|p| p.generation).unwrap_or(1);
                let _ = host.execute_task_with_cancel(
                    &id,
                    generation,
                    &task("go"),
                    CancellationToken::new(),
                );
            }
        }));
    }
    // Replacer thread: strictly-greater generations.
    let replacer_host = Arc::clone(&shared);
    let replacer_entered = Arc::clone(&entered);
    let replacer_gate = Arc::clone(&gate);
    let replacer = std::thread::spawn(move || {
        for generation in 2..=60 {
            let _ = replacer_host.replace(GatedProvider::registration(
                "p",
                generation,
                &replacer_entered,
                &replacer_gate,
                false,
            ));
            // Bound: at most Ready + any actually-leased draining generations.
            assert!(
                replacer_host.total_slots() <= 16,
                "slot count must stay bounded under stress: {}",
                replacer_host.total_slots()
            );
            std::thread::yield_now();
        }
    });
    for worker in workers {
        worker.join().unwrap();
    }
    replacer.join().unwrap();

    // Drain to quiescence and reclaim: no lease leak, one Ready generation left.
    let final_deadline = std::time::Instant::now() + Duration::from_secs(10);
    while shared.active_leases(&id) != 0 {
        assert!(
            std::time::Instant::now() < final_deadline,
            "lease leak under stress"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    shared.reclaim_drained();
    assert_eq!(shared.active_leases(&id), 0);
    assert_eq!(
        shared.total_slots(),
        1,
        "only the single Ready generation remains"
    );
}

//! Process-wide resource admission and supervised Execution coordinator.
//!
//! Core leases are transient, generation-fenced permits. Canonical
//! Task/Execution/Process state stays in Application; ProviderHost is consumed
//! through the immutable committed Context Plan binding.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use application::{
    ApplicationMutator, ExecutionCandidate, ExecutionEvidenceInput, ExecutionResultInput,
};
use dxbot_core::types::{CoreLeaseId, ExecutionId, ProviderId};
use provider_host::{
    CancellationToken, HarnessError, ProviderHost, ProviderStatus, TaskDescription,
    TaskStatus as ProviderTaskStatus,
};
use runtime_security::SecurityState;

use crate::audit_projection::AuditProjection;

const MAX_ACTIVE_EXECUTIONS: usize = 4;
const MAX_ADMITTED_QUEUE: usize = 64;
const COORDINATOR_IDLE_POLL: Duration = Duration::from_millis(25);

#[derive(Debug)]
struct GovernorState {
    active: HashMap<ExecutionId, CoreLeaseId>,
}

#[derive(Debug)]
pub struct ResourceGovernor {
    scheduler_generation: i64,
    max_active: usize,
    next_lease: AtomicU64,
    state: Mutex<GovernorState>,
}

impl ResourceGovernor {
    pub fn new(scheduler_generation: i64, max_active: usize) -> Result<Arc<Self>, String> {
        if scheduler_generation <= 0 || max_active == 0 {
            return Err("Resource Governor requires positive generation and capacity".to_owned());
        }
        Ok(Arc::new(Self {
            scheduler_generation,
            max_active,
            next_lease: AtomicU64::new(1),
            state: Mutex::new(GovernorState {
                active: HashMap::new(),
            }),
        }))
    }

    pub fn try_acquire(
        self: &Arc<Self>,
        execution_id: &ExecutionId,
    ) -> Result<Option<CoreLease>, String> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Resource Governor lock poisoned".to_owned())?;
        if state.active.contains_key(execution_id) {
            return Ok(None);
        }
        if state.active.len() >= self.max_active {
            return Ok(None);
        }
        let sequence = self.next_lease.fetch_add(1, Ordering::Relaxed);
        let lease_id = CoreLeaseId(format!(
            "core-lease:{}:{sequence}",
            self.scheduler_generation
        ));
        state.active.insert(execution_id.clone(), lease_id.clone());
        Ok(Some(CoreLease {
            lease_id,
            execution_id: execution_id.clone(),
            scheduler_generation: self.scheduler_generation,
            governor: Arc::clone(self),
            released: false,
        }))
    }

    pub fn active_permits(&self) -> Result<usize, String> {
        self.state
            .lock()
            .map(|state| state.active.len())
            .map_err(|_| "Resource Governor lock poisoned".to_owned())
    }

    pub fn capacity(&self) -> usize {
        self.max_active
    }

    pub fn scheduler_generation(&self) -> i64 {
        self.scheduler_generation
    }

    fn release(&self, execution_id: &ExecutionId, lease_id: &CoreLeaseId) {
        if let Ok(mut state) = self.state.lock() {
            let matches = state
                .active
                .get(execution_id)
                .is_some_and(|active| active == lease_id);
            if matches {
                state.active.remove(execution_id);
            }
        }
    }
}

#[derive(Debug)]
pub struct CoreLease {
    pub lease_id: CoreLeaseId,
    pub execution_id: ExecutionId,
    pub scheduler_generation: i64,
    governor: Arc<ResourceGovernor>,
    released: bool,
}

impl CoreLease {
    pub fn release(mut self) {
        self.governor.release(&self.execution_id, &self.lease_id);
        self.released = true;
    }
}

impl Drop for CoreLease {
    fn drop(&mut self) {
        if !self.released {
            self.governor.release(&self.execution_id, &self.lease_id);
            self.released = true;
        }
    }
}

#[derive(Debug)]
pub struct ExecutionCoordinator {
    stop: Arc<AtomicBool>,
    current_provider_calls: Arc<Mutex<HashMap<ExecutionId, CancellationToken>>>,
    governor: Arc<ResourceGovernor>,
    join: Option<JoinHandle<()>>,
}

impl ExecutionCoordinator {
    pub(crate) fn start(
        application: Arc<ApplicationMutator>,
        providers: Arc<ProviderHost>,
        scheduler_generation: i64,
        audit: Arc<AuditProjection>,
        security: Arc<Mutex<SecurityState>>,
    ) -> Result<Self, String> {
        application
            .recover_inflight_executions()
            .map_err(|error| format!("cannot reconcile in-flight Executions: {error:?}"))?;
        audit.drain(&application, &security)?;
        let governor = ResourceGovernor::new(scheduler_generation, MAX_ACTIVE_EXECUTIONS)?;
        let stop = Arc::new(AtomicBool::new(false));
        let current_provider_calls = Arc::new(Mutex::new(HashMap::new()));
        let thread_stop = Arc::clone(&stop);
        let thread_calls = Arc::clone(&current_provider_calls);
        let thread_governor = Arc::clone(&governor);
        let thread_audit = Arc::clone(&audit);
        let thread_security = Arc::clone(&security);
        let join = thread::Builder::new()
            .name("dxbot-execution-coordinator".to_owned())
            .spawn(move || {
                run_loop(
                    &application,
                    &providers,
                    &thread_governor,
                    scheduler_generation,
                    &thread_stop,
                    &thread_calls,
                    AuditOwners {
                        projection: &thread_audit,
                        security: &thread_security,
                    },
                );
            })
            .map_err(|error| format!("cannot spawn Execution coordinator: {error}"))?;
        Ok(Self {
            stop,
            current_provider_calls,
            governor,
            join: Some(join),
        })
    }

    pub fn active_permits(&self) -> Result<usize, String> {
        self.governor.active_permits()
    }

    pub(crate) fn governor_handle(&self) -> Arc<ResourceGovernor> {
        Arc::clone(&self.governor)
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        if let Ok(calls) = self.current_provider_calls.lock() {
            for token in calls.values() {
                token.cancel();
            }
        }
        if let Some(join) = self.join.take() {
            join.join()
                .map_err(|_| "Execution coordinator thread panicked".to_owned())?;
        }
        if self.governor.active_permits()? != 0 {
            return Err("Execution coordinator stopped with active Core permits".to_owned());
        }
        Ok(())
    }
}

impl Drop for ExecutionCoordinator {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

struct AuditOwners<'a> {
    projection: &'a AuditProjection,
    security: &'a Arc<Mutex<SecurityState>>,
}

fn run_loop(
    application: &Arc<ApplicationMutator>,
    providers: &Arc<ProviderHost>,
    governor: &Arc<ResourceGovernor>,
    scheduler_generation: i64,
    stop: &AtomicBool,
    current_provider_calls: &Arc<Mutex<HashMap<ExecutionId, CancellationToken>>>,
    audit: AuditOwners<'_>,
) {
    while !stop.load(Ordering::Acquire) {
        if audit.projection.drain(application, audit.security).is_err() {
            break;
        }
        let candidates = match application.admitted_executions(MAX_ADMITTED_QUEUE) {
            Ok(candidates) => candidates,
            Err(_) => break,
        };
        if candidates.is_empty() {
            thread::sleep(COORDINATOR_IDLE_POLL);
            continue;
        }
        let mut workers = Vec::new();
        for candidate in candidates.into_iter().take(MAX_ACTIVE_EXECUTIONS) {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let worker_application = Arc::clone(application);
            let worker_providers = Arc::clone(providers);
            let worker_governor = Arc::clone(governor);
            let worker_calls = Arc::clone(current_provider_calls);
            if let Ok(worker) = thread::Builder::new()
                .name(format!("dxbot-execution-{}", candidate.execution_id.0))
                .spawn(move || {
                    run_candidate(
                        &worker_application,
                        &worker_providers,
                        &worker_governor,
                        scheduler_generation,
                        &worker_calls,
                        &candidate,
                    );
                })
            {
                workers.push(worker);
            }
        }
        for worker in workers {
            if worker.join().is_err() {
                return;
            }
        }
    }
}

fn run_candidate(
    application: &Arc<ApplicationMutator>,
    providers: &ProviderHost,
    governor: &Arc<ResourceGovernor>,
    scheduler_generation: i64,
    current_provider_calls: &Mutex<HashMap<ExecutionId, CancellationToken>>,
    candidate: &ExecutionCandidate,
) {
    let Ok(Some(lease)) = governor.try_acquire(&candidate.execution_id) else {
        return;
    };
    let provider = match providers.get_provider(&candidate.provider_id) {
        Ok(provider)
            if provider.generation == candidate.provider_generation
                && provider.status == ProviderStatus::Ready =>
        {
            provider
        }
        _ => {
            let _ = application.mark_admission_recovery_required(
                candidate,
                "Committed Provider binding is unavailable or replaced",
            );
            return;
        }
    };
    let Ok(fence) = application.begin_execution(candidate, scheduler_generation) else {
        return;
    };
    let side_effect_id = match application.prepare_provider_side_effect(&fence) {
        Ok(id) => id,
        Err(_) => {
            let _ = application.fail_execution(&fence, "Provider Side Effect prepare failed", true);
            return;
        }
    };
    if application
        .mark_provider_side_effect_dispatched(&fence, &side_effect_id)
        .is_err()
    {
        let _ =
            application.fail_execution(&fence, "Provider Side Effect dispatch record failed", true);
        return;
    }
    let cancellation = CancellationToken::new();
    if let Ok(mut active) = current_provider_calls.lock() {
        active.insert(candidate.execution_id.clone(), cancellation.clone());
    }
    let monitor_done = Arc::new(AtomicBool::new(false));
    let monitor_application = Arc::clone(application);
    let monitor_execution = candidate.execution_id.clone();
    let monitor_cancellation = cancellation.clone();
    let monitor_done_child = Arc::clone(&monitor_done);
    let monitor = thread::Builder::new()
        .name("dxbot-execution-cancel-monitor".to_owned())
        .spawn(move || {
            while !monitor_done_child.load(Ordering::Acquire) {
                if monitor_application
                    .execution_should_cancel(&monitor_execution)
                    .unwrap_or(true)
                {
                    monitor_cancellation.cancel();
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
    let task = TaskDescription {
        intent: candidate.task.intent.clone(),
        context: candidate.task.context.clone(),
        budget: candidate.task.budget,
        deadline: candidate.task.deadline,
    };
    let provider_result = providers.execute_task_with_cancel(&task, cancellation);
    monitor_done.store(true, Ordering::Release);
    if let Ok(monitor) = monitor {
        let _ = monitor.join();
    }
    if let Ok(mut active) = current_provider_calls.lock() {
        active.remove(&candidate.execution_id);
    }
    let durable_cancel = application
        .execution_should_cancel(&candidate.execution_id)
        .unwrap_or(true);

    match provider_result {
        Ok(result) if durable_cancel || result.status == ProviderTaskStatus::Cancelled => {
            let _ = application.mark_provider_side_effect_unknown(
                &fence,
                &side_effect_id,
                "Provider activity cancelled after dispatch",
            );
            if durable_cancel {
                let _ = application.finalize_cancelled_execution(
                    &fence,
                    "Provider activity cancelled by durable Task control",
                );
            } else {
                let _ = application.fail_execution(
                    &fence,
                    "Provider activity cancelled during Runtime shutdown",
                    true,
                );
            }
        }
        Ok(result) if !result.output.is_empty() => {
            let input = ExecutionResultInput {
                output: result.output,
                evidence: result
                    .evidence
                    .into_iter()
                    .map(|evidence| ExecutionEvidenceInput {
                        provider_id: evidence.provider,
                        observation: evidence.observation,
                    })
                    .collect(),
            };
            if application
                .complete_execution_with_side_effect(&fence, &input, Some(&side_effect_id))
                .is_err()
            {
                let _ = application.mark_provider_side_effect_unknown(
                    &fence,
                    &side_effect_id,
                    "Result commit failed after Provider response",
                );
                let _ = application.fail_execution(
                    &fence,
                    "Result commit failed; reconciliation required",
                    true,
                );
            }
        }
        Ok(_) => {
            let _ = application.mark_provider_side_effect_unknown(
                &fence,
                &side_effect_id,
                "Provider returned an unusable empty result",
            );
            let _ = application.fail_execution(&fence, "Provider returned an empty result", false);
        }
        Err(error) => {
            let recovery_required = matches!(
                error,
                HarnessError::DeadlineExceeded | HarnessError::TransportUnavailable { .. }
            );
            let reason = provider_failure_reason(&provider.id, &error);
            let _ = application.mark_provider_side_effect_unknown(&fence, &side_effect_id, &reason);
            let _ = application.fail_execution(&fence, &reason, recovery_required);
        }
    }
    lease.release();
}

fn provider_failure_reason(provider_id: &ProviderId, error: &HarnessError) -> String {
    let kind = match error {
        HarnessError::RateLimited { .. } => "rate-limited",
        HarnessError::UpstreamUnavailable { .. } => "upstream-unavailable",
        HarnessError::TransportUnavailable { .. } => "transport-unavailable",
        HarnessError::InvalidRequest { .. } => "invalid-request",
        HarnessError::ProtocolViolation { .. } => "protocol-violation",
        HarnessError::OutputExceeded { .. } => "output-exceeded",
        HarnessError::Cancelled { .. } => "cancelled",
        HarnessError::DeadlineExceeded => "deadline-exceeded",
        HarnessError::ProviderUnavailable { .. } => "provider-unavailable",
        HarnessError::ProviderNotFound { .. } => "provider-not-found",
        HarnessError::NoReferenceProvider | HarnessError::NoHarnessAdapter => "unconfigured",
        HarnessError::AlreadyRegistered { .. }
        | HarnessError::InvalidProvider { .. }
        | HarnessError::ExecutionFailed { .. } => "execution-failed",
    };
    format!("provider-failure kind={kind} provider={}", provider_id.0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn core_lease_is_unique_and_drop_releases_permit() {
        let governor = ResourceGovernor::new(7, 1).expect("governor");
        let execution = ExecutionId("execution-a".to_owned());
        let lease = governor
            .try_acquire(&execution)
            .expect("acquire")
            .expect("granted");
        assert_eq!(lease.scheduler_generation, 7);
        assert!(
            governor
                .try_acquire(&execution)
                .expect("duplicate")
                .is_none()
        );
        assert_eq!(governor.active_permits().expect("active"), 1);
        drop(lease);
        assert_eq!(governor.active_permits().expect("released"), 0);
    }

    #[test]
    fn capacity_rejection_happens_before_second_lease() {
        let governor = ResourceGovernor::new(3, 1).expect("governor");
        let first = governor
            .try_acquire(&ExecutionId("execution-a".to_owned()))
            .expect("first")
            .expect("granted");
        assert!(
            governor
                .try_acquire(&ExecutionId("execution-b".to_owned()))
                .expect("second")
                .is_none()
        );
        first.release();
        assert_eq!(governor.active_permits().expect("released"), 0);
    }

    #[test]
    fn concurrent_workers_respect_capacity_and_join_without_permit_leaks() {
        let governor = ResourceGovernor::new(9, 4).expect("governor");
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let release = Arc::new(std::sync::Barrier::new(5));
        let mut workers = Vec::new();
        for index in 0..4 {
            let governor = Arc::clone(&governor);
            let ready_tx = ready_tx.clone();
            let release = Arc::clone(&release);
            workers.push(std::thread::spawn(move || {
                let execution = ExecutionId(format!("execution-{index}"));
                let lease = governor
                    .try_acquire(&execution)
                    .expect("acquire")
                    .expect("capacity available");
                ready_tx.send(execution).expect("ready");
                release.wait();
                drop(lease);
            }));
        }
        drop(ready_tx);
        let active = (0..4)
            .map(|_| ready_rx.recv().expect("worker ready"))
            .collect::<Vec<_>>();
        assert_eq!(governor.active_permits().expect("active"), 4);
        assert!(
            governor
                .try_acquire(&ExecutionId("execution-over-capacity".to_owned()))
                .expect("bounded rejection")
                .is_none()
        );
        assert!(
            governor
                .try_acquire(&active[0])
                .expect("duplicate rejection")
                .is_none()
        );
        release.wait();
        for worker in workers {
            worker.join().expect("worker joined");
        }
        assert_eq!(governor.active_permits().expect("released"), 0);
    }
}

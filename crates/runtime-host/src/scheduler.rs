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
    AllowOnceGrant, CancellationToken, ExecuteRequest, HarnessError, ProviderActivityRef,
    ProviderHost, ProviderStatus, TaskDescription, TaskStatus as ProviderTaskStatus,
    ToolDisposition,
};
use runtime_security::SecurityState;
use runtime_security::approval::ApprovalState;

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
            let worker_security = Arc::clone(audit.security);
            if let Ok(worker) = thread::Builder::new()
                .name(format!("dxbot-execution-{}", candidate.execution_id.0))
                .spawn(move || {
                    run_candidate(
                        &worker_application,
                        &worker_providers,
                        &worker_governor,
                        scheduler_generation,
                        &worker_calls,
                        &worker_security,
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
    security: &Arc<Mutex<SecurityState>>,
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
    let mut activity_ref: Option<ProviderActivityRef> = None;
    let task = TaskDescription {
        intent: candidate.task.intent.clone(),
        context: candidate.task.context.clone(),
        budget: candidate.task.budget,
        deadline: candidate.task.deadline,
    };
    // Resolve the request-scoped allow-once authority grant (`DXB-DEL-068`
    // H10 Task 7). It is minted ONLY from an Approved runtime-security Approval
    // bound to action `provider-tool-allow-once`, target the exact execution
    // ref, and a positive policy generation. Absent an approved authority, the
    // grant is `None` and the provider defaults to reject; a DSH/ambient
    // approval never suffices because it never appears in the canonical
    // Security ApprovalManager consulted here.
    let allow_once_grant = resolve_allow_once_grant(security, &candidate.execution_id);
    let side_effect_ref = Some(side_effect_id.clone());
    let core_lease_ref = Some(lease.lease_id.0.clone());
    let request = ExecuteRequest {
        provider_id: candidate.provider_id.clone(),
        provider_generation: candidate.provider_generation,
        intent: task.intent.clone(),
        context: task.context.clone(),
        budget: task.budget,
        deadline: task.deadline,
        permission_refs: candidate.task.permission_refs.clone(),
        resource_refs: candidate.task.resource_refs.clone(),
        side_effect_ref,
        core_lease_ref,
        allow_once_grant,
    };
    // Hold the in-flight activity lease for the whole call (`DXB-DEL-068` H9).
    // The captured generation travels with the lease so a result produced under
    // a generation that has since been superseded (replaced/unregistered) is
    // fenced at commit time and never committed as a fresh result.
    let provider_result = providers
        .execute_with_activity(&request, cancellation)
        .map(|(result, activity)| (result, Some(activity)));
    let provider_result = match provider_result {
        Ok((result, activity)) => {
            activity_ref = activity;
            Ok(result)
        }
        Err(error) => Err(error),
    };
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
            // Commit-time generation fence (`DXB-DEL-068` H9). A result produced
            // under a captured generation that has since been superseded
            // (provider replaced or unregistered mid-flight) must not be
            // committed as a fresh result. The lease kept the old slot alive for
            // this call, but the active generation has moved on, so we mark the
            // side effect Unknown and require reconciliation rather than
            // duplicating an effect under a stale fence.
            let generation_current = activity_ref.as_ref().is_some_and(|activity| {
                providers.is_active_generation(activity.provider_id(), activity.generation())
            });
            if !generation_current {
                let _ = application.mark_provider_side_effect_unknown(
                    &fence,
                    &side_effect_id,
                    "Provider generation superseded before result commit",
                );
                let _ = application.fail_execution(
                    &fence,
                    "Provider generation fenced at commit; reconciliation required",
                    true,
                );
                drop(activity_ref);
                lease.release();
                return;
            }
            // Per-tool side-effect disposition fold (`DXB-DEL-068` H10 Task 8).
            // Any tool effect whose disposition is Unknown forces the execution
            // into RecoveryRequired with the side effect marked Unknown; the
            // committed final result is never produced under an unknown external
            // effect, and there is no blind retry. Rejected/Prepared/Dispatched/
            // Confirmed dispositions do not block a clean commit.
            let has_unknown_effect = result
                .tool_effects
                .iter()
                .any(|effect| effect.disposition == ToolDisposition::Unknown);
            if has_unknown_effect {
                let _ = application.mark_provider_side_effect_unknown(
                    &fence,
                    &side_effect_id,
                    "Tool side effect disposition Unknown; reconciliation required",
                );
                let _ = application.fail_execution(
                    &fence,
                    "Unknown tool side effect; reconciliation required, no blind retry",
                    true,
                );
                drop(activity_ref);
                lease.release();
                return;
            }
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

/// The canonical binding action for a provider tool allow-once authority. Only
/// an Approved runtime-security Approval bound to this exact action, targeting
/// the exact execution, with a positive policy generation, authorizes a single
/// provider allow-once tool decision (`DXB-DEL-068` H10 Task 7).
const ALLOW_ONCE_ACTION: &str = "provider-tool-allow-once";

/// Resolve the request-scoped allow-once authority grant for an execution from
/// the canonical Security ApprovalManager. Returns `Some` only when an Approval
/// is `Approved`, bound to action [`ALLOW_ONCE_ACTION`], targets exactly this
/// execution, and has a positive policy generation. Any other state — pending,
/// denied, mis-targeted, wrong action, non-positive generation, or absent —
/// yields `None`, so the provider defaults to reject. A DSH/ambient approval is
/// never consulted and can never mint a grant.
fn resolve_allow_once_grant(
    security: &Arc<Mutex<SecurityState>>,
    execution_id: &ExecutionId,
) -> Option<AllowOnceGrant> {
    let target = format!("execution:{}", execution_id.0);
    let guard = security.lock().ok()?;
    for approval in guard.approvals.list_approvals() {
        if approval.state == ApprovalState::Approved
            && approval.binding.action == ALLOW_ONCE_ACTION
            && approval.binding.target == target
            && approval.binding.policy_generation > 0
        {
            let grant = AllowOnceGrant {
                approval_ref: approval.id.0.clone(),
                execution_ref: target.clone(),
                policy_generation: approval.binding.policy_generation,
            };
            if grant.is_valid() {
                return Some(grant);
            }
        }
    }
    None
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
        HarnessError::GenerationFenced { .. } => "generation-fenced",
        HarnessError::NoProviderConfigured => "unconfigured",
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
    use dxbot_core::types::{OperationId, PrincipalRef};
    use runtime_security::approval::{ApprovalBinding, ApprovalDecision};

    fn approved_allow_once(
        security: &Arc<Mutex<SecurityState>>,
        execution_ref: &str,
        approver: &PrincipalRef,
    ) {
        let mut guard = security.lock().expect("security");
        let id = guard
            .approvals
            .create_bound_approval(
                OperationId(format!("op-{execution_ref}")),
                ApprovalBinding {
                    action: ALLOW_ONCE_ACTION.to_owned(),
                    target: execution_ref.to_owned(),
                    policy_generation: 3,
                },
                vec![approver.clone()],
            )
            .expect("create approval");
        let state = guard
            .approvals
            .decide_approval(&id, ApprovalDecision::Approve, approver)
            .expect("approve");
        assert_eq!(state, ApprovalState::Approved);
    }

    #[test]
    fn allow_once_grant_requires_approved_authority_bound_to_exact_execution() {
        let security = Arc::new(Mutex::new(SecurityState::new()));
        let approver = PrincipalRef("operator".to_owned());
        let execution = ExecutionId("exec-1".to_owned());
        // No approval yet: default deny (no grant).
        assert!(resolve_allow_once_grant(&security, &execution).is_none());

        // An Approved allow-once bound to the exact execution mints a valid grant.
        approved_allow_once(&security, "execution:exec-1", &approver);
        let grant = resolve_allow_once_grant(&security, &execution).expect("grant");
        assert!(grant.is_valid());
        assert_eq!(grant.execution_ref, "execution:exec-1");
        assert_eq!(grant.policy_generation, 3);
    }

    #[test]
    fn allow_once_grant_is_denied_for_a_different_execution_target() {
        let security = Arc::new(Mutex::new(SecurityState::new()));
        let approver = PrincipalRef("operator".to_owned());
        // Approved, but bound to a DIFFERENT execution: never grants ours.
        approved_allow_once(&security, "execution:other", &approver);
        assert!(
            resolve_allow_once_grant(&security, &ExecutionId("exec-1".to_owned())).is_none(),
            "an approval for another execution must never authorize this one"
        );
    }

    #[test]
    fn allow_once_grant_is_denied_for_wrong_action() {
        let security = Arc::new(Mutex::new(SecurityState::new()));
        let approver = PrincipalRef("operator".to_owned());
        {
            let mut guard = security.lock().expect("security");
            let id = guard
                .approvals
                .create_bound_approval(
                    OperationId("op-x".to_owned()),
                    ApprovalBinding {
                        // Wrong action: not the allow-once tool authority.
                        action: "continue-operation".to_owned(),
                        target: "execution:exec-1".to_owned(),
                        policy_generation: 1,
                    },
                    vec![approver.clone()],
                )
                .expect("create");
            guard
                .approvals
                .decide_approval(&id, ApprovalDecision::Approve, &approver)
                .expect("approve");
        }
        assert!(
            resolve_allow_once_grant(&security, &ExecutionId("exec-1".to_owned())).is_none(),
            "an approval for a different action must never authorize allow-once"
        );
    }

    #[test]
    fn allow_once_grant_is_denied_while_pending() {
        let security = Arc::new(Mutex::new(SecurityState::new()));
        let approver = PrincipalRef("operator".to_owned());
        {
            let mut guard = security.lock().expect("security");
            guard
                .approvals
                .create_bound_approval(
                    OperationId("op-1".to_owned()),
                    ApprovalBinding {
                        action: ALLOW_ONCE_ACTION.to_owned(),
                        target: "execution:exec-1".to_owned(),
                        policy_generation: 1,
                    },
                    vec![approver],
                )
                .expect("create");
            // Left Pending on purpose.
        }
        assert!(
            resolve_allow_once_grant(&security, &ExecutionId("exec-1".to_owned())).is_none(),
            "a pending approval must never authorize allow-once"
        );
    }

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

    #[test]
    fn documented_resource_bounds_are_exactly_active_four_queue_sixtyfour() {
        // `DXB-DEL-068` H11 §10: the runtime's bounded high-water marks are a
        // single-owner fact. Assert the exact documented numbers so a silent
        // drift of the active/queue bound fails this owner test.
        assert_eq!(MAX_ACTIVE_EXECUTIONS, 4, "active high-water must be 4");
        assert_eq!(
            MAX_ADMITTED_QUEUE, 64,
            "admitted queue high-water must be 64"
        );
        // The production coordinator's governor is built at exactly the active
        // bound, so its capacity is the same single-owner number.
        let governor = ResourceGovernor::new(1, MAX_ACTIVE_EXECUTIONS).expect("governor");
        assert_eq!(governor.capacity(), 4);
    }

    #[test]
    fn governor_never_exceeds_active_high_water_under_sequential_pressure() {
        // Deterministic (no timing): acquire up to capacity, then every further
        // acquire is bounded-rejected, and the live permit count never exceeds
        // the high-water mark. Releasing returns the count to zero (no leak).
        let governor = ResourceGovernor::new(2, MAX_ACTIVE_EXECUTIONS).expect("governor");
        let mut leases = Vec::new();
        for index in 0..MAX_ACTIVE_EXECUTIONS {
            let lease = governor
                .try_acquire(&ExecutionId(format!("execution-{index}")))
                .expect("acquire")
                .expect("within capacity");
            leases.push(lease);
            assert!(
                governor.active_permits().expect("active") <= MAX_ACTIVE_EXECUTIONS,
                "live permits must never exceed the high-water mark"
            );
        }
        assert_eq!(governor.active_permits().expect("active"), 4);
        // Pressure beyond capacity: fail closed, no over-admission.
        for over in 0..8 {
            assert!(
                governor
                    .try_acquire(&ExecutionId(format!("over-{over}")))
                    .expect("bounded")
                    .is_none(),
                "over-capacity admission must be rejected, not queued unbounded"
            );
        }
        assert_eq!(governor.active_permits().expect("active"), 4);
        leases.clear();
        assert_eq!(governor.active_permits().expect("released"), 0);
    }
}

//! Canonical Task/Execution/Context Plan run coordination.
//!
//! Runtime consumes immutable plans through bounded candidates. Every state
//! transition is persisted atomically by Application and fenced by Task,
//! Process, Execution, Scheduler, and Provider generations/revisions.

#![forbid(unsafe_code)]

use dxbot_core::types::{ContentSource, ExecutionId, ProcessId, ProviderId, TaskId};

use crate::mutation::{AppError, ApplicationMutator};
use crate::state::{
    ExecutionAuditIntent, ExecutionAuditPhase, ExecutionEvidence, ExecutionStatus,
    ProcessLifecycle, TaskStatus,
};

const MAX_ADMITTED_BATCH: usize = 64;
const MAX_RESULT_BYTES: usize = 1024 * 1024;
const MAX_EVIDENCE_ITEMS: usize = 64;
const MAX_EVIDENCE_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionWork {
    pub intent: String,
    pub context: String,
    pub budget: Option<u64>,
    pub deadline: Option<i64>,
    /// Immutable Context Plan permission references, propagated into the
    /// provider call so a provider-neutral tool/permission bridge can attribute
    /// requests to the committed plan (`DXB-DEL-068` H10 Task 4). In-memory
    /// candidate field only; no persisted schema change.
    pub permission_refs: Vec<String>,
    /// Immutable Context Plan resource references / budget ref, propagated for
    /// the same reason. In-memory candidate field only.
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionEvidenceInput {
    pub provider_id: ProviderId,
    pub observation: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionResultInput {
    pub output: String,
    pub evidence: Vec<ExecutionEvidenceInput>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionCandidate {
    pub execution_id: ExecutionId,
    pub execution_generation: i64,
    pub task_id: TaskId,
    pub task_revision: i64,
    pub process_id: ProcessId,
    pub process_revision: i64,
    pub provider_id: ProviderId,
    pub provider_generation: i64,
    pub task: ExecutionWork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionRunFence {
    pub execution_id: ExecutionId,
    pub execution_generation: i64,
    pub scheduler_generation: i64,
    pub provider_id: ProviderId,
    pub provider_generation: i64,
    pub task_id: TaskId,
    pub task_revision: i64,
    pub process_id: ProcessId,
    pub process_revision: i64,
}

impl ApplicationMutator {
    pub fn admitted_executions(&self, limit: usize) -> Result<Vec<ExecutionCandidate>, AppError> {
        let limit = limit.min(MAX_ADMITTED_BATCH);
        let guard = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))?;
        let mut executions = guard
            .executions
            .values()
            .filter(|execution| execution.status == ExecutionStatus::Admitted)
            .collect::<Vec<_>>();
        executions.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        executions
            .into_iter()
            .take(limit)
            .map(|execution| {
                let task = guard.tasks.get(&execution.task_id).ok_or_else(|| {
                    AppError::Internal(format!(
                        "execution {} references missing task {}",
                        execution.id.0, execution.task_id.0
                    ))
                })?;
                let process_id = task
                    .process_ref
                    .as_deref()
                    .and_then(|reference| reference.strip_prefix("process:"))
                    .map(|value| ProcessId(value.to_owned()))
                    .ok_or_else(|| {
                        AppError::Internal(format!(
                            "task {} has no canonical process ref",
                            task.id.0
                        ))
                    })?;
                let process = guard.processes.get(&process_id).ok_or_else(|| {
                    AppError::Internal(format!(
                        "task {} references missing process {}",
                        task.id.0, process_id.0
                    ))
                })?;
                let intent = match task.intent.as_ref() {
                    Some(ContentSource::Text { value }) => value.clone(),
                    Some(ContentSource::ArtifactRef {
                        artifact_id,
                        digest,
                    }) => {
                        format!("artifact:{artifact_id}@{digest}")
                    }
                    Some(ContentSource::InputFile { .. } | ContentSource::Stdin) | None => {
                        return Err(AppError::Conflict(format!(
                            "task {} intent is not materialized for execution",
                            task.id.0
                        )));
                    }
                };
                let budget = execution.context_plan.max_output_tokens.map(u64::from);
                let deadline = execution.context_plan.deadline_unix_seconds;
                Ok(ExecutionCandidate {
                    execution_id: execution.id.clone(),
                    execution_generation: execution.generation,
                    task_id: task.id.clone(),
                    task_revision: task.revision,
                    process_id,
                    process_revision: process.revision,
                    provider_id: execution.context_plan.provider_binding.provider_id.clone(),
                    provider_generation: execution.context_plan.provider_binding.generation,
                    task: ExecutionWork {
                        intent,
                        context: execution.context_plan.bounded_context.clone(),
                        budget,
                        deadline,
                        permission_refs: execution.context_plan.permission_refs.clone(),
                        resource_refs: execution
                            .context_plan
                            .resource_budget
                            .clone()
                            .into_iter()
                            .collect(),
                    },
                })
            })
            .collect()
    }

    pub fn execution_audit_intents(
        &self,
        limit: usize,
    ) -> Result<Vec<ExecutionAuditIntent>, AppError> {
        let state = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))?;
        let mut intents = state
            .execution_audit_intents
            .values()
            .cloned()
            .collect::<Vec<_>>();
        intents.sort_by(|left, right| left.key.cmp(&right.key));
        intents.truncate(limit.min(MAX_EXECUTION_AUDIT_INTENTS));
        Ok(intents)
    }

    pub fn mark_admission_recovery_required(
        &self,
        candidate: &ExecutionCandidate,
        reason: &str,
    ) -> Result<(), AppError> {
        let reason = bounded_reason(reason);
        self.execution_transaction(|state| {
            let execution = state
                .executions
                .get_mut(&candidate.execution_id)
                .ok_or_else(|| AppError::NotFound("Execution does not exist".to_owned()))?;
            if execution.generation != candidate.execution_generation
                || execution.status != ExecutionStatus::Admitted
            {
                return Err(AppError::Conflict("stale Execution admission".to_owned()));
            }
            execution.status = ExecutionStatus::RecoveryRequired;
            execution.terminal_reason = Some(reason.clone());
            let task = state
                .tasks
                .get_mut(&candidate.task_id)
                .ok_or_else(|| AppError::NotFound("Execution Task does not exist".to_owned()))?;
            if task.revision != candidate.task_revision || task.status != TaskStatus::Admitted {
                return Err(AppError::Conflict("stale Task admission".to_owned()));
            }
            task.status = TaskStatus::RecoveryRequired;
            task.revision = checked_next(task.revision, "task")?;
            let process = state
                .processes
                .get_mut(&candidate.process_id)
                .ok_or_else(|| AppError::NotFound("Execution Process does not exist".to_owned()))?;
            if process.revision != candidate.process_revision
                || process.lifecycle != ProcessLifecycle::Running
            {
                return Err(AppError::Conflict("stale Process admission".to_owned()));
            }
            process.lifecycle = ProcessLifecycle::RecoveryRequired;
            process.current_activity_ref = None;
            process.continuation_ref = None;
            process.terminal_reason = Some(reason.clone());
            process.revision = checked_next(process.revision, "process")?;
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &candidate.execution_id,
                    execution_generation: candidate.execution_generation,
                    task_id: &candidate.task_id,
                    process_id: &candidate.process_id,
                    provider_id: &candidate.provider_id,
                    provider_generation: candidate.provider_generation,
                    phase: ExecutionAuditPhase::RecoveryRequired,
                    detail: &reason,
                },
            )?;
            Ok(())
        })
    }

    pub fn begin_execution(
        &self,
        candidate: &ExecutionCandidate,
        scheduler_generation: i64,
    ) -> Result<ExecutionRunFence, AppError> {
        if scheduler_generation <= 0 {
            return Err(AppError::Conflict(
                "SchedulerGeneration must be positive".to_owned(),
            ));
        }
        self.execution_transaction(|state| {
            let execution = state
                .executions
                .get_mut(&candidate.execution_id)
                .ok_or_else(|| AppError::NotFound("execution does not exist".to_owned()))?;
            if execution.generation != candidate.execution_generation
                || execution.status != ExecutionStatus::Admitted
                || execution.context_plan.provider_binding.generation
                    != candidate.provider_generation
            {
                return Err(AppError::Conflict(
                    "stale or non-admitted Execution candidate".to_owned(),
                ));
            }
            let task = state
                .tasks
                .get_mut(&candidate.task_id)
                .ok_or_else(|| AppError::NotFound("execution Task does not exist".to_owned()))?;
            if task.revision != candidate.task_revision || task.status != TaskStatus::Admitted {
                return Err(AppError::Conflict(
                    "stale Task admission revision".to_owned(),
                ));
            }
            let process = state
                .processes
                .get_mut(&candidate.process_id)
                .ok_or_else(|| AppError::NotFound("execution Process does not exist".to_owned()))?;
            if process.revision != candidate.process_revision
                || process.lifecycle != ProcessLifecycle::Running
            {
                return Err(AppError::Conflict(
                    "stale Process admission revision".to_owned(),
                ));
            }

            execution.status = ExecutionStatus::Running;
            execution.scheduler_generation = Some(scheduler_generation);
            task.status = TaskStatus::Running;
            task.revision = checked_next(task.revision, "task")?;
            process.current_activity_ref = Some(format!("execution:{}", execution.id.0));
            process.continuation_ref = Some(format!("continue:execution:{}", execution.id.0));
            process.progress = checked_next(process.progress, "process progress")?;
            process.revision = checked_next(process.revision, "process")?;

            let fence = ExecutionRunFence {
                execution_id: execution.id.clone(),
                execution_generation: execution.generation,
                scheduler_generation,
                provider_id: execution.context_plan.provider_binding.provider_id.clone(),
                provider_generation: execution.context_plan.provider_binding.generation,
                task_id: task.id.clone(),
                task_revision: task.revision,
                process_id: process.id.clone(),
                process_revision: process.revision,
            };
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &fence.execution_id,
                    execution_generation: fence.execution_generation,
                    task_id: &fence.task_id,
                    process_id: &fence.process_id,
                    provider_id: &fence.provider_id,
                    provider_generation: fence.provider_generation,
                    phase: ExecutionAuditPhase::Running,
                    detail: "Core Lease acquired and Provider execution started",
                },
            )?;
            Ok(fence)
        })
    }

    pub fn prepare_provider_side_effect(
        &self,
        fence: &ExecutionRunFence,
    ) -> Result<String, AppError> {
        self.execution_transaction(|state| {
            let execution = fenced_execution(state, fence)?;
            if execution.status != ExecutionStatus::Running {
                return Err(AppError::Conflict("Execution is not running".to_owned()));
            }
            let id = format!(
                "side-effect:provider:{}:{}",
                fence.execution_id.0, fence.execution_generation
            );
            if let Some(existing) = state.side_effects.get(&id) {
                if existing.execution_ref.as_ref() == Some(&fence.execution_id)
                    && existing.process_ref.as_ref() == Some(&fence.process_id)
                {
                    return Ok(id);
                }
                return Err(AppError::Conflict(
                    "Side Effect identity collision".to_owned(),
                ));
            }
            state.side_effects.insert(
                id.clone(),
                crate::state::SideEffectState {
                    id: id.clone(),
                    revision: 1,
                    status: crate::state::SideEffectStatus::Prepared,
                    evidence: Vec::new(),
                    operation_ref: None,
                    execution_ref: Some(fence.execution_id.clone()),
                    process_ref: Some(fence.process_id.clone()),
                },
            );
            Ok(id)
        })
    }

    pub fn mark_provider_side_effect_dispatched(
        &self,
        fence: &ExecutionRunFence,
        side_effect_id: &str,
    ) -> Result<(), AppError> {
        self.execution_transaction(|state| {
            let execution = fenced_execution(state, fence)?;
            if execution.status != ExecutionStatus::Running {
                return Err(AppError::Conflict("Execution is not running".to_owned()));
            }
            let effect = state.side_effects.get_mut(side_effect_id).ok_or_else(|| {
                AppError::NotFound("Provider Side Effect does not exist".to_owned())
            })?;
            if effect.status != crate::state::SideEffectStatus::Prepared
                || effect.execution_ref.as_ref() != Some(&fence.execution_id)
            {
                return Err(AppError::Conflict(
                    "stale Side Effect dispatch fence".to_owned(),
                ));
            }
            effect.status = crate::state::SideEffectStatus::Dispatched;
            effect.revision = checked_next(effect.revision, "side effect")?;
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &fence.execution_id,
                    execution_generation: fence.execution_generation,
                    task_id: &fence.task_id,
                    process_id: &fence.process_id,
                    provider_id: &fence.provider_id,
                    provider_generation: fence.provider_generation,
                    phase: ExecutionAuditPhase::Dispatched,
                    detail: "Provider request dispatched after durable Side Effect preparation",
                },
            )?;
            Ok(())
        })
    }

    pub fn mark_provider_side_effect_unknown(
        &self,
        fence: &ExecutionRunFence,
        side_effect_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        let reason = bounded_reason(reason);
        self.execution_transaction(|state| {
            let execution = fenced_execution(state, fence)?;
            if execution.status != ExecutionStatus::Running {
                return Err(AppError::Conflict("Execution is not running".to_owned()));
            }
            let effect = state.side_effects.get_mut(side_effect_id).ok_or_else(|| {
                AppError::NotFound("Provider Side Effect does not exist".to_owned())
            })?;
            if effect.status != crate::state::SideEffectStatus::Dispatched
                || effect.execution_ref.as_ref() != Some(&fence.execution_id)
            {
                return Err(AppError::Conflict(
                    "stale Side Effect unknown fence".to_owned(),
                ));
            }
            effect.status = crate::state::SideEffectStatus::Unknown;
            effect.revision = checked_next(effect.revision, "side effect")?;
            effect.evidence.push(reason);
            Ok(())
        })
    }

    pub fn complete_execution(
        &self,
        fence: &ExecutionRunFence,
        result: &ExecutionResultInput,
    ) -> Result<(), AppError> {
        self.complete_execution_with_side_effect(fence, result, None)
    }

    pub fn complete_execution_with_side_effect(
        &self,
        fence: &ExecutionRunFence,
        result: &ExecutionResultInput,
        side_effect_id: Option<&str>,
    ) -> Result<(), AppError> {
        validate_result(result)?;
        self.execution_transaction(|state| {
            if let Some(side_effect_id) = side_effect_id {
                let effect = state.side_effects.get(side_effect_id).ok_or_else(|| {
                    AppError::NotFound("Provider Side Effect does not exist".to_owned())
                })?;
                if effect.status != crate::state::SideEffectStatus::Dispatched
                    || effect.execution_ref.as_ref() != Some(&fence.execution_id)
                    || effect.process_ref.as_ref() != Some(&fence.process_id)
                {
                    return Err(AppError::Conflict(
                        "stale Provider Side Effect result fence".to_owned(),
                    ));
                }
            }
            let execution = fenced_execution(state, fence)?;
            if execution.status != ExecutionStatus::Running {
                return Err(AppError::Conflict("Execution is not running".to_owned()));
            }
            execution.status = ExecutionStatus::Succeeded;
            execution.result = Some(result.output.clone());
            execution.evidence = result
                .evidence
                .iter()
                .map(|evidence| ExecutionEvidence {
                    provider_id: evidence.provider_id.clone(),
                    observation: evidence.observation.clone(),
                })
                .collect();

            let task = state
                .tasks
                .get_mut(&fence.task_id)
                .ok_or_else(|| AppError::NotFound("Execution Task does not exist".to_owned()))?;
            if task.revision != fence.task_revision || task.status != TaskStatus::Running {
                return Err(AppError::Conflict("stale Task result fence".to_owned()));
            }
            task.status = TaskStatus::Succeeded;
            task.result = Some(serde_json::json!({
                "execution_ref": format!("execution:{}", fence.execution_id.0),
                "output": result.output,
                "evidence": result.evidence.iter().map(|evidence| serde_json::json!({
                    "provider_id": evidence.provider_id.0,
                    "observation": evidence.observation,
                })).collect::<Vec<_>>(),
            }));
            task.revision = checked_next(task.revision, "task")?;

            let process = state
                .processes
                .get_mut(&fence.process_id)
                .ok_or_else(|| AppError::NotFound("Execution Process does not exist".to_owned()))?;
            if process.revision != fence.process_revision
                || process.lifecycle != ProcessLifecycle::Running
            {
                return Err(AppError::Conflict("stale Process result fence".to_owned()));
            }
            process.lifecycle = ProcessLifecycle::Completed;
            process.current_activity_ref = None;
            process.continuation_ref = None;
            process
                .outcome_refs
                .push(format!("execution-result:{}", fence.execution_id.0));
            process.progress = checked_next(process.progress, "process progress")?;
            process.revision = checked_next(process.revision, "process")?;
            if let Some(side_effect_id) = side_effect_id {
                let effect = state.side_effects.get_mut(side_effect_id).ok_or_else(|| {
                    AppError::NotFound("Provider Side Effect does not exist".to_owned())
                })?;
                effect.status = crate::state::SideEffectStatus::Confirmed;
                effect.revision = checked_next(effect.revision, "side effect")?;
                effect
                    .evidence
                    .push(format!("execution-result:{}", fence.execution_id.0));
            }
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &fence.execution_id,
                    execution_generation: fence.execution_generation,
                    task_id: &fence.task_id,
                    process_id: &fence.process_id,
                    provider_id: &fence.provider_id,
                    provider_generation: fence.provider_generation,
                    phase: ExecutionAuditPhase::Succeeded,
                    detail: "Provider result and evidence committed",
                },
            )?;
            Ok(())
        })
    }

    pub fn fail_execution(
        &self,
        fence: &ExecutionRunFence,
        reason: &str,
        recovery_required: bool,
    ) -> Result<(), AppError> {
        let reason = bounded_reason(reason);
        self.execution_transaction(|state| {
            let execution = fenced_execution(state, fence)?;
            if execution.status != ExecutionStatus::Running {
                return Err(AppError::Conflict("Execution is not running".to_owned()));
            }
            execution.status = if recovery_required {
                ExecutionStatus::RecoveryRequired
            } else {
                ExecutionStatus::Failed
            };
            execution.terminal_reason = Some(reason.clone());
            let task = state
                .tasks
                .get_mut(&fence.task_id)
                .ok_or_else(|| AppError::NotFound("Execution Task does not exist".to_owned()))?;
            if task.revision != fence.task_revision || task.status != TaskStatus::Running {
                return Err(AppError::Conflict("stale Task failure fence".to_owned()));
            }
            task.status = if recovery_required {
                TaskStatus::RecoveryRequired
            } else {
                TaskStatus::Failed
            };
            task.revision = checked_next(task.revision, "task")?;
            let process = state
                .processes
                .get_mut(&fence.process_id)
                .ok_or_else(|| AppError::NotFound("Execution Process does not exist".to_owned()))?;
            if process.revision != fence.process_revision
                || process.lifecycle != ProcessLifecycle::Running
            {
                return Err(AppError::Conflict("stale Process failure fence".to_owned()));
            }
            process.lifecycle = if recovery_required {
                ProcessLifecycle::RecoveryRequired
            } else {
                ProcessLifecycle::Failed
            };
            process.current_activity_ref = None;
            process.continuation_ref = None;
            process.terminal_reason = Some(reason.clone());
            process.progress = checked_next(process.progress, "process progress")?;
            process.revision = checked_next(process.revision, "process")?;
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &fence.execution_id,
                    execution_generation: fence.execution_generation,
                    task_id: &fence.task_id,
                    process_id: &fence.process_id,
                    provider_id: &fence.provider_id,
                    provider_generation: fence.provider_generation,
                    phase: if recovery_required {
                        ExecutionAuditPhase::RecoveryRequired
                    } else {
                        ExecutionAuditPhase::Failed
                    },
                    detail: &reason,
                },
            )?;
            Ok(())
        })
    }

    pub fn execution_should_cancel(&self, execution_id: &ExecutionId) -> Result<bool, AppError> {
        let state = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))?;
        let execution = state
            .executions
            .get(execution_id)
            .ok_or_else(|| AppError::NotFound("Execution does not exist".to_owned()))?;
        let task = state
            .tasks
            .get(&execution.task_id)
            .ok_or_else(|| AppError::NotFound("Execution Task does not exist".to_owned()))?;
        Ok(
            matches!(task.status, TaskStatus::Cancelled | TaskStatus::Suspended)
                || execution.status == ExecutionStatus::Cancelled,
        )
    }

    pub fn finalize_cancelled_execution(
        &self,
        fence: &ExecutionRunFence,
        reason: &str,
    ) -> Result<(), AppError> {
        let reason = bounded_reason(reason);
        self.execution_transaction(|state| {
            let task = state
                .tasks
                .get(&fence.task_id)
                .ok_or_else(|| AppError::NotFound("Execution Task does not exist".to_owned()))?;
            if !matches!(task.status, TaskStatus::Cancelled | TaskStatus::Suspended) {
                return Err(AppError::Conflict(
                    "Execution cancellation lacks durable Task control".to_owned(),
                ));
            }
            let execution = fenced_execution(state, fence)?;
            if execution.status != ExecutionStatus::Running {
                return Err(AppError::Conflict("Execution is not running".to_owned()));
            }
            execution.status = ExecutionStatus::Cancelled;
            execution.terminal_reason = Some(reason.clone());
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &fence.execution_id,
                    execution_generation: fence.execution_generation,
                    task_id: &fence.task_id,
                    process_id: &fence.process_id,
                    provider_id: &fence.provider_id,
                    provider_generation: fence.provider_generation,
                    phase: ExecutionAuditPhase::Cancelled,
                    detail: &reason,
                },
            )?;
            Ok(())
        })
    }

    /// On startup, an in-flight Provider activity has an uncertain outcome.
    /// It is fenced as RecoveryRequired and is never blindly re-dispatched.
    pub fn recover_inflight_executions(&self) -> Result<usize, AppError> {
        self.execution_transaction(|state| {
            for effect in state.side_effects.values_mut() {
                if effect.status == crate::state::SideEffectStatus::Dispatched
                    && effect.execution_ref.is_some()
                {
                    effect.status = crate::state::SideEffectStatus::Unknown;
                    effect.revision = checked_next(effect.revision, "side effect")?;
                    effect
                        .evidence
                        .push("Runtime restarted with uncertain Provider outcome".to_owned());
                }
            }
            let running = state
                .executions
                .values()
                .filter(|execution| execution.status == ExecutionStatus::Running)
                .map(|execution| execution.id.clone())
                .collect::<Vec<_>>();
            for execution_id in &running {
                let execution = state
                    .executions
                    .get_mut(execution_id)
                    .ok_or_else(|| AppError::Internal("Execution disappeared".to_owned()))?;
                execution.status = ExecutionStatus::RecoveryRequired;
                execution.terminal_reason =
                    Some("Runtime restarted during Provider activity".to_owned());
                if let Some(task) = state.tasks.get_mut(&execution.task_id) {
                    task.status = TaskStatus::RecoveryRequired;
                    task.revision = checked_next(task.revision, "task")?;
                    if let Some(process_id) = task
                        .process_ref
                        .as_deref()
                        .and_then(|reference| reference.strip_prefix("process:"))
                        .map(|value| ProcessId(value.to_owned()))
                    {
                        if let Some(process) = state.processes.get_mut(&process_id) {
                            process.lifecycle = ProcessLifecycle::RecoveryRequired;
                            process.current_activity_ref = None;
                            process.continuation_ref = None;
                            process.terminal_reason =
                                Some("Runtime restarted during Provider activity".to_owned());
                            process.revision = checked_next(process.revision, "process")?;
                        }
                    }
                }
            }
            for execution_id in &running {
                let execution = state
                    .executions
                    .get(execution_id)
                    .cloned()
                    .ok_or_else(|| AppError::Internal("Execution disappeared".to_owned()))?;
                let task = state
                    .tasks
                    .get(&execution.task_id)
                    .ok_or_else(|| AppError::Internal("Execution Task disappeared".to_owned()))?;
                let process_id = task
                    .process_ref
                    .as_deref()
                    .and_then(|reference| reference.strip_prefix("process:"))
                    .map(|value| ProcessId(value.to_owned()))
                    .ok_or_else(|| {
                        AppError::Internal("Execution Process disappeared".to_owned())
                    })?;
                record_execution_audit(
                    state,
                    AuditIntentInput {
                        execution_id,
                        execution_generation: execution.generation,
                        task_id: &execution.task_id,
                        process_id: &process_id,
                        provider_id: &execution.context_plan.provider_binding.provider_id,
                        provider_generation: execution.context_plan.provider_binding.generation,
                        phase: ExecutionAuditPhase::RecoveryRequired,
                        detail: "Runtime restarted during uncertain Provider activity",
                    },
                )?;
            }
            Ok(running.len())
        })
    }

    fn execution_transaction<T>(
        &self,
        operation: impl FnOnce(&mut crate::state::DomainState) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))?;
        let rollback = state.clone();
        let value = operation(&mut state)?;
        if let Some(store) = &self.persistence {
            if let Err(error) = store.persist(&state) {
                *state = rollback;
                return Err(error);
            }
        }
        Ok(value)
    }
}

fn fenced_execution<'a>(
    state: &'a mut crate::state::DomainState,
    fence: &ExecutionRunFence,
) -> Result<&'a mut crate::state::ExecutionState, AppError> {
    let execution = state
        .executions
        .get_mut(&fence.execution_id)
        .ok_or_else(|| AppError::NotFound("Execution does not exist".to_owned()))?;
    if execution.generation != fence.execution_generation
        || execution.scheduler_generation != Some(fence.scheduler_generation)
        || execution.context_plan.provider_binding.provider_id != fence.provider_id
        || execution.context_plan.provider_binding.generation != fence.provider_generation
    {
        return Err(AppError::Conflict(
            "stale Execution result generation".to_owned(),
        ));
    }
    Ok(execution)
}

fn validate_result(result: &ExecutionResultInput) -> Result<(), AppError> {
    if result.output.len() > MAX_RESULT_BYTES {
        return Err(AppError::Conflict(
            "Provider result exceeds byte cap".to_owned(),
        ));
    }
    if result.evidence.len() > MAX_EVIDENCE_ITEMS
        || result
            .evidence
            .iter()
            .any(|evidence| evidence.observation.len() > MAX_EVIDENCE_BYTES)
    {
        return Err(AppError::Conflict(
            "Provider evidence exceeds bounded limits".to_owned(),
        ));
    }
    Ok(())
}

const MAX_EXECUTION_AUDIT_INTENTS: usize = 100_000;
const MAX_AUDIT_DETAIL_BYTES: usize = 1024;

pub(crate) struct AuditIntentInput<'a> {
    pub(crate) execution_id: &'a ExecutionId,
    pub(crate) execution_generation: i64,
    pub(crate) task_id: &'a TaskId,
    pub(crate) process_id: &'a ProcessId,
    pub(crate) provider_id: &'a ProviderId,
    pub(crate) provider_generation: i64,
    pub(crate) phase: ExecutionAuditPhase,
    pub(crate) detail: &'a str,
}

pub(crate) fn record_execution_audit(
    state: &mut crate::state::DomainState,
    input: AuditIntentInput<'_>,
) -> Result<(), AppError> {
    let phase = audit_phase_name(input.phase);
    let key = format!(
        "execution:{}:{}:{phase}",
        input.execution_id.0, input.execution_generation
    );
    let detail = bounded_audit_detail(input.detail);
    if let Some(existing) = state.execution_audit_intents.get(&key) {
        if existing.execution_id == *input.execution_id
            && existing.execution_generation == input.execution_generation
            && existing.task_id == *input.task_id
            && existing.process_id == *input.process_id
            && existing.provider_id == *input.provider_id
            && existing.provider_generation == input.provider_generation
            && existing.phase == input.phase
            && existing.detail == detail
        {
            return Ok(());
        }
        return Err(AppError::Conflict(format!(
            "Execution AuditIntent conflict: {key}"
        )));
    }
    if state.execution_audit_intents.len() >= MAX_EXECUTION_AUDIT_INTENTS {
        return Err(AppError::ResourceExhausted(format!(
            "Execution AuditIntent capacity {MAX_EXECUTION_AUDIT_INTENTS} is exhausted"
        )));
    }
    state.execution_audit_intents.insert(
        key.clone(),
        ExecutionAuditIntent {
            key,
            execution_id: input.execution_id.clone(),
            execution_generation: input.execution_generation,
            task_id: input.task_id.clone(),
            process_id: input.process_id.clone(),
            provider_id: input.provider_id.clone(),
            provider_generation: input.provider_generation,
            phase: input.phase,
            detail,
            created_at: now_secs(),
        },
    );
    Ok(())
}

fn bounded_audit_detail(value: &str) -> String {
    if value.len() <= MAX_AUDIT_DETAIL_BYTES {
        return value.to_owned();
    }
    let mut end = MAX_AUDIT_DETAIL_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn audit_phase_name(phase: ExecutionAuditPhase) -> &'static str {
    match phase {
        ExecutionAuditPhase::Admitted => "admitted",
        ExecutionAuditPhase::Running => "running",
        ExecutionAuditPhase::Dispatched => "dispatched",
        ExecutionAuditPhase::Succeeded => "succeeded",
        ExecutionAuditPhase::Failed => "failed",
        ExecutionAuditPhase::Cancelled => "cancelled",
        ExecutionAuditPhase::RecoveryRequired => "recovery-required",
    }
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn checked_next(value: i64, owner: &str) -> Result<i64, AppError> {
    value
        .checked_add(1)
        .ok_or_else(|| AppError::Internal(format!("{owner} revision exhausted")))
}

fn bounded_reason(reason: &str) -> String {
    reason.chars().take(1024).collect()
}

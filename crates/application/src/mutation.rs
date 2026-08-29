//! Canonical Application mutation owner.
//!
//! CommandId/OperationId/IdempotencyKey/RequestDigest are preserved unchanged
//! through mutation, receipt and persistence. Domain changes, operation
//! bindings and the optional cross-owner commit hook are rolled back together
//! if validation, security coordination or durable publication fails.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::*;

use crate::delegation::{DelegationRecord, DelegationStatus};
use crate::execution::{AuditIntentInput, record_execution_audit};
use crate::membership::{MembershipManager, MembershipRecord, membership_key};
use crate::outcome::{DomainOutcome, resolve_outcome};
use crate::persistence::ApplicationStateStore;
use crate::query::scope_owner;
use crate::state::{
    BotPolicyBindings, BotState, ChannelState, ContextMemoryRef, ContextPlan, ConversationOwner,
    ConversationState, DeclassificationRecord, DomainState, ExecutionAuditPhase, ExecutionState,
    ExecutionStatus, IdempotencyBindingState, LifecycleState, MemoryAssertionStatus,
    MemoryRevisionState, MemoryState, MessageState, ProcessLifecycle, ProcessState,
    ProjectLifecycle, ProjectState, ProviderBinding, SideEffectStatus, TaskControlDirective,
    TaskExecutionConstraints, TaskState, TaskStatus, ThreadState,
};

const MAX_NONTERMINAL_EXECUTIONS: usize = 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    NotFound(String),
    Conflict(String),
    PermissionDenied(String),
    ResourceExhausted(String),
    Internal(String),
    GapDetected(String),
    Timeout(String),
}

#[derive(Debug, Clone)]
struct OperationIdentity {
    operation_id: OperationId,
    command_id: CommandId,
    instance_id: InstanceId,
    request_digest: RequestDigest,
}

type CommitHook<'a> = dyn FnMut(&CommandPayload, &DomainState) -> Result<(), AppError> + 'a;

#[derive(Debug)]
pub struct ApplicationMutator {
    pub(crate) state: Arc<Mutex<DomainState>>,
    pub(crate) persistence: Option<Arc<ApplicationStateStore>>,
}

impl Default for ApplicationMutator {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationMutator {
    pub fn new() -> Self {
        Self::with_state(DomainState::new())
    }

    pub fn with_state(state: DomainState) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
            persistence: None,
        }
    }

    pub fn with_persistent_state(path: PathBuf) -> Result<Self, AppError> {
        let (store, state) = ApplicationStateStore::open(path)?;
        Ok(Self {
            state: Arc::new(Mutex::new(state)),
            persistence: Some(Arc::new(store)),
        })
    }

    pub fn snapshot(&self) -> Result<DomainState, AppError> {
        self.lock()
            .map(|guard| guard.clone())
            .map_err(AppError::Internal)
    }

    pub fn mutate(&self, request: &OperationRequest) -> Result<OperationResult, AppError> {
        self.mutate_internal(request, None, None)
    }

    pub fn mutate_with_provider_binding(
        &self,
        request: &OperationRequest,
        provider_binding: &ProviderBinding,
    ) -> Result<OperationResult, AppError> {
        if !matches!(
            request.payload.command_key.as_str(),
            "task-submit" | "task-resume"
        ) {
            return Err(AppError::Conflict(
                "Provider binding is only valid for task-submit or task-resume".to_owned(),
            ));
        }
        self.mutate_internal(request, None, Some(provider_binding))
    }

    pub fn mutate_with_commit_hook<F>(
        &self,
        request: &OperationRequest,
        mut hook: F,
    ) -> Result<OperationResult, AppError>
    where
        F: FnMut(&CommandPayload, &DomainState) -> Result<(), AppError>,
    {
        self.mutate_internal(request, Some(&mut hook), None)
    }

    fn mutate_internal(
        &self,
        request: &OperationRequest,
        mut hook: Option<&mut CommitHook<'_>>,
        provider_binding: Option<&ProviderBinding>,
    ) -> Result<OperationResult, AppError> {
        validate_request_identity(request)?;
        let payload = &request.payload;
        let binding_key = (
            request.idempotency_key.principal_ref.0.clone(),
            request.idempotency_key.key_digest.clone(),
        );
        let mut guard = self.lock().map_err(|message| {
            AppError::Internal(format!("domain state lock unavailable: {message}"))
        })?;

        match (
            guard.command_bindings.get(&request.command_id),
            guard.idempotency_bindings.get(&binding_key),
        ) {
            (Some(operation_id), Some(bound_binding)) => {
                validate_existing_binding(&guard, operation_id, bound_binding, request)?;
                let result = guard.results.get(operation_id).cloned().ok_or_else(|| {
                    AppError::Internal(
                        "committed command binding exists without operation result".to_owned(),
                    )
                })?;
                validate_committed_result(&result, request)?;
                return Ok(result);
            }
            (None, None) => {}
            _ => {
                return Err(AppError::Conflict(
                    "incomplete or conflicting command/idempotency binding".to_owned(),
                ));
            }
        }
        if guard.receipts.contains_key(&request.new_operation_id)
            || guard.results.contains_key(&request.new_operation_id)
        {
            return Err(AppError::Conflict(
                "new operation id is already in use".to_owned(),
            ));
        }

        let rollback = guard.clone();
        let identity = OperationIdentity {
            operation_id: request.new_operation_id.clone(),
            command_id: request.command_id.clone(),
            instance_id: payload.instance_id.clone(),
            request_digest: request.request_digest.clone(),
        };
        let committed_payload =
            match apply_command(&mut guard, payload, &identity, provider_binding) {
                Ok(payload) => payload,
                Err(error) => {
                    *guard = rollback;
                    return Err(error);
                }
            };
        let receipt = ReceiptRecord {
            operation_id: identity.operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: format!("operation:{}", identity.operation_id.0),
            resolved_binding_digest: identity.request_digest.0.clone(),
            owner_kind: "application".to_owned(),
            lease_until: None,
            last_progress: 1,
            reconciliation_policy: "idempotent".to_owned(),
        };
        let result = OperationResult {
            operation_id: identity.operation_id.clone(),
            command_id: identity.command_id.clone(),
            instance_id: identity.instance_id.clone(),
            receipt: receipt.clone(),
            status: "committed".to_owned(),
            committed_payload,
            error: None,
            operation_may_continue: false,
        };
        guard
            .receipts
            .insert(identity.operation_id.clone(), receipt);
        guard
            .results
            .insert(identity.operation_id.clone(), result.clone());
        guard
            .command_request_digests
            .insert(identity.command_id.clone(), identity.request_digest.clone());
        guard
            .command_bindings
            .insert(identity.command_id.clone(), identity.operation_id.clone());
        guard.idempotency_bindings.insert(
            binding_key,
            IdempotencyBindingState {
                command_id: identity.command_id,
                expires_at: request.idempotency_key.expires_at,
            },
        );

        if let Some(hook) = hook.as_mut() {
            if let Err(error) = hook(payload, &guard) {
                *guard = rollback;
                return Err(error);
            }
        }
        if let Some(store) = &self.persistence {
            if let Err(error) = store.persist(&guard) {
                *guard = rollback;
                return Err(error);
            }
        }
        Ok(result)
    }

    pub fn lookup_binding(
        &self,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<Option<OperationResult>, AppError> {
        let guard = self.lock().map_err(AppError::Internal)?;
        let binding_key = (key.principal_ref.0.clone(), key.key_digest.clone());
        let Some(bound) = guard.idempotency_bindings.get(&binding_key) else {
            return Ok(None);
        };
        if bound.command_id != *command_id || bound.expires_at != key.expires_at {
            return Err(AppError::Conflict(
                "idempotency binding does not match command lookup".to_owned(),
            ));
        }
        let operation_id = guard.command_bindings.get(command_id).ok_or_else(|| {
            AppError::Internal("idempotency binding exists without command binding".to_owned())
        })?;
        guard
            .results
            .get(operation_id)
            .cloned()
            .map(Some)
            .ok_or_else(|| {
                AppError::Internal("command binding exists without operation result".to_owned())
            })
    }

    pub fn validate_target(
        &self,
        target: &CanonicalTarget,
        cas: &Option<CasConditions>,
    ) -> Result<(), AppError> {
        let guard = self.lock().map_err(AppError::Internal)?;
        validate_target_against(&guard, target, cas)
    }

    pub fn get_receipt(
        &self,
        operation_id: &OperationId,
    ) -> Result<Option<ReceiptRecord>, AppError> {
        let guard = self.lock().map_err(AppError::Internal)?;
        Ok(guard.receipts.get(operation_id).cloned())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, String> {
        self.state
            .lock()
            .map_err(|_| "domain state mutex poisoned".to_owned())
    }
}

fn apply_command(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
    provider_binding: Option<&ProviderBinding>,
) -> Result<Option<serde_json::Value>, AppError> {
    match payload.command_key.as_str() {
        "runtime-stop-graceful" => Ok(Some(serde_json::json!({"shutdown": "requested"}))),
        "approval-approve" | "approval-deny" => Ok(Some(serde_json::json!({
            "approval_ref": target_ref(&payload.canonical_target),
            "decision": if payload.command_key == "approval-approve" { "approve" } else { "deny" },
            "coordination": "security"
        }))),
        "bot-create" => create_bot(state, payload),
        "bot-activate" | "bot-deactivate" | "bot-archive" | "bot-restore" => {
            mutate_bot(state, payload)
        }
        "conversation-send" => send_conversation(state, payload, identity),
        "thread-create" => create_thread(state, payload, identity),
        "thread-send" => send_thread(state, payload, identity),
        "thread-branch" => branch_thread(state, payload, identity),
        "task-submit" => submit_task(state, payload, identity, provider_binding),
        "task-cancel" | "task-suspend" | "task-resume" | "task-redirect" => {
            control_task(state, payload, provider_binding)
        }
        "memory-propose" => propose_memory(state, payload, identity),
        "memory-promote" => promote_memory(state, payload),
        "project-create" => create_project(state, payload),
        "project-archive" | "project-restore" => mutate_project(state, payload),
        "project-member-set" | "project-member-remove" => mutate_membership(state, payload),
        "channel-create" => create_channel(state, payload),
        "channel-member-set" | "channel-member-remove" => mutate_membership(state, payload),
        "channel-send" => send_channel(state, payload, identity),
        "operation-reconcile" => reconcile_operation(state, payload),
        "side-effect-reconcile" => reconcile_side_effect(state, payload),
        other if target_exists(state, &payload.canonical_target) => Err(AppError::Conflict(
            format!("no Application mutation owner for command {other}"),
        )),
        other => Err(AppError::NotFound(format!(
            "target for unsupported Application command {other} does not exist"
        ))),
    }
}

fn create_bot(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    let name = required_field(payload, "name")?;
    if state.bots.values().any(|bot| bot.name == name) {
        return Err(AppError::Conflict(format!(
            "bot name already exists: {name}"
        )));
    }
    let id = BotId(name.to_owned());
    let conversation_id = ConversationId(format!("{}:main", id.0));
    let policy_bindings = BotPolicyBindings {
        brain_policy: optional_field(payload, "brain_policy").map(str::to_owned),
        permission_policy: optional_field(payload, "permission_policy").map(str::to_owned),
        resource_policy: optional_field(payload, "resource_policy").map(str::to_owned),
        provider_policy: optional_field(payload, "provider_policy").map(str::to_owned),
    };
    state.bots.insert(
        id.clone(),
        BotState {
            id: id.clone(),
            name: name.to_owned(),
            revision: 1,
            lifecycle: LifecycleState::Inactive,
            policy_bindings: policy_bindings.clone(),
        },
    );
    state.conversations.insert(
        conversation_id.clone(),
        ConversationState {
            id: conversation_id.clone(),
            owner: ConversationOwner::Bot { bot_id: id.clone() },
            revision: 1,
            messages: Vec::new(),
        },
    );
    Ok(Some(serde_json::json!({
        "bot_ref": format!("bot:{}", id.0),
        "bot_revision": 1,
        "lifecycle": "inactive",
        "main_conversation_ref": format!("conversation:{}", conversation_id.0),
        "policy_bindings": policy_bindings_value(&policy_bindings),
    })))
}

fn mutate_bot(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Bot { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Bot"));
    };
    let bot = state
        .bots
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound(format!("bot {} does not exist", id.0)))?;
    bot.lifecycle = match payload.command_key.as_str() {
        "bot-activate" => LifecycleState::Active,
        "bot-deactivate" | "bot-restore" => LifecycleState::Inactive,
        "bot-archive" => LifecycleState::Terminated,
        _ => {
            return Err(AppError::Internal(
                "invalid bot lifecycle command".to_owned(),
            ));
        }
    };
    bot.revision = next_revision(bot.revision, "bot")?;
    Ok(Some(serde_json::json!({
        "bot_ref": format!("bot:{}", bot.id.0),
        "bot_revision": bot.revision,
        "lifecycle": lifecycle_name(bot.lifecycle),
    })))
}

fn send_conversation(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Conversation { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Conversation"));
    };
    let content = required_materialized_content(payload)?;
    let message_id = MessageId(format!("message:{}", identity.operation_id.0));
    let (sequence, revision) = {
        let conversation = state
            .conversations
            .get_mut(id)
            .ok_or_else(|| AppError::NotFound(format!("conversation {} does not exist", id.0)))?;
        let sequence = i64::try_from(conversation.messages.len())
            .map_err(|_| AppError::Internal("conversation message count exceeds i64".to_owned()))?
            .checked_add(1)
            .ok_or_else(|| AppError::Internal("conversation sequence exhausted".to_owned()))?;
        conversation.messages.push(message_id.clone());
        conversation.revision = next_revision(conversation.revision, "conversation")?;
        (sequence, conversation.revision)
    };
    state.messages.insert(
        message_id.clone(),
        MessageState {
            id: message_id.clone(),
            content,
            sequence,
        },
    );
    Ok(Some(serde_json::json!({
        "conversation_ref": format!("conversation:{}", id.0),
        "conversation_revision": revision,
        "message_ref": format!("message:{}", message_id.0),
    })))
}

fn create_thread(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Conversation {
        id: conversation_id,
        ..
    } = &payload.canonical_target
    else {
        return Err(target_error(payload, "Conversation"));
    };
    if !state.conversations.contains_key(conversation_id) {
        return Err(AppError::NotFound(format!(
            "conversation {} does not exist",
            conversation_id.0
        )));
    }
    let id = ThreadId(format!("thread:{}", identity.operation_id.0));
    let title = optional_field(payload, "title")
        .unwrap_or(id.0.as_str())
        .to_owned();
    state.threads.insert(
        id.clone(),
        ThreadState {
            id: id.clone(),
            conversation_id: conversation_id.clone(),
            revision: 1,
            parent_message_id: None,
            title,
            messages: Vec::new(),
        },
    );
    Ok(Some(serde_json::json!({
        "thread_ref": format!("thread:{}", id.0),
        "thread_revision": 1,
        "conversation_ref": format!("conversation:{}", conversation_id.0)
    })))
}

fn send_thread(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Thread { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Thread"));
    };
    let content = required_materialized_content(payload)?;
    let message_id = MessageId(format!("message:{}", identity.operation_id.0));
    let (sequence, revision) = {
        let thread = state
            .threads
            .get_mut(id)
            .ok_or_else(|| AppError::NotFound(format!("thread {} does not exist", id.0)))?;
        let sequence = i64::try_from(thread.messages.len())
            .map_err(|_| AppError::Internal("thread message count exceeds i64".to_owned()))?
            .checked_add(1)
            .ok_or_else(|| AppError::Internal("thread sequence exhausted".to_owned()))?;
        thread.messages.push(message_id.clone());
        thread.revision = next_revision(thread.revision, "thread")?;
        (sequence, thread.revision)
    };
    state.messages.insert(
        message_id.clone(),
        MessageState {
            id: message_id.clone(),
            content,
            sequence,
        },
    );
    Ok(Some(serde_json::json!({
        "thread_ref": format!("thread:{}", id.0),
        "thread_revision": revision,
        "message_ref": format!("message:{}", message_id.0)
    })))
}

fn branch_thread(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Thread { id: source_id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Thread"));
    };
    let source = state
        .threads
        .get(source_id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("thread {} does not exist", source_id.0)))?;
    let message_id = MessageId(required_field(payload, "message_id")?.to_owned());
    if !source.messages.contains(&message_id) {
        return Err(AppError::NotFound(format!(
            "message {} is not in source thread {}",
            message_id.0, source_id.0
        )));
    }
    let id = ThreadId(format!("thread:{}", identity.operation_id.0));
    state.threads.insert(
        id.clone(),
        ThreadState {
            id: id.clone(),
            conversation_id: source.conversation_id,
            revision: 1,
            parent_message_id: Some(message_id.clone()),
            title: optional_field(payload, "title")
                .unwrap_or(id.0.as_str())
                .to_owned(),
            messages: Vec::new(),
        },
    );
    Ok(Some(serde_json::json!({
        "thread_ref": format!("thread:{}", id.0),
        "source_thread_ref": format!("thread:{}", source_id.0),
        "parent_message_ref": format!("message:{}", message_id.0)
    })))
}

#[derive(Debug, Clone)]
struct ResolvedDelegation {
    from_bot: BotId,
    to_bot: BotId,
    role: String,
}

fn resolve_delegation(
    state: &DomainState,
    scope: &ScopeSelector,
    constraints: &TaskExecutionConstraints,
) -> Result<Option<ResolvedDelegation>, AppError> {
    let Some(recipient_ref) = constraints.delegate_to_bot.as_deref() else {
        return Ok(None);
    };
    if recipient_ref.len() > 256 {
        return Err(AppError::ResourceExhausted(
            "delegation recipient reference exceeds 256 bytes".to_owned(),
        ));
    }
    if matches!(scope, ScopeSelector::Bot(_)) {
        return Err(AppError::Conflict(
            "cross-Bot delegation requires a shared Project or Channel scope".to_owned(),
        ));
    }
    let sender_ref = constraints.requested_sender_bot.as_deref().ok_or_else(|| {
        AppError::PermissionDenied(
            "operator-issued delegation requires server-authorized requested_sender_bot".to_owned(),
        )
    })?;
    if sender_ref.len() > 256 {
        return Err(AppError::ResourceExhausted(
            "delegation sender reference exceeds 256 bytes".to_owned(),
        ));
    }
    let from_bot = resolve_bot_id(state, sender_ref)?;
    let to_bot = resolve_bot_id(state, recipient_ref)?;
    if from_bot == to_bot {
        return Err(AppError::Conflict(
            "delegation sender and recipient must be distinct".to_owned(),
        ));
    }
    let sender = state
        .memberships
        .get(&membership_key(
            scope,
            &BotSelector::CanonicalId(from_bot.clone()),
        ))
        .filter(|membership| membership.active)
        .ok_or_else(|| {
            AppError::PermissionDenied("delegation sender is not an active scope member".to_owned())
        })?;
    if !MembershipManager::can_delegate(&sender.role) {
        return Err(AppError::PermissionDenied(format!(
            "membership role {} cannot delegate",
            sender.role
        )));
    }
    state
        .memberships
        .get(&membership_key(
            scope,
            &BotSelector::CanonicalId(to_bot.clone()),
        ))
        .filter(|membership| membership.active)
        .ok_or_else(|| {
            AppError::PermissionDenied(
                "delegation recipient is not an active member of the same scope".to_owned(),
            )
        })?;
    Ok(Some(ResolvedDelegation {
        from_bot,
        to_bot,
        role: sender.role.clone(),
    }))
}

fn resolve_bot_id(state: &DomainState, reference: &str) -> Result<BotId, AppError> {
    let candidate = strip_prefix(reference, "bot:");
    let direct = BotId(candidate.clone());
    if state.bots.contains_key(&direct) {
        return Ok(direct);
    }
    state
        .bots
        .values()
        .find(|bot| bot.name == candidate)
        .map(|bot| bot.id.clone())
        .ok_or_else(|| AppError::NotFound(format!("unknown Bot reference: {reference}")))
}

fn submit_task(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
    provider_binding: Option<&ProviderBinding>,
) -> Result<Option<serde_json::Value>, AppError> {
    let scope = scope_from_target(&payload.canonical_target)?;
    ensure_scope_exists(state, &scope)?;
    validate_optional_scope_revision(
        state,
        &scope,
        payload.cas.as_ref().and_then(|cas| cas.if_scope_revision),
    )?;
    let constraints = TaskExecutionConstraints {
        delegate_to_bot: optional_field(payload, "delegate_to_bot").map(str::to_owned),
        requested_sender_bot: optional_field(payload, "requested_sender_bot").map(str::to_owned),
        deadline: optional_field(payload, "deadline").map(str::to_owned),
        budget: optional_field(payload, "budget").map(str::to_owned),
    };
    for (field, reference) in [
        ("delegate_to_bot", constraints.delegate_to_bot.as_deref()),
        (
            "requested_sender_bot",
            constraints.requested_sender_bot.as_deref(),
        ),
    ] {
        let Some(reference) = reference else {
            continue;
        };
        let bot_id = BotId(strip_prefix(reference, "bot:"));
        if !state.bots.contains_key(&bot_id) && !state.bots.values().any(|bot| bot.name == bot_id.0)
        {
            return Err(AppError::NotFound(format!(
                "{field} references unknown bot: {reference}"
            )));
        }
    }
    let delegation = resolve_delegation(state, &scope, &constraints)?;
    if let Some(binding) = provider_binding {
        if binding.provider_id.0.trim().is_empty()
            || binding.capability.trim().is_empty()
            || binding.generation <= 0
        {
            return Err(AppError::Conflict(
                "task admission requires a valid Provider binding".to_owned(),
            ));
        }
        let nonterminal = state
            .executions
            .values()
            .filter(|execution| {
                matches!(
                    execution.status,
                    ExecutionStatus::Admitted | ExecutionStatus::Running
                )
            })
            .count();
        if nonterminal >= MAX_NONTERMINAL_EXECUTIONS {
            return Err(AppError::ResourceExhausted(format!(
                "Execution admission capacity {MAX_NONTERMINAL_EXECUTIONS} is exhausted"
            )));
        }
    }

    let id = TaskId(format!("task:{}", identity.operation_id.0));
    let process_id = ProcessId(format!("process:{}", identity.operation_id.0));
    let execution_id =
        provider_binding.map(|_| ExecutionId(format!("execution:{}:1", identity.operation_id.0)));
    let task_ref = format!("task:{}", id.0);
    let process_ref = format!("process:{}", process_id.0);
    let execution_ref = execution_id
        .as_ref()
        .map(|id| format!("execution:{}", id.0));
    let scope_ref = scope_owner(&scope);
    let owner = delegation
        .as_ref()
        .map(|delegation| format!("bot:{}", delegation.to_bot.0))
        .unwrap_or_else(|| scope_ref.clone());
    let context_bot = delegation
        .as_ref()
        .map(|delegation| delegation.to_bot.clone());
    let context_plan = match (execution_id.as_ref(), provider_binding) {
        (Some(execution_id), Some(binding)) => Some(build_context_plan(
            state,
            ContextPlanBuild {
                task_id: &id,
                execution_id,
                scope_ref: &scope_ref,
                context_bot: context_bot.as_ref(),
                task_revision: 1,
                constraints: &constraints,
                provider_binding: binding,
            },
        )?),
        _ => None,
    };

    state.tasks.insert(
        id.clone(),
        TaskState {
            id: id.clone(),
            owner: owner.clone(),
            revision: 1,
            execution_generation: 1,
            status: if provider_binding.is_some() {
                TaskStatus::Admitted
            } else {
                TaskStatus::Pending
            },
            intent: payload.content.clone(),
            result: None,
            constraints: constraints.clone(),
            control_history: Vec::new(),
            process_ref: Some(process_ref.clone()),
            execution_refs: execution_id.iter().cloned().collect(),
        },
    );
    if let (Some(execution_id), Some(context_plan)) = (execution_id.as_ref(), context_plan) {
        state.executions.insert(
            execution_id.clone(),
            ExecutionState {
                id: execution_id.clone(),
                task_id: id.clone(),
                generation: 1,
                attempt: 1,
                status: ExecutionStatus::Admitted,
                context_plan,
                scheduler_generation: None,
                result: None,
                evidence: Vec::new(),
                terminal_reason: None,
            },
        );
    }
    let mut child_refs = vec![task_ref.clone()];
    if let Some(execution_ref) = &execution_ref {
        child_refs.push(execution_ref.clone());
    }
    if let Some(delegation) = &delegation {
        state.delegations.push(DelegationRecord {
            id: format!("delegation:{}", identity.operation_id.0),
            task_id: id.clone(),
            from_bot: BotSelector::CanonicalId(delegation.from_bot.clone()),
            to_bot: BotSelector::CanonicalId(delegation.to_bot.clone()),
            scope: scope.clone(),
            role: delegation.role.clone(),
            status: DelegationStatus::Accepted,
            created_at: now_secs().saturating_mul(1_000),
            resolved_at: Some(now_secs().saturating_mul(1_000)),
        });
    }
    state.processes.insert(
        process_id.clone(),
        ProcessState {
            id: process_id.clone(),
            definition_id: "task-execution".to_owned(),
            definition_version: "1".to_owned(),
            revision: 1,
            scope_ref: scope_ref.clone(),
            initiator_ref: delegation
                .as_ref()
                .map(|delegation| format!("bot:{}", delegation.from_bot.0))
                .unwrap_or_else(|| payload.principal_ref.0.clone()),
            lifecycle: ProcessLifecycle::Running,
            current_step_ref: Some(if execution_ref.is_some() {
                "provider-execution".to_owned()
            } else {
                task_ref.clone()
            }),
            current_activity_ref: execution_ref.clone(),
            waiting_condition_ref: None,
            continuation_ref: execution_ref
                .as_ref()
                .map(|reference| format!("continue:{reference}")),
            child_refs,
            outcome_refs: Vec::new(),
            progress: 0,
            terminal_reason: None,
        },
    );
    if let (Some(execution_id), Some(binding)) = (execution_id.as_ref(), provider_binding) {
        record_execution_audit(
            state,
            AuditIntentInput {
                execution_id,
                execution_generation: 1,
                task_id: &id,
                process_id: &process_id,
                provider_id: &binding.provider_id,
                provider_generation: binding.generation,
                phase: ExecutionAuditPhase::Admitted,
                detail: "Task, Execution, Context Plan, and Process admitted atomically",
            },
        )?;
    }
    Ok(Some(serde_json::json!({
        "task_ref": task_ref,
        "task_revision": 1,
        "execution_ref": execution_ref,
        "execution_generation": 1,
        "context_plan_ref": execution_id.as_ref().map(|id| format!("context-plan:{}", id.0)),
        "state": if provider_binding.is_some() { "admitted" } else { "pending" },
        "process_ref": process_ref,
        "process_revision": 1,
        "delegation_ref": delegation.as_ref().map(|_| format!("delegation:{}", identity.operation_id.0)),
        "sender_bot_ref": delegation.as_ref().map(|value| format!("bot:{}", value.from_bot.0)),
        "recipient_bot_ref": delegation.as_ref().map(|value| format!("bot:{}", value.to_bot.0)),
        "constraints": constraints_value(&constraints),
    })))
}

struct ContextPlanBuild<'a> {
    task_id: &'a TaskId,
    execution_id: &'a ExecutionId,
    scope_ref: &'a str,
    context_bot: Option<&'a BotId>,
    task_revision: i64,
    constraints: &'a TaskExecutionConstraints,
    provider_binding: &'a ProviderBinding,
}

fn build_context_plan(
    state: &DomainState,
    input: ContextPlanBuild<'_>,
) -> Result<ContextPlan, AppError> {
    let ContextPlanBuild {
        task_id,
        execution_id,
        scope_ref,
        context_bot,
        task_revision,
        constraints,
        provider_binding,
    } = input;
    const MAX_MEMORY_ITEMS: usize = 32;
    const MAX_CONTEXT_BYTES: usize = 64 * 1024;
    const MAX_CONVERSATIONS: usize = 8;

    let bot_id = context_bot.cloned().or_else(|| {
        scope_ref
            .strip_prefix("bot:")
            .map(|value| BotId(value.to_owned()))
    });
    let bot = bot_id.as_ref().and_then(|id| state.bots.get(id));
    let mut conversation_refs = bot_id
        .as_ref()
        .map(|id| {
            state
                .conversations
                .values()
                .filter(|conversation| {
                    matches!(&conversation.owner, ConversationOwner::Bot { bot_id } if bot_id == id)
                })
                .map(|conversation| format!("conversation:{}", conversation.id.0))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    conversation_refs.sort();
    conversation_refs.truncate(MAX_CONVERSATIONS);

    let mut memories = state
        .memories
        .values()
        .filter(|memory| {
            memory.scope_key == scope_ref && memory.status == MemoryAssertionStatus::Accepted
        })
        .collect::<Vec<_>>();
    memories.sort_by(|left, right| left.id.0.cmp(&right.id.0));
    let mut memory_refs = Vec::new();
    let mut bounded_context = String::new();
    for memory in memories.into_iter().take(MAX_MEMORY_ITEMS) {
        let ContentSource::Text { value } = &memory.statement else {
            continue;
        };
        let line = format!("[memory:{}@{}] {}\n", memory.id.0, memory.revision, value);
        if bounded_context.len().saturating_add(line.len()) > MAX_CONTEXT_BYTES {
            break;
        }
        bounded_context.push_str(&line);
        memory_refs.push(ContextMemoryRef {
            memory_id: memory.id.clone(),
            revision: memory.revision,
        });
    }
    let permission_refs = bot
        .and_then(|bot| bot.policy_bindings.permission_policy.clone())
        .into_iter()
        .collect();

    let max_output_tokens = parse_budget(constraints.budget.as_deref())?;
    let deadline_unix_seconds = parse_deadline(constraints.deadline.as_deref())?;
    Ok(ContextPlan {
        plan_ref: format!("context-plan:{}", execution_id.0),
        schema_version: "context-plan-v1".to_owned(),
        task_id: task_id.clone(),
        task_revision,
        scope_ref: scope_ref.to_owned(),
        bot_ref: bot_id.map(|id| format!("bot:{}", id.0)),
        identity_revision: bot.map(|bot| bot.revision),
        conversation_refs,
        memory_refs,
        capability_requirements: vec![provider_binding.capability.clone()],
        provider_binding: provider_binding.clone(),
        permission_refs,
        resource_budget: constraints.budget.clone(),
        max_output_tokens,
        deadline: constraints.deadline.clone(),
        deadline_unix_seconds,
        bounded_context,
    })
}

fn parse_budget(value: Option<&str>) -> Result<Option<u32>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let parsed = value.parse::<u64>().map_err(|_| {
        AppError::Conflict(
            "task budget must be a positive numeric token ceiling until BudgetRef policy resolution is implemented"
                .to_owned(),
        )
    })?;
    let parsed = u32::try_from(parsed).map_err(|_| {
        AppError::Conflict("task budget exceeds the Provider token ceiling".to_owned())
    })?;
    if parsed == 0 {
        return Err(AppError::Conflict(
            "task budget must be greater than zero".to_owned(),
        ));
    }
    Ok(Some(parsed))
}

fn parse_deadline(value: Option<&str>) -> Result<Option<i64>, AppError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if let Ok(epoch) = value.parse::<i64>() {
        if epoch <= 0 {
            return Err(AppError::Conflict(
                "task deadline epoch must be positive".to_owned(),
            ));
        }
        return Ok(Some(epoch));
    }
    parse_rfc3339_utc(value).map(Some).ok_or_else(|| {
        AppError::Conflict(
            "task deadline must be Unix epoch seconds or RFC3339 UTC (YYYY-MM-DDTHH:MM:SSZ)"
                .to_owned(),
        )
    })
}

fn parse_rfc3339_utc(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let number = |start: usize, end: usize| -> Option<i64> {
        std::str::from_utf8(&bytes[start..end]).ok()?.parse().ok()
    };
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;
    if !(1..=12).contains(&month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let month_index = usize::try_from(month.checked_sub(1)?).ok()?;
    if day < 1 || day > i64::from(month_days[month_index]) {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    days.checked_mul(86_400)?
        .checked_add(hour.checked_mul(3_600)?)?
        .checked_add(minute.checked_mul(60)?)?
        .checked_add(second)
}

fn control_task(
    state: &mut DomainState,
    payload: &CommandPayload,
    provider_binding: Option<&ProviderBinding>,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Task {
        id,
        execution_generation,
        ..
    } = &payload.canonical_target
    else {
        return Err(target_error(payload, "Task"));
    };
    let replacement = if payload.command_key == "task-redirect" {
        Some(required_materialized_content(payload)?)
    } else {
        None
    };
    let reason = optional_field(payload, "reason").map(str::to_owned);
    let resume_attempt = if payload.command_key == "task-resume" {
        let current = state
            .tasks
            .get(id)
            .ok_or_else(|| AppError::NotFound(format!("task {} does not exist", id.0)))?;
        validate_task_control_transition(current.status, payload.command_key.as_str())?;
        match provider_binding {
            Some(binding) => {
                if binding.provider_id.0.trim().is_empty()
                    || binding.capability.trim().is_empty()
                    || binding.generation <= 0
                {
                    return Err(AppError::Conflict(
                        "task resume requires a valid fresh Provider binding".to_owned(),
                    ));
                }
                let nonterminal = state
                    .executions
                    .values()
                    .filter(|execution| {
                        matches!(
                            execution.status,
                            ExecutionStatus::Admitted | ExecutionStatus::Running
                        )
                    })
                    .count();
                if nonterminal >= MAX_NONTERMINAL_EXECUTIONS {
                    return Err(AppError::ResourceExhausted(format!(
                        "Execution admission capacity {MAX_NONTERMINAL_EXECUTIONS} is exhausted"
                    )));
                }
                let generation =
                    next_revision(current.execution_generation, "execution generation")?;
                let task_revision = next_revision(current.revision, "task")?;
                let execution_id = ExecutionId(format!("execution:{}:{generation}", id.0));
                let resume_scope = current
                    .process_ref
                    .as_deref()
                    .and_then(|reference| {
                        state
                            .processes
                            .get(&ProcessId(strip_prefix(reference, "process:")))
                    })
                    .map(|process| process.scope_ref.clone())
                    .unwrap_or_else(|| current.owner.clone());
                let resume_bot = current
                    .owner
                    .strip_prefix("bot:")
                    .map(|value| BotId(value.to_owned()));
                let context_plan = build_context_plan(
                    state,
                    ContextPlanBuild {
                        task_id: id,
                        execution_id: &execution_id,
                        scope_ref: &resume_scope,
                        context_bot: resume_bot.as_ref(),
                        task_revision,
                        constraints: &current.constraints,
                        provider_binding: binding,
                    },
                )?;
                Some((execution_id, generation, task_revision, context_plan))
            }
            None if current.execution_refs.is_empty() => None,
            None => {
                return Err(AppError::Conflict(
                    "task resume requires a fresh Ready Provider binding and immutable Execution attempt"
                        .to_owned(),
                ));
            }
        }
    } else {
        None
    };
    let task = state
        .tasks
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound(format!("task {} does not exist", id.0)))?;
    if matches!(payload.command_key.as_str(), "task-cancel" | "task-suspend") {
        let expected = execution_generation
            .or_else(|| {
                payload
                    .cas
                    .as_ref()
                    .and_then(|cas| cas.if_execution_generation)
            })
            .ok_or_else(|| {
                AppError::Conflict("task control requires execution generation".to_owned())
            })?;
        if task.execution_generation != expected {
            return Err(AppError::Conflict(format!(
                "task execution generation mismatch: expected {expected}, current {}",
                task.execution_generation
            )));
        }
    }
    validate_task_control_transition(task.status, payload.command_key.as_str())?;
    match payload.command_key.as_str() {
        "task-cancel" => task.status = TaskStatus::Cancelled,
        "task-suspend" => task.status = TaskStatus::Suspended,
        "task-resume" => {
            if let Some((_, generation, _, _)) = &resume_attempt {
                task.status = TaskStatus::Admitted;
                task.execution_generation = *generation;
            } else {
                task.status = TaskStatus::Running;
                task.execution_generation =
                    next_revision(task.execution_generation, "execution generation")?;
            }
        }
        "task-redirect" => {
            task.intent = replacement;
            task.status = TaskStatus::Pending;
        }
        _ => {
            return Err(AppError::Internal(
                "invalid task control command".to_owned(),
            ));
        }
    }
    task.revision = next_revision(task.revision, "task")?;
    // Durably record the supervision directive (including its reason) as an
    // append-only audit entry. The directive references the task revision it
    // took effect at; it never copies task lifecycle into a second owner.
    task.control_history.push(TaskControlDirective {
        command_key: payload.command_key.clone(),
        applied_revision: task.revision,
        execution_generation: task.execution_generation,
        reason: reason.clone(),
    });
    if let Some((execution_id, _, _, _)) = &resume_attempt {
        task.execution_refs.push(execution_id.clone());
    }
    let task_status = task.status;
    let task_revision = task.revision;
    let process_ref = task.process_ref.clone();
    let execution_refs = task.execution_refs.clone();

    // Transition the linked orchestration Process by reference. Legacy tasks
    // have no process_ref and remain controllable; a present but unresolved ref
    // is corruption and rolls the whole Task+Process mutation back.
    if let Some(process_ref) = process_ref.as_deref() {
        advance_process_for_task_control(
            state,
            process_ref,
            payload.command_key.as_str(),
            reason.as_deref(),
        )?;
    }
    let execution_ref =
        if let Some((execution_id, generation, planned_task_revision, context_plan)) =
            resume_attempt
        {
            let reference = format!("execution:{}", execution_id.0);
            state.executions.insert(
                execution_id.clone(),
                ExecutionState {
                    id: execution_id.clone(),
                    task_id: id.clone(),
                    generation,
                    attempt: generation,
                    status: ExecutionStatus::Admitted,
                    context_plan,
                    scheduler_generation: None,
                    result: None,
                    evidence: Vec::new(),
                    terminal_reason: None,
                },
            );
            if planned_task_revision != task_revision {
                return Err(AppError::Internal(
                    "resume Context Plan task revision drifted during atomic mutation".to_owned(),
                ));
            }
            let process_id = process_ref
                .as_deref()
                .map(|process_ref| ProcessId(strip_prefix(process_ref, "process:")))
                .ok_or_else(|| {
                    AppError::Internal(
                        "fresh Execution attempt requires an orchestration Process".to_owned(),
                    )
                })?;
            let process = state.processes.get_mut(&process_id).ok_or_else(|| {
                AppError::Internal(format!(
                    "task references missing orchestration process: {}",
                    process_ref.as_deref().unwrap_or_default()
                ))
            })?;
            process.current_step_ref = Some("provider-execution".to_owned());
            process.current_activity_ref = Some(reference.clone());
            process.continuation_ref = Some(format!("continue:{reference}"));
            process.child_refs.push(reference.clone());
            let binding = provider_binding.ok_or_else(|| {
                AppError::Internal("fresh Execution attempt lost Provider binding".to_owned())
            })?;
            record_execution_audit(
                state,
                AuditIntentInput {
                    execution_id: &execution_id,
                    execution_generation: generation,
                    task_id: id,
                    process_id: &process_id,
                    provider_id: &binding.provider_id,
                    provider_generation: binding.generation,
                    phase: ExecutionAuditPhase::Admitted,
                    detail: "fresh resume Execution and Context Plan admitted atomically",
                },
            )?;
            Some(reference)
        } else {
            None
        };
    if matches!(payload.command_key.as_str(), "task-cancel" | "task-suspend") {
        let mut cancelled = Vec::new();
        for execution_id in execution_refs {
            if let Some(execution) = state.executions.get_mut(&execution_id) {
                if execution.status == ExecutionStatus::Admitted {
                    execution.status = ExecutionStatus::Cancelled;
                    execution.terminal_reason =
                        Some(format!("{} before Provider activity", payload.command_key));
                    cancelled.push((
                        execution.id.clone(),
                        execution.generation,
                        execution.context_plan.provider_binding.provider_id.clone(),
                        execution.context_plan.provider_binding.generation,
                    ));
                }
            }
        }
        if !cancelled.is_empty() {
            let process_id = process_ref
                .as_deref()
                .map(|reference| ProcessId(strip_prefix(reference, "process:")))
                .ok_or_else(|| {
                    AppError::Internal(
                        "cancelled Execution requires an orchestration Process".to_owned(),
                    )
                })?;
            for (execution_id, generation, provider_id, provider_generation) in cancelled {
                record_execution_audit(
                    state,
                    AuditIntentInput {
                        execution_id: &execution_id,
                        execution_generation: generation,
                        task_id: id,
                        process_id: &process_id,
                        provider_id: &provider_id,
                        provider_generation,
                        phase: ExecutionAuditPhase::Cancelled,
                        detail: "Execution cancelled before Provider activity",
                    },
                )?;
            }
        }
    }
    Ok(Some(serde_json::json!({
        "task_ref": format!("task:{}", id.0),
        "task_revision": task_revision,
        "execution_generation": execution_generation_of(state, id),
        "execution_ref": execution_ref.clone(),
        "context_plan_ref": execution_ref
            .as_ref()
            .map(|reference| format!("context-plan:{}", strip_prefix(reference, "execution:"))),
        "state": task_status_name(task_status),
        "process_ref": process_ref,
    })))
}

fn propose_memory(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
) -> Result<Option<serde_json::Value>, AppError> {
    let scope = scope_from_target(&payload.canonical_target)?;
    ensure_scope_exists(state, &scope)?;
    let statement = required_materialized_content(payload)?;
    let evidence = string_list(payload, "evidence");
    let id = MemoryId(format!("memory:{}", identity.operation_id.0));
    state.memories.insert(
        id.clone(),
        MemoryState {
            id: id.clone(),
            scope_key: scope_owner(&scope),
            revision: 1,
            status: MemoryAssertionStatus::Proposed,
            statement: statement.clone(),
            evidence: evidence.clone(),
            history: vec![MemoryRevisionState {
                revision: 1,
                status: MemoryAssertionStatus::Proposed,
                statement,
                evidence,
            }],
            declassifications: Vec::new(),
        },
    );
    Ok(Some(serde_json::json!({
        "memory_ref": format!("memory:{}", id.0),
        "revision": 1,
        "state": "proposed"
    })))
}

fn promote_memory(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Memory { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Memory"));
    };
    let target_scope = parse_scope(required_field(payload, "target_scope")?);
    ensure_scope_exists(state, &target_scope)?;
    validate_optional_scope_revision(
        state,
        &target_scope,
        payload
            .cas
            .as_ref()
            .and_then(|cas| cas.if_target_scope_revision),
    )?;
    let evidence = string_list(payload, "evidence");
    if evidence.is_empty() {
        return Err(AppError::Conflict(
            "memory promotion requires evidence".to_owned(),
        ));
    }
    let memory = state
        .memories
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound(format!("memory {} does not exist", id.0)))?;
    if memory.status != MemoryAssertionStatus::Proposed {
        return Err(AppError::Conflict(
            "only Proposed memory can be promoted".to_owned(),
        ));
    }
    memory.revision = next_revision(memory.revision, "memory")?;
    memory.status = MemoryAssertionStatus::Accepted;
    memory.scope_key = scope_owner(&target_scope);
    memory.evidence.extend(evidence);
    if let Some(declassification_ref) = optional_field(payload, "declassification_ref") {
        // Persist declassification provenance. The Application does not own the
        // information-label policy; it durably records the reference and the
        // revision it was applied at so the field is auditable, not dropped.
        memory.declassifications.push(DeclassificationRecord {
            declassification_ref: declassification_ref.to_owned(),
            applied_revision: memory.revision,
        });
    }
    memory.history.push(MemoryRevisionState {
        revision: memory.revision,
        status: memory.status,
        statement: memory.statement.clone(),
        evidence: memory.evidence.clone(),
    });
    let declassification_refs: Vec<&str> = memory
        .declassifications
        .iter()
        .map(|record| record.declassification_ref.as_str())
        .collect();
    Ok(Some(serde_json::json!({
        "memory_ref": format!("memory:{}", id.0),
        "revision": memory.revision,
        "state": "accepted",
        "scope": memory.scope_key,
        "declassification_refs": declassification_refs,
    })))
}

fn create_project(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    let name = required_field(payload, "name")?;
    if state.projects.values().any(|project| project.name == name) {
        return Err(AppError::Conflict(format!(
            "project name already exists: {name}"
        )));
    }
    let owner_bot = BotId(strip_prefix(required_field(payload, "owner_bot")?, "bot:"));
    if !state.bots.contains_key(&owner_bot) {
        return Err(AppError::NotFound(format!(
            "owner bot {} does not exist",
            owner_bot.0
        )));
    }
    let id = ProjectId(name.to_owned());
    state.projects.insert(
        id.clone(),
        ProjectState {
            id: id.clone(),
            name: name.to_owned(),
            owner_bot: owner_bot.clone(),
            revision: 1,
            lifecycle: ProjectLifecycle::Active,
        },
    );
    let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(id.clone()));
    let member = BotSelector::CanonicalId(owner_bot.clone());
    let key = membership_key(&scope, &member);
    state.memberships.insert(
        key.clone(),
        MembershipRecord {
            id: key,
            scope,
            member_bot: member,
            role: "owner".to_owned(),
            generation: 1,
            active: true,
            created_at: now_secs(),
        },
    );
    Ok(Some(serde_json::json!({
        "project_ref": format!("project:{}", id.0),
        "project_revision": 1,
        "owner_bot_ref": format!("bot:{}", owner_bot.0)
    })))
}

fn mutate_project(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Project { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Project"));
    };
    let project = state
        .projects
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound(format!("project {} does not exist", id.0)))?;
    project.lifecycle = if payload.command_key == "project-archive" {
        ProjectLifecycle::Archived
    } else {
        ProjectLifecycle::Active
    };
    project.revision = next_revision(project.revision, "project")?;
    Ok(Some(serde_json::json!({
        "project_ref": format!("project:{}", id.0),
        "project_revision": project.revision,
        "lifecycle": project_lifecycle_name(project.lifecycle)
    })))
}

fn mutate_membership(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    let CanonicalTarget::Membership { scope, member_bot } = &payload.canonical_target else {
        return Err(target_error(payload, "Membership"));
    };
    validate_scope_revision(state, scope, payload.cas.as_ref())?;
    let key = membership_key(scope, member_bot);
    let expected_generation = payload
        .cas
        .as_ref()
        .and_then(|cas| cas.if_membership_generation);
    match payload.command_key.as_str() {
        "project-member-set" | "channel-member-set" => {
            if let Some(expected) = expected_generation {
                let current = state.memberships.get(&key).map(|row| row.generation);
                if current != Some(expected) {
                    return Err(AppError::Conflict(format!(
                        "membership generation mismatch: expected {expected}, current {current:?}"
                    )));
                }
            }
            let generation = match state.memberships.get(&key) {
                Some(row) => next_revision(row.generation, "membership generation")?,
                None => 1,
            };
            let record = MembershipRecord {
                id: key.clone(),
                scope: scope.clone(),
                member_bot: member_bot.clone(),
                role: required_field(payload, "role_ref")?.to_owned(),
                generation,
                active: true,
                created_at: state
                    .memberships
                    .get(&key)
                    .map_or_else(now_secs, |row| row.created_at),
            };
            state.memberships.insert(key, record.clone());
            bump_scope_revision(state, scope)?;
            Ok(Some(serde_json::json!({
                "membership_ref": record.id,
                "generation": record.generation,
                "role": record.role
            })))
        }
        "project-member-remove" | "channel-member-remove" => {
            let expected = expected_generation.ok_or_else(|| {
                AppError::Conflict("membership remove requires generation CAS".to_owned())
            })?;
            let current = state
                .memberships
                .get_mut(&key)
                .ok_or_else(|| AppError::NotFound(format!("membership {key} does not exist")))?;
            if !current.active {
                return Err(AppError::NotFound(format!(
                    "membership {key} is not active"
                )));
            }
            if current.generation != expected {
                return Err(AppError::Conflict(format!(
                    "membership generation mismatch: expected {expected}, current {}",
                    current.generation
                )));
            }
            current.generation = next_revision(current.generation, "membership generation")?;
            current.active = false;
            let generation = current.generation;
            bump_scope_revision(state, scope)?;
            Ok(Some(serde_json::json!({
                "membership_ref": key,
                "generation": generation,
                "removed": true
            })))
        }
        _ => Err(AppError::Internal("invalid membership command".to_owned())),
    }
}

fn create_channel(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Project { id: project_id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Project"));
    };
    let name = required_field(payload, "name")?;
    if state
        .channels
        .values()
        .any(|channel| channel.project_id == *project_id && channel.name == name)
    {
        return Err(AppError::Conflict(format!(
            "channel name already exists in project {}: {name}",
            project_id.0
        )));
    }
    let id = ChannelId(format!("{}:{name}", project_id.0));
    let conversation_id = ConversationId(format!("{}:conversation", id.0));
    state.channels.insert(
        id.clone(),
        ChannelState {
            id: id.clone(),
            project_id: project_id.clone(),
            name: name.to_owned(),
            revision: 1,
            conversation_id: conversation_id.clone(),
        },
    );
    state.conversations.insert(
        conversation_id.clone(),
        ConversationState {
            id: conversation_id.clone(),
            owner: ConversationOwner::Channel {
                project_id: project_id.clone(),
                channel_id: id.clone(),
            },
            revision: 1,
            messages: Vec::new(),
        },
    );
    if let Some(project) = state.projects.get_mut(project_id) {
        project.revision = next_revision(project.revision, "project")?;
    }
    Ok(Some(serde_json::json!({
        "channel_ref": format!("channel:{}", id.0),
        "channel_revision": 1,
        "conversation_ref": format!("conversation:{}", conversation_id.0)
    })))
}

fn send_channel(
    state: &mut DomainState,
    payload: &CommandPayload,
    identity: &OperationIdentity,
) -> Result<Option<serde_json::Value>, AppError> {
    let CanonicalTarget::Channel { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Channel"));
    };
    let channel = state
        .channels
        .get(id)
        .cloned()
        .ok_or_else(|| AppError::NotFound(format!("channel {} does not exist", id.0)))?;
    let conversation_revision = state
        .conversations
        .get(&channel.conversation_id)
        .ok_or_else(|| AppError::Internal("channel has no conversation".to_owned()))?
        .revision;
    if payload
        .cas
        .as_ref()
        .and_then(|cas| cas.if_revision)
        .is_some_and(|want| want != conversation_revision)
    {
        return Err(AppError::Conflict(format!(
            "channel conversation revision mismatch: expected {:?}, current {conversation_revision}",
            payload.cas.as_ref().and_then(|cas| cas.if_revision)
        )));
    }
    let mut projected = payload.clone();
    projected.canonical_target = CanonicalTarget::Conversation {
        id: channel.conversation_id,
        revision: conversation_revision,
    };
    projected.cas = Some(CasConditions {
        if_revision: payload.cas.as_ref().and_then(|cas| cas.if_revision),
        ..empty_cas()
    });
    send_conversation(state, &projected, identity)
}

fn reconcile_operation(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::Operation { operation_id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "Operation"));
    };
    let result = state.results.get(operation_id).ok_or_else(|| {
        AppError::NotFound(format!("operation {} does not exist", operation_id.0))
    })?;
    Ok(Some(serde_json::json!({
        "reconciled_operation_ref": format!("operation:{}", operation_id.0),
        "status": result.status,
        "receipt_disposition": format!("{:?}", result.receipt.disposition)
    })))
}

fn reconcile_side_effect(
    state: &mut DomainState,
    payload: &CommandPayload,
) -> Result<Option<serde_json::Value>, AppError> {
    validate_target_against(state, &payload.canonical_target, &payload.cas)?;
    let CanonicalTarget::SideEffect { id, .. } = &payload.canonical_target else {
        return Err(target_error(payload, "SideEffect"));
    };
    let evidence = string_list(payload, "evidence");
    let row = state
        .side_effects
        .get_mut(id)
        .ok_or_else(|| AppError::NotFound(format!("side effect {id} does not exist")))?;
    row.status = match required_field(payload, "action")? {
        "confirm" => SideEffectStatus::Confirmed,
        "fail" => SideEffectStatus::Failed,
        "manual" => SideEffectStatus::ManualResolutionRequired,
        other => {
            return Err(AppError::Conflict(format!(
                "invalid reconcile action: {other}"
            )));
        }
    };
    row.revision = next_revision(row.revision, "side effect")?;
    row.evidence.extend(evidence);
    let operation_ref = row
        .operation_ref
        .as_ref()
        .map(|operation| format!("operation:{}", operation.0));
    Ok(Some(serde_json::json!({
        "side_effect_ref": id,
        "revision": row.revision,
        "status": side_effect_status_name(row.status),
        "operation_ref": operation_ref,
    })))
}

fn validate_target_against(
    state: &DomainState,
    target: &CanonicalTarget,
    cas: &Option<CasConditions>,
) -> Result<(), AppError> {
    match resolve_outcome(state, target, cas) {
        DomainOutcome::Conflict => Err(AppError::Conflict(format!(
            "target {target:?} revision does not match CAS conditions"
        ))),
        DomainOutcome::NotFound => Err(AppError::NotFound(
            "target does not exist but a precondition requires it".to_owned(),
        )),
        DomainOutcome::PermissionDenied => Err(AppError::PermissionDenied(
            "principal is not permitted to mutate the target".to_owned(),
        )),
        DomainOutcome::Created | DomainOutcome::Updated => Ok(()),
    }
}

fn scope_from_target(target: &CanonicalTarget) -> Result<ScopeSelector, AppError> {
    match target {
        CanonicalTarget::Bot { id, .. } => {
            Ok(ScopeSelector::Bot(BotSelector::CanonicalId(id.clone())))
        }
        CanonicalTarget::Project { id, .. } => Ok(ScopeSelector::Project(
            ProjectSelector::CanonicalId(id.clone()),
        )),
        CanonicalTarget::Channel { id, .. } => Ok(ScopeSelector::Channel(
            ChannelSelector::CanonicalId(id.clone()),
        )),
        _ => Err(AppError::Conflict(
            "command target is not a Bot/Project/Channel scope".to_owned(),
        )),
    }
}

fn parse_scope(value: &str) -> ScopeSelector {
    if let Some(id) = value.strip_prefix("project:") {
        ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(id.to_owned())))
    } else if let Some(id) = value.strip_prefix("channel:") {
        ScopeSelector::Channel(ChannelSelector::CanonicalId(ChannelId(id.to_owned())))
    } else if let Some(id) = value.strip_prefix("bot:") {
        ScopeSelector::Bot(BotSelector::CanonicalId(BotId(id.to_owned())))
    } else {
        ScopeSelector::Bot(BotSelector::CanonicalId(BotId(value.to_owned())))
    }
}

fn ensure_scope_exists(state: &DomainState, scope: &ScopeSelector) -> Result<(), AppError> {
    if state.scope_exists(scope) {
        Ok(())
    } else {
        Err(AppError::NotFound(format!(
            "scope does not exist: {scope:?}"
        )))
    }
}

fn validate_scope_revision(
    state: &DomainState,
    scope: &ScopeSelector,
    cas: Option<&CasConditions>,
) -> Result<(), AppError> {
    let expected = match scope {
        ScopeSelector::Project(_) => cas.and_then(|cas| cas.if_project_revision),
        ScopeSelector::Channel(_) => cas.and_then(|cas| cas.if_channel_revision),
        ScopeSelector::Bot(_) => cas.and_then(|cas| cas.if_scope_revision),
    };
    validate_optional_scope_revision(state, scope, expected)
}

fn validate_optional_scope_revision(
    state: &DomainState,
    scope: &ScopeSelector,
    expected: Option<i64>,
) -> Result<(), AppError> {
    let current = scope_revision(state, scope)?;
    if expected.is_some_and(|want| want != current) {
        Err(AppError::Conflict(format!(
            "scope revision mismatch: expected {expected:?}, current {current}"
        )))
    } else {
        Ok(())
    }
}

fn scope_revision(state: &DomainState, scope: &ScopeSelector) -> Result<i64, AppError> {
    match scope {
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => state
            .projects
            .get(id)
            .map(|row| row.revision)
            .ok_or_else(|| AppError::NotFound(format!("project {} does not exist", id.0))),
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => state
            .channels
            .get(id)
            .map(|row| row.revision)
            .ok_or_else(|| AppError::NotFound(format!("channel {} does not exist", id.0))),
        ScopeSelector::Bot(BotSelector::CanonicalId(id)) => state
            .bots
            .get(id)
            .map(|row| row.revision)
            .ok_or_else(|| AppError::NotFound(format!("bot {} does not exist", id.0))),
        _ => Err(AppError::Conflict(
            "scope CAS requires canonical scope".to_owned(),
        )),
    }
}

fn bump_scope_revision(state: &mut DomainState, scope: &ScopeSelector) -> Result<(), AppError> {
    match scope {
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => {
            let row = state
                .projects
                .get_mut(id)
                .ok_or_else(|| AppError::NotFound(format!("project {} does not exist", id.0)))?;
            row.revision = next_revision(row.revision, "project")?;
            Ok(())
        }
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => {
            let row = state
                .channels
                .get_mut(id)
                .ok_or_else(|| AppError::NotFound(format!("channel {} does not exist", id.0)))?;
            row.revision = next_revision(row.revision, "channel")?;
            Ok(())
        }
        ScopeSelector::Bot(BotSelector::CanonicalId(id)) => {
            let row = state
                .bots
                .get_mut(id)
                .ok_or_else(|| AppError::NotFound(format!("bot {} does not exist", id.0)))?;
            row.revision = next_revision(row.revision, "bot")?;
            Ok(())
        }
        _ => Err(AppError::Conflict(
            "membership mutation requires canonical scope".to_owned(),
        )),
    }
}

fn target_exists(state: &DomainState, target: &CanonicalTarget) -> bool {
    match target {
        CanonicalTarget::Instance(_) => true,
        CanonicalTarget::Bot { id, .. } => state.bots.contains_key(id),
        CanonicalTarget::Conversation { id, .. } => state.conversations.contains_key(id),
        CanonicalTarget::Thread { id, .. } => state.threads.contains_key(id),
        CanonicalTarget::Task { id, .. } => state.tasks.contains_key(id),
        CanonicalTarget::Project { id, .. } => state.projects.contains_key(id),
        CanonicalTarget::Channel { id, .. } => state.channels.contains_key(id),
        CanonicalTarget::Operation { operation_id, .. } => state.results.contains_key(operation_id),
        CanonicalTarget::Memory { id, .. } => state.memories.contains_key(id),
        CanonicalTarget::SideEffect { id, .. } => state.side_effects.contains_key(id),
        CanonicalTarget::Membership { scope, member_bot } => state
            .memberships
            .get(&membership_key(scope, member_bot))
            .is_some_and(|row| row.active),
        CanonicalTarget::Approval { .. }
        | CanonicalTarget::Provider { .. }
        | CanonicalTarget::Process { .. } => true,
    }
}

fn target_ref(target: &CanonicalTarget) -> String {
    match target {
        CanonicalTarget::Approval { id, .. } => format!("approval:{}", id.0),
        other => format!("{other:?}"),
    }
}

fn required_materialized_content(payload: &CommandPayload) -> Result<ContentSource, AppError> {
    match payload.content.clone() {
        Some(source @ ContentSource::Text { .. })
        | Some(source @ ContentSource::ArtifactRef { .. }) => Ok(source),
        Some(ContentSource::InputFile { .. } | ContentSource::Stdin) => Err(AppError::Conflict(
            "local content source crossed the Prepared boundary".to_owned(),
        )),
        None => Err(AppError::Conflict(format!(
            "{} requires content",
            payload.command_key
        ))),
    }
}

fn required_field<'a>(payload: &'a CommandPayload, name: &str) -> Result<&'a str, AppError> {
    optional_field(payload, name).ok_or_else(|| {
        AppError::Conflict(format!("{} requires field '{name}'", payload.command_key))
    })
}

fn optional_field<'a>(payload: &'a CommandPayload, name: &str) -> Option<&'a str> {
    payload
        .semantic_options
        .get(name)
        .and_then(|value| value.as_str())
}

fn string_list(payload: &CommandPayload, name: &str) -> Vec<String> {
    match payload.semantic_options.get(name) {
        Some(serde_json::Value::Array(values)) => values
            .iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect(),
        Some(serde_json::Value::String(value)) => vec![value.clone()],
        _ => Vec::new(),
    }
}

fn strip_prefix(value: &str, prefix: &str) -> String {
    value.strip_prefix(prefix).unwrap_or(value).to_owned()
}

fn next_revision(value: i64, label: &str) -> Result<i64, AppError> {
    value
        .checked_add(1)
        .ok_or_else(|| AppError::Internal(format!("{label} revision exhausted")))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn policy_bindings_value(bindings: &BotPolicyBindings) -> serde_json::Value {
    serde_json::json!({
        "brain_policy": bindings.brain_policy,
        "permission_policy": bindings.permission_policy,
        "resource_policy": bindings.resource_policy,
        "provider_policy": bindings.provider_policy,
    })
}

fn constraints_value(constraints: &TaskExecutionConstraints) -> serde_json::Value {
    serde_json::json!({
        "delegate_to_bot": constraints.delegate_to_bot,
        "requested_sender_bot": constraints.requested_sender_bot,
        "deadline": constraints.deadline,
        "budget": constraints.budget,
    })
}

fn execution_generation_of(state: &DomainState, id: &TaskId) -> i64 {
    state
        .tasks
        .get(id)
        .map(|task| task.execution_generation)
        .unwrap_or_default()
}

fn validate_task_control_transition(
    current: TaskStatus,
    command_key: &str,
) -> Result<(), AppError> {
    let allowed = match command_key {
        "task-cancel" => matches!(
            current,
            TaskStatus::Pending
                | TaskStatus::Admitted
                | TaskStatus::Running
                | TaskStatus::Suspended
                | TaskStatus::Deferred
        ),
        "task-suspend" => matches!(
            current,
            TaskStatus::Pending | TaskStatus::Admitted | TaskStatus::Running
        ),
        "task-resume" => matches!(
            current,
            TaskStatus::Suspended | TaskStatus::RecoveryRequired
        ),
        "task-redirect" => matches!(
            current,
            TaskStatus::Pending
                | TaskStatus::Admitted
                | TaskStatus::Running
                | TaskStatus::Suspended
                | TaskStatus::Deferred
        ),
        _ => false,
    };
    if allowed {
        Ok(())
    } else {
        Err(AppError::Conflict(format!(
            "{command_key} is invalid while task is {}",
            task_status_name(current)
        )))
    }
}

/// Transition the orchestration Process linked to a task control directive.
///
/// The Process owns only lifecycle/progress; it never copies task fields. A
/// process reference that no longer resolves (e.g. legacy task predating the
/// producer) is not fatal — the task control already committed. Terminal
/// process transitions are idempotent: re-applying the same terminal control
/// does not advance progress twice or resurrect a terminal Process.
fn advance_process_for_task_control(
    state: &mut DomainState,
    process_ref: &str,
    command_key: &str,
    reason: Option<&str>,
) -> Result<(), AppError> {
    let process_id = ProcessId(strip_prefix(process_ref, "process:"));
    let process = state.processes.get_mut(&process_id).ok_or_else(|| {
        AppError::Internal(format!(
            "task references missing orchestration process: {process_ref}"
        ))
    })?;
    let (next_lifecycle, terminal_reason) = match command_key {
        "task-cancel" => (
            ProcessLifecycle::Cancelled,
            Some(reason.unwrap_or("task cancelled").to_owned()),
        ),
        "task-suspend" => (ProcessLifecycle::Suspended, None),
        "task-resume" | "task-redirect" => (ProcessLifecycle::Running, None),
        _ => return Ok(()),
    };
    // A terminal Process can never be resurrected or advanced by another
    // control operation. Exact retries are intercepted by operation identity
    // before this function is reached.
    let recovery_resume =
        process.lifecycle == ProcessLifecycle::RecoveryRequired && command_key == "task-resume";
    if !recovery_resume
        && matches!(
            process.lifecycle,
            ProcessLifecycle::Completed
                | ProcessLifecycle::Cancelled
                | ProcessLifecycle::Failed
                | ProcessLifecycle::RecoveryRequired
        )
    {
        return Err(AppError::Conflict(format!(
            "process {} is already terminal",
            process.id.0
        )));
    }
    process.lifecycle = next_lifecycle;
    process.terminal_reason = terminal_reason;
    process.progress = process
        .progress
        .checked_add(1)
        .ok_or_else(|| AppError::Internal("process progress exhausted".to_owned()))?;
    process.revision = next_revision(process.revision, "process")?;
    Ok(())
}

fn target_error(payload: &CommandPayload, expected: &str) -> AppError {
    AppError::Conflict(format!(
        "{} expected {expected} target, got {:?}",
        payload.command_key, payload.canonical_target
    ))
}

fn lifecycle_name(value: LifecycleState) -> &'static str {
    match value {
        LifecycleState::Active => "active",
        LifecycleState::Inactive => "inactive",
        LifecycleState::Degraded => "degraded",
        LifecycleState::Terminated => "archived",
    }
}

fn project_lifecycle_name(value: ProjectLifecycle) -> &'static str {
    match value {
        ProjectLifecycle::Active => "active",
        ProjectLifecycle::Archived => "archived",
    }
}

fn task_status_name(value: TaskStatus) -> &'static str {
    match value {
        TaskStatus::Pending => "pending",
        TaskStatus::Admitted => "admitted",
        TaskStatus::Running => "running",
        TaskStatus::Suspended => "suspended",
        TaskStatus::Succeeded => "succeeded",
        TaskStatus::Failed => "failed",
        TaskStatus::Rejected => "rejected",
        TaskStatus::Deferred => "deferred",
        TaskStatus::Cancelled => "cancelled",
        TaskStatus::RecoveryRequired => "recovery-required",
    }
}

fn side_effect_status_name(value: SideEffectStatus) -> &'static str {
    match value {
        SideEffectStatus::Prepared => "prepared",
        SideEffectStatus::Dispatched => "dispatched",
        SideEffectStatus::Confirmed => "confirmed",
        SideEffectStatus::Failed => "failed",
        SideEffectStatus::Unknown => "unknown",
        SideEffectStatus::Reconciling => "reconciling",
        SideEffectStatus::ManualResolutionRequired => "manual-resolution-required",
    }
}

fn validate_existing_binding(
    state: &DomainState,
    operation_id: &OperationId,
    bound_binding: &IdempotencyBindingState,
    request: &OperationRequest,
) -> Result<(), AppError> {
    if bound_binding.command_id != request.command_id {
        return Err(AppError::Conflict(
            "idempotency key is already bound to another command".to_owned(),
        ));
    }
    if bound_binding.expires_at != request.idempotency_key.expires_at {
        return Err(AppError::Conflict(
            "idempotency key expiry does not match durable binding".to_owned(),
        ));
    }
    if operation_id != &request.new_operation_id {
        return Err(AppError::Conflict(
            "command binding operation id mismatch".to_owned(),
        ));
    }
    let stored_digest = state
        .command_request_digests
        .get(&request.command_id)
        .ok_or_else(|| {
            AppError::Internal("command binding exists without request digest".to_owned())
        })?;
    if stored_digest != &request.request_digest {
        return Err(AppError::Conflict(
            "command binding request digest mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn validate_request_identity(request: &OperationRequest) -> Result<(), AppError> {
    if request.command_id.0.trim().is_empty()
        || request.new_operation_id.0.trim().is_empty()
        || request.request_digest.0.trim().is_empty()
        || request.idempotency_key.key_digest.trim().is_empty()
    {
        return Err(AppError::Conflict(
            "operation identity fields must not be empty".to_owned(),
        ));
    }
    if request.idempotency_key.principal_ref != request.payload.principal_ref {
        return Err(AppError::Conflict(
            "idempotency principal does not match payload principal".to_owned(),
        ));
    }
    if let CanonicalTarget::Instance(target_instance) = &request.payload.canonical_target {
        if target_instance != &request.payload.instance_id {
            return Err(AppError::Conflict(
                "instance target does not match request instance".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_committed_result(
    result: &OperationResult,
    request: &OperationRequest,
) -> Result<(), AppError> {
    if result.operation_id != request.new_operation_id
        || result.command_id != request.command_id
        || result.instance_id != request.payload.instance_id
        || result.receipt.operation_id != request.new_operation_id.0
        || result.receipt.resolved_binding_digest != request.request_digest.0
    {
        return Err(AppError::Internal(
            "stored operation result diverges from canonical command binding".to_owned(),
        ));
    }
    Ok(())
}

fn empty_cas() -> CasConditions {
    CasConditions {
        if_revision: None,
        if_generation: None,
        if_host_generation: None,
        if_execution_generation: None,
        if_source_revision: None,
        if_scope_revision: None,
        if_project_revision: None,
        if_channel_revision: None,
        if_membership_generation: None,
        if_proposal_revision: None,
        if_target_scope_revision: None,
        if_receipt_revision: None,
    }
}

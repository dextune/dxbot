//! Domain mutation: target materialization, binding-first identity, receipt, and
//! domain-outcome semantics.
//!
//! [`ApplicationMutator`] consumes the complete [`OperationRequest`] so the
//! command/idempotency/request identities chosen before dispatch are preserved
//! through the authoritative mutation boundary.

use std::sync::{Arc, Mutex};

use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::{
    CanonicalTarget, CasConditions, CommandId, CommandPayload, ConversationId, InstanceId,
    OperationId, OperationRequest, OperationResult, RequestDigest, ThreadId,
};

use crate::outcome::{DomainOutcome, resolve_outcome};
use crate::state::{BotState, ConversationState, DomainState, LifecycleState, ThreadState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    NotFound(String),
    Conflict(String),
    PermissionDenied(String),
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

#[derive(Debug)]
pub struct ApplicationMutator {
    state: Arc<Mutex<DomainState>>,
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
        }
    }

    pub fn snapshot(&self) -> Result<DomainState, AppError> {
        self.lock().map(|guard| guard.clone()).map_err(AppError::Internal)
    }

    /// Applies one complete operation request atomically in the in-memory
    /// fixture. Existing bindings are resolved before any domain effect.
    pub fn mutate(&self, request: &OperationRequest) -> Result<OperationResult, AppError> {
        validate_request_identity(request)?;
        let payload = &request.payload;
        let idempotency_binding = (
            request.idempotency_key.principal_ref.0.clone(),
            request.idempotency_key.key_digest.clone(),
        );
        let mut guard = self
            .lock()
            .map_err(|message| AppError::Internal(format!("domain state lock unavailable: {message}")))?;

        match (
            guard.command_bindings.get(&request.command_id),
            guard.idempotency_bindings.get(&idempotency_binding),
        ) {
            (Some(operation_id), Some(bound_command_id)) => {
                if bound_command_id != &request.command_id {
                    return Err(AppError::Conflict(
                        "idempotency key is already bound to another command".to_string(),
                    ));
                }
                let stored_digest = guard
                    .command_request_digests
                    .get(&request.command_id)
                    .ok_or_else(|| {
                        AppError::Internal(
                            "command binding exists without request digest".to_string(),
                        )
                    })?;
                if stored_digest != &request.request_digest {
                    return Err(AppError::Conflict(
                        "command binding request digest mismatch".to_string(),
                    ));
                }
                return guard.results.get(operation_id).cloned().ok_or_else(|| {
                    AppError::Internal(
                        "committed command binding exists without operation result".to_string(),
                    )
                });
            }
            (None, None) => {}
            _ => {
                return Err(AppError::Conflict(
                    "incomplete or conflicting command/idempotency binding".to_string(),
                ));
            }
        }

        if guard.receipts.contains_key(&request.new_operation_id)
            || guard.results.contains_key(&request.new_operation_id)
        {
            return Err(AppError::Conflict(
                "new operation id is already in use".to_string(),
            ));
        }

        self.validate_target_against(&guard, &payload.canonical_target, &payload.cas)?;
        let outcome = resolve_outcome(&guard, &payload.canonical_target, &payload.cas);
        let identity = OperationIdentity {
            operation_id: request.new_operation_id.clone(),
            command_id: request.command_id.clone(),
            instance_id: payload.instance_id.clone(),
            request_digest: request.request_digest.clone(),
        };
        let result = self.apply(&mut guard, payload, &identity, outcome)?;

        guard
            .command_request_digests
            .insert(identity.command_id.clone(), identity.request_digest.clone());
        guard
            .command_bindings
            .insert(identity.command_id.clone(), identity.operation_id.clone());
        guard
            .idempotency_bindings
            .insert(idempotency_binding, identity.command_id.clone());
        Ok(result)
    }

    pub fn validate_target(
        &self,
        target: &CanonicalTarget,
        cas: &Option<CasConditions>,
    ) -> Result<(), AppError> {
        let guard = self
            .lock()
            .map_err(|message| AppError::Internal(format!("domain state lock unavailable: {message}")))?;
        self.validate_target_against(&guard, target, cas)
    }

    pub fn get_receipt(
        &self,
        operation_id: &OperationId,
    ) -> Result<Option<ReceiptRecord>, AppError> {
        let guard = self
            .lock()
            .map_err(|message| AppError::Internal(format!("domain state lock unavailable: {message}")))?;
        Ok(guard.receipts.get(operation_id).cloned())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, String> {
        self.state
            .lock()
            .map_err(|_| "domain state mutex poisoned".to_string())
    }

    fn validate_target_against(
        &self,
        guard: &DomainState,
        target: &CanonicalTarget,
        cas: &Option<CasConditions>,
    ) -> Result<(), AppError> {
        match resolve_outcome(guard, target, cas) {
            DomainOutcome::Conflict => Err(AppError::Conflict(format!(
                "target {target:?} revision does not match CAS conditions"
            ))),
            DomainOutcome::NotFound => Err(AppError::NotFound(
                "target does not exist but a precondition requires it".to_string(),
            )),
            DomainOutcome::PermissionDenied => Err(AppError::PermissionDenied(
                "principal is not permitted to mutate the target".to_string(),
            )),
            DomainOutcome::Created | DomainOutcome::Updated => Ok(()),
        }
    }

    fn apply(
        &self,
        guard: &mut DomainState,
        payload: &CommandPayload,
        identity: &OperationIdentity,
        outcome: DomainOutcome,
    ) -> Result<OperationResult, AppError> {
        let committed_payload = match (&payload.canonical_target, outcome) {
            (CanonicalTarget::Instance(_), DomainOutcome::Updated)
                if payload.command_key == "bot-create" =>
            {
                self.materialize_bot_from_payload(guard, payload)?
            }
            (CanonicalTarget::Bot { id, .. }, DomainOutcome::Created) => {
                self.materialize_bot(guard, payload, id)
            }
            (CanonicalTarget::Conversation { id, .. }, DomainOutcome::Created) => {
                self.materialize_conversation(guard, id)
            }
            (CanonicalTarget::Thread { id, parent_id, .. }, DomainOutcome::Created) => {
                self.materialize_thread(guard, id, parent_id.clone())
            }
            (CanonicalTarget::Bot { .. }, DomainOutcome::Updated)
                if payload.command_key == "bot-create" =>
            {
                return Err(AppError::Conflict(
                    "bot-create cannot update an existing bot identity".to_string(),
                ));
            }
            (CanonicalTarget::Bot { id, .. }, DomainOutcome::Updated) => {
                self.update_bot(guard, id, payload)
            }
            _ => {
                return Err(AppError::NotFound(format!(
                    "no mutation path for target {outcome:?}"
                )));
            }
        };

        let receipt = ReceiptRecord {
            operation_id: identity.operation_id.0.clone(),
            disposition: ReceiptDisposition::Committed,
            result_ref: format!("operation:{}", identity.operation_id.0),
            resolved_binding_digest: identity.request_digest.0.clone(),
            owner_kind: "application".to_string(),
            lease_until: None,
            last_progress: 1,
            reconciliation_policy: "idempotent".to_string(),
        };
        let result = OperationResult {
            operation_id: identity.operation_id.clone(),
            command_id: identity.command_id.clone(),
            instance_id: identity.instance_id.clone(),
            receipt: receipt.clone(),
            status: "committed".to_string(),
            committed_payload,
            error: None,
            operation_may_continue: false,
        };
        guard.receipts.insert(identity.operation_id.clone(), receipt);
        guard
            .results
            .insert(identity.operation_id.clone(), result.clone());
        Ok(result)
    }

    fn materialize_bot_from_payload(
        &self,
        guard: &mut DomainState,
        payload: &CommandPayload,
    ) -> Result<Option<serde_json::Value>, AppError> {
        let name = payload
            .semantic_options
            .get("name")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| AppError::NotFound("bot-create requires a materialized name".to_string()))?;
        let id = dxbot_core::types::BotId(name.to_string());
        if guard.bots.contains_key(&id) {
            return Err(AppError::Conflict(format!(
                "bot identity already exists: {}",
                id.0
            )));
        }
        Ok(self.materialize_bot(guard, payload, &id))
    }

    fn materialize_bot(
        &self,
        guard: &mut DomainState,
        payload: &CommandPayload,
        id: &dxbot_core::types::BotId,
    ) -> Option<serde_json::Value> {
        let name = payload
            .semantic_options
            .get("name")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| id.0.clone());
        let conversation_id = ConversationId(format!("{}:main", id.0));
        let revision = 1_i64;
        guard.bots.insert(
            id.clone(),
            BotState {
                id: id.clone(),
                name: name.clone(),
                revision,
                lifecycle: LifecycleState::Inactive,
            },
        );
        guard.conversations.insert(
            conversation_id.clone(),
            ConversationState {
                id: conversation_id.clone(),
                bot_id: id.clone(),
                revision,
                messages: Vec::new(),
            },
        );
        Some(serde_json::json!({
            "bot_ref": format!("bot:{}", id.0),
            "bot_revision": revision,
            "lifecycle": "inactive",
            "main_conversation_ref": format!("conversation:{}", conversation_id.0),
        }))
    }

    fn update_bot(
        &self,
        guard: &mut DomainState,
        id: &dxbot_core::types::BotId,
        payload: &CommandPayload,
    ) -> Option<serde_json::Value> {
        let bot = guard.bots.get_mut(id)?;
        bot.revision += 1;
        let lifecycle = match payload.command_key.as_str() {
            "bot-deactivate" => LifecycleState::Inactive,
            "bot-activate" | "bot-restore" => LifecycleState::Active,
            _ => bot.lifecycle,
        };
        bot.lifecycle = lifecycle;
        Some(serde_json::json!({
            "bot_ref": format!("bot:{}", id.0),
            "bot_revision": bot.revision,
            "lifecycle": lifecycle_str(bot.lifecycle),
        }))
    }

    fn materialize_conversation(
        &self,
        guard: &mut DomainState,
        id: &ConversationId,
    ) -> Option<serde_json::Value> {
        let bot_id = dxbot_core::types::BotId(format!("bot-of-{}", id.0));
        guard.conversations.insert(
            id.clone(),
            ConversationState {
                id: id.clone(),
                bot_id,
                revision: 1,
                messages: Vec::new(),
            },
        );
        Some(serde_json::json!({
            "conversation_ref": format!("conversation:{}", id.0),
            "conversation_revision": 1,
        }))
    }

    fn materialize_thread(
        &self,
        guard: &mut DomainState,
        id: &ThreadId,
        parent_id: Option<ConversationId>,
    ) -> Option<serde_json::Value> {
        let conversation_id = parent_id.unwrap_or_else(|| ConversationId(format!("c-{}", id.0)));
        guard.threads.insert(
            id.clone(),
            ThreadState {
                id: id.clone(),
                conversation_id,
                revision: 1,
                parent_message_id: None,
            },
        );
        Some(serde_json::json!({
            "thread_ref": format!("thread:{}", id.0),
            "thread_revision": 1,
        }))
    }
}

fn validate_request_identity(request: &OperationRequest) -> Result<(), AppError> {
    if request.command_id.0.trim().is_empty()
        || request.new_operation_id.0.trim().is_empty()
        || request.request_digest.0.trim().is_empty()
        || request.idempotency_key.key_digest.trim().is_empty()
    {
        return Err(AppError::Conflict(
            "operation identity fields must not be empty".to_string(),
        ));
    }
    if request.idempotency_key.principal_ref != request.payload.principal_ref {
        return Err(AppError::Conflict(
            "idempotency principal does not match payload principal".to_string(),
        ));
    }
    Ok(())
}

fn lifecycle_str(lifecycle: LifecycleState) -> &'static str {
    match lifecycle {
        LifecycleState::Active => "active",
        LifecycleState::Inactive => "inactive",
        LifecycleState::Degraded => "degraded",
        LifecycleState::Terminated => "terminated",
    }
}

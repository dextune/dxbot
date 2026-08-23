//! Domain mutation: target materialization, receipt, and domain-outcome semantics.
//!
//! [`ApplicationMutator`] applies domain mutations against an in-memory
//! [`DomainState`]. It validates the canonical target against the current state
//! and CAS conditions, applies the mutation atomically, records an operation
//! receipt, and returns an [`OperationResult`]. The same command identity is
//! idempotent: a retry with the same payload returns the existing receipt with
//! no duplicate effect.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use dxbot_core::receipt::{ReceiptDisposition, ReceiptRecord};
use dxbot_core::types::{
    CanonicalTarget, CasConditions, CommandId, CommandPayload, ConversationId, InstanceId,
    OperationId, OperationResult, ThreadId,
};

use crate::outcome::{DomainOutcome, resolve_outcome};
use crate::state::{BotState, ConversationState, DomainState, LifecycleState, ThreadState};

/// Application-layer error surfaced when a mutation is rejected by the domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// The canonical target does not exist and a precondition requires it.
    NotFound(String),
    /// The target's revision/generation does not match the CAS conditions.
    Conflict(String),
    /// The principal is not permitted to mutate the target.
    PermissionDenied(String),
    /// An internal invariant or state failure.
    Internal(String),
    /// The requested stream cursor is behind the oldest retained event; the
    /// caller must resync from the current state. The payload carries a resync
    /// hint (the oldest available cursor).
    GapDetected(String),
    /// No event arrived within the requested window.
    Timeout(String),
}

/// The resolved identity of an operation derived from its command payload.
#[derive(Debug, Clone)]
struct OperationIdentity {
    operation_id: OperationId,
    command_id: CommandId,
    instance_id: InstanceId,
    binding_digest: String,
}

/// Apply domain mutations with receipt and domain-outcome semantics.
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
    /// Create a mutator over an empty in-memory domain state.
    pub fn new() -> Self {
        Self::with_state(DomainState::new())
    }

    /// Create a mutator over a caller-provided in-memory domain state.
    ///
    /// Used to seed state for tests and offline scenarios.
    pub fn with_state(state: DomainState) -> Self {
        Self {
            state: Arc::new(Mutex::new(state)),
        }
    }

    /// Clone a snapshot of the current domain state.
    ///
    /// Returns [`AppError::Internal`] when the internal mutex is poisoned,
    /// indicating a prior panic in a critical section.
    pub fn snapshot(&self) -> Result<DomainState, AppError> {
        self.lock().map(|guard| guard.clone()).map_err(AppError::Internal)
    }

    /// Apply a domain mutation described by `payload`, atomically, and return
    /// an [`OperationResult`] carrying the operation receipt.
    pub fn mutate(&self, payload: &CommandPayload) -> Result<OperationResult, AppError> {
        let instance_id = payload.instance_id.clone();
        let binding_digest = binding_digest(payload)?;
        let command_id = command_id_for(payload);
        let operation_id = operation_id_for(payload, &binding_digest);

        let mut guard = self
            .lock()
            .map_err(|m| AppError::Internal(format!("domain state lock unavailable: {m}")))?;

        // Same command bound to a different digest -> typed conflict.
        if let Some(existing_op) = guard.command_bindings.get(&command_id) {
            if existing_op != &operation_id {
                return Err(AppError::Conflict(format!(
                    "idempotency conflict: command {command_id:?} already bound to {existing_op:?}"
                )));
            }
        }

        // Idempotent replay: the same command already committed.
        if guard.receipts.contains_key(&operation_id) {
            if let Some(existing) = guard.results.get(&operation_id) {
                return Ok(existing.clone());
            }
        }

        // Preflight + resolve the domain outcome.
        self.validate_target_against(&guard, &payload.canonical_target, &payload.cas)?;
        let outcome = resolve_outcome(&guard, &payload.canonical_target, &payload.cas);

        let identity = OperationIdentity {
            operation_id,
            command_id,
            instance_id,
            binding_digest,
        };
        let result = self.apply(&mut guard, payload, &identity, outcome)?;
        guard
            .command_bindings
            .insert(identity.command_id.clone(), identity.operation_id.clone());
        Ok(result)
    }

    /// Preflight-validate a target/CAS pair without mutating state.
    pub fn validate_target(
        &self,
        target: &CanonicalTarget,
        cas: &Option<CasConditions>,
    ) -> Result<(), AppError> {
        let guard = self
            .lock()
            .map_err(|m| AppError::Internal(format!("domain state lock unavailable: {m}")))?;
        self.validate_target_against(&guard, target, cas)
    }

    /// Retrieve the receipt for an operation, if one was recorded.
    pub fn get_receipt(
        &self,
        operation_id: &OperationId,
    ) -> Result<Option<ReceiptRecord>, AppError> {
        let guard = self
            .lock()
            .map_err(|m| AppError::Internal(format!("domain state lock unavailable: {m}")))?;
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
            (CanonicalTarget::Bot { id, .. }, DomainOutcome::Created) => {
                self.materialize_bot(guard, payload, id)
            }
            (CanonicalTarget::Conversation { id, .. }, DomainOutcome::Created) => {
                self.materialize_conversation(guard, id)
            }
            (CanonicalTarget::Thread { id, parent_id, .. }, DomainOutcome::Created) => {
                self.materialize_thread(guard, id, parent_id.clone())
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
            resolved_binding_digest: identity.binding_digest.clone(),
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

        guard
            .receipts
            .insert(identity.operation_id.clone(), receipt);
        guard
            .results
            .insert(identity.operation_id.clone(), result.clone());
        Ok(result)
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
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| id.0.clone());

        let conversation_id = ConversationId(format!("main-{}", id.0));
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
            "bot-activate" => LifecycleState::Active,
            "bot-restore" => LifecycleState::Active,
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

fn lifecycle_str(lifecycle: LifecycleState) -> &'static str {
    match lifecycle {
        LifecycleState::Active => "active",
        LifecycleState::Inactive => "inactive",
        LifecycleState::Degraded => "degraded",
        LifecycleState::Terminated => "terminated",
    }
}

fn digest_of(values: &[&str]) -> String {
    let mut hasher = DefaultHasher::new();
    for value in values {
        value.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Stable binding digest for the whole payload (identity of the intent).
fn binding_digest(payload: &CommandPayload) -> Result<String, AppError> {
    let serialized = serde_json::to_string(payload)
        .map_err(|e| AppError::Internal(format!("payload serialization failed: {e}")))?;
    Ok(digest_of(&[&serialized]))
}

/// Command identity: an instance-scoped, command-scoped key.
fn command_id_for(payload: &CommandPayload) -> CommandId {
    let key = digest_of(&[&payload.instance_id.0, &payload.command_key]);
    CommandId(format!("{}-{key}", payload.command_key))
}

/// Operation identity: a command identity pinned to the exact binding digest.
fn operation_id_for(payload: &CommandPayload, binding_digest: &str) -> OperationId {
    let key = digest_of(&[
        &payload.instance_id.0,
        &payload.command_key,
        binding_digest,
    ]);
    OperationId(format!("{}-{key}", payload.command_key))
}

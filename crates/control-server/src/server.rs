//! Control endpoint server: trusted authentication boundary plus owner routing.
//!
//! Every local operation first authenticates the peer. Client-supplied Principal
//! fields are consistency bindings only; they never create authority. Application,
//! Security and Provider owners are routed behind the same authenticated boundary.
//! Cross-owner Application+Security writes use a durable recovery marker rather
//! than duplicating either owner's canonical state.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::ApplicationMutator;
use application::mutation::AppError;
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode, TypedNextAction};
use dxbot_core::types::*;
use provider_host::{HarnessError, ProviderHost, ProviderInfo, ProviderStatus};
use runtime_security::{
    ApprovalBinding, ApprovalDecision, ApprovalDecisionDelta, ApprovalRecord, ApprovalState,
    MembershipAuthorityBinding, MembershipBindingDelta, ParkedGateBinding, ParkedGateState,
    ParkingDelta, PrincipalStatus, SecurityAuditIntent, SecurityDelta, SecurityState,
    SecurityStateStore,
};
use serde_json::{Value, json};

use crate::coordination::{SecurityCoordinationRecord, SecurityCoordinationStore};
use crate::parking_store::{ParkedOperationRecord, ParkedOperationStore};

const OPERATION_ROLE: &str = "mutator";
const LOCAL_OPERATOR_ROLE: &str = "operator";
const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 1000;
const TASK_PROVIDER_CAPABILITY: &str = "llm-chat";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    PermissionDenied(String),
    NotFound(String),
    Conflict(String),
    ApprovalRequired(ApprovalRequired),
    ProviderUnavailable(String),
    GapDetected(String),
    Timeout(String),
    RecoveryRequired(String),
    InternalInvariant(String),
}

/// Typed non-success behavior returned when a high-risk operation is parked
/// behind a bound approval. Compatible with the existing `ErrorCode` surface via
/// [`ErrorCode::ApprovalRequired`]; carries the created approval/operation refs
/// so the caller can locate the pending decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalRequired {
    pub approval_id: ApprovalId,
    pub operation_id: OperationId,
    pub command_key: String,
    pub target: String,
    pub policy_generation: i64,
}

impl ServerError {
    pub fn to_dxbot_error(&self) -> DxbotError {
        if let Self::ApprovalRequired(pending) = self {
            let approval_ref = format!("approval:{}", pending.approval_id.0);
            let operation_ref = format!("operation:{}", pending.operation_id.0);
            return DxbotError {
                code: ErrorCode::ApprovalRequired,
                category: ErrorCategory::Approval,
                message: format!(
                    "{} is high-risk and is parked pending approval {}",
                    pending.command_key, pending.approval_id.0
                ),
                retryable: true,
                operation_ref: Some(operation_ref),
                target_refs: vec![pending.target.clone()],
                field_violations: Vec::new(),
                current_revision: None,
                current_generation: Some(pending.policy_generation),
                resume_cursor: None,
                next_actions: vec![
                    TypedNextAction {
                        action_code: "approval-approve".to_owned(),
                        command_key: "approval-approve".to_owned(),
                        args: json!({ "approval": approval_ref.clone() }),
                    },
                    TypedNextAction {
                        action_code: "approval-deny".to_owned(),
                        command_key: "approval-deny".to_owned(),
                        args: json!({ "approval": approval_ref }),
                    },
                ],
            };
        }
        let (code, category, message, retryable) = match self {
            Self::PermissionDenied(message) => (
                ErrorCode::PermissionDenied,
                ErrorCategory::Permission,
                message.clone(),
                false,
            ),
            Self::NotFound(message) => (
                ErrorCode::NotFound,
                ErrorCategory::Input,
                message.clone(),
                false,
            ),
            Self::Conflict(message) => (
                ErrorCode::Conflict,
                ErrorCategory::Conflict,
                message.clone(),
                false,
            ),
            Self::ProviderUnavailable(message) => (
                ErrorCode::ProviderUnavailable,
                ErrorCategory::Availability,
                message.clone(),
                true,
            ),
            Self::GapDetected(message) => (
                ErrorCode::PartialOrResync,
                ErrorCategory::Recovery,
                message.clone(),
                true,
            ),
            Self::Timeout(message) => (
                ErrorCode::Timeout,
                ErrorCategory::Availability,
                message.clone(),
                true,
            ),
            Self::RecoveryRequired(message) => (
                ErrorCode::RecoveryRequired,
                ErrorCategory::Recovery,
                message.clone(),
                true,
            ),
            Self::InternalInvariant(message) => (
                ErrorCode::InternalInvariant,
                ErrorCategory::Internal,
                message.clone(),
                false,
            ),
            Self::ApprovalRequired(_) => unreachable!("handled by early return above"),
        };
        DxbotError {
            code,
            category,
            message,
            retryable,
            operation_ref: None,
            target_refs: Vec::new(),
            field_violations: Vec::new(),
            current_revision: None,
            current_generation: None,
            resume_cursor: None,
            next_actions: Vec::new(),
        }
    }
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "control-server error: {self:?}")
    }
}

impl std::error::Error for ServerError {}

#[derive(Debug)]
pub struct ControlServer {
    security: Arc<Mutex<SecurityState>>,
    application: Arc<ApplicationMutator>,
    providers: Mutex<ProviderHost>,
    security_store: Option<Arc<SecurityStateStore>>,
    coordination_store: Option<Arc<SecurityCoordinationStore>>,
    parked_store: Option<Arc<ParkedOperationStore>>,
    parked_memory: Mutex<BTreeMap<String, ParkedOperationRecord>>,
    coordination_lock: Mutex<()>,
}

impl ControlServer {
    pub fn new(security: Arc<Mutex<SecurityState>>, application: Arc<ApplicationMutator>) -> Self {
        Self::with_provider_host(security, application, ProviderHost::new())
    }

    pub fn with_provider_host(
        security: Arc<Mutex<SecurityState>>,
        application: Arc<ApplicationMutator>,
        providers: ProviderHost,
    ) -> Self {
        Self {
            security,
            application,
            providers: Mutex::new(providers),
            security_store: None,
            coordination_store: None,
            parked_store: None,
            parked_memory: Mutex::new(BTreeMap::new()),
            coordination_lock: Mutex::new(()),
        }
    }

    pub fn with_persistence(
        security: Arc<Mutex<SecurityState>>,
        application: Arc<ApplicationMutator>,
        providers: ProviderHost,
        security_store: Arc<SecurityStateStore>,
        coordination_store: Arc<SecurityCoordinationStore>,
    ) -> Result<Self, ServerError> {
        let parked_store = Arc::new(ParkedOperationStore::new(
            security_store
                .path()
                .with_file_name("parked-operations.json"),
        ));
        let server = Self {
            security,
            application,
            providers: Mutex::new(providers),
            security_store: Some(security_store),
            coordination_store: Some(coordination_store),
            parked_store: Some(parked_store),
            parked_memory: Mutex::new(BTreeMap::new()),
            coordination_lock: Mutex::new(()),
        };
        let guard = server.coordination_guard()?;
        server.hydrate_parked_memory_locked()?;
        server.recover_pending_locked()?;
        server.drive_continuations_locked()?;
        drop(guard);
        Ok(server)
    }

    pub fn register_local_operator(&self, principal: &PrincipalRef) -> Result<(), ServerError> {
        let _coordination = self.coordination_guard()?;
        self.recover_pending_locked()?;
        let current = self.lock_security()?.clone();
        let mut candidate = current;
        ensure_local_operator(&mut candidate, principal)?;
        self.persist_security_candidate(candidate)
    }

    pub fn preflight(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
        raw_selector: Option<&Value>,
    ) -> Result<(CanonicalTarget, CasConditions), ServerError> {
        self.recover_pending()?;
        self.authorize_payload_identity(authenticated_principal, payload)?;
        self.require_local_operator_or_scope(authenticated_principal, &payload.canonical_target)?;
        match payload.command_key.as_str() {
            "approval-show" | "approval-approve" | "approval-deny" => {
                self.approval_preflight(payload, raw_selector)
            }
            "provider-show" => self.provider_preflight(payload, raw_selector),
            _ => self
                .application
                .preflight(payload, raw_selector)
                .map_err(map_app_error),
        }
    }

    pub fn query(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
    ) -> Result<Value, ServerError> {
        self.recover_pending()?;
        self.authorize_payload_identity(authenticated_principal, payload)?;
        self.require_local_operator_or_scope(authenticated_principal, &payload.canonical_target)?;
        match payload.command_key.as_str() {
            "approval-list" | "approval-show" => self.approval_query(payload),
            "provider-list" | "provider-show" => self.provider_query(payload),
            _ => self.application.query(payload).map_err(map_app_error),
        }
    }

    pub fn watch_next(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
        cursor: Option<&str>,
        timeout: Duration,
    ) -> Result<Value, ServerError> {
        self.recover_pending()?;
        self.authorize_payload_identity(authenticated_principal, payload)?;
        self.require_local_operator_or_scope(authenticated_principal, &payload.canonical_target)?;
        self.application
            .watch_next(payload, cursor, timeout)
            .map_err(map_app_error)
    }

    pub fn handle_request(
        &self,
        authenticated_principal: &PrincipalRef,
        request: &OperationRequest,
    ) -> Result<OperationResult, ServerError> {
        if request.idempotency_key.principal_ref != *authenticated_principal {
            return Err(ServerError::PermissionDenied(
                "idempotency principal binding mismatch".to_owned(),
            ));
        }
        self.authorize_payload_identity(authenticated_principal, &request.payload)?;
        self.authorize_mutation(authenticated_principal, &request.payload.canonical_target)?;
        self.authorize_sender_override(authenticated_principal, &request.payload)?;

        let _coordination = self.coordination_guard()?;
        self.recover_pending_locked()?;
        if let Some(result) = self
            .application
            .lookup_binding(&request.command_id, &request.idempotency_key)
            .map_err(map_app_error)?
        {
            validate_exact_retry(&result, request)?;
            return Ok(result);
        }

        application_contract::validate_materialized_cas(&request.payload)
            .map_err(|error| ServerError::Conflict(error.message))?;
        self.require_provider_admission(&request.payload)?;

        if is_high_risk(&request.payload.command_key)? {
            return self.park_high_risk_locked(authenticated_principal, request);
        }

        let delta = self.security_delta(authenticated_principal, request)?;
        if delta.is_empty() {
            return self.application.mutate(request).map_err(map_app_error);
        }

        let mut candidate = self.lock_security()?.clone();
        candidate.apply_delta(&delta).map_err(map_security_error)?;
        let result = self.commit_with_delta_locked(request, &delta, candidate)?;
        // A committed approval decision may have cleared or denied a parked gate;
        // drive any now-approved continuations to their terminal Application
        // commit before returning.
        self.drive_continuations_locked()?;
        Ok(result)
    }

    /// Commit a request together with its non-empty security delta using the
    /// crash-safe cross-owner protocol. Shared by first submission and parked
    /// operation continuation so both paths apply exactly once and recover
    /// identically.
    fn commit_with_delta_locked(
        &self,
        request: &OperationRequest,
        delta: &SecurityDelta,
        candidate: SecurityState,
    ) -> Result<OperationResult, ServerError> {
        match (&self.security_store, &self.coordination_store) {
            (Some(security_store), Some(coordination_store)) => {
                coordination_store
                    .prepare(&SecurityCoordinationRecord {
                        request: request.clone(),
                        delta: delta.clone(),
                    })
                    .map_err(|error| {
                        ServerError::RecoveryRequired(format!(
                            "cannot prepare cross-owner security transaction: {error}"
                        ))
                    })?;
                let result = match self.application.mutate(request) {
                    Ok(result) => result,
                    Err(error) => {
                        coordination_store.clear().map_err(|clear_error| {
                            ServerError::RecoveryRequired(format!(
                                "Application rejected coordinated mutation ({error:?}) and marker cleanup failed: {clear_error}"
                            ))
                        })?;
                        return Err(map_app_error(error));
                    }
                };
                if let Err(error) = security_store.persist(&candidate) {
                    return Err(ServerError::RecoveryRequired(format!(
                        "Application committed but Security publication failed; recovery marker retained: {error}"
                    )));
                }
                *self.lock_security()? = candidate;
                coordination_store.clear().map_err(|error| {
                    ServerError::RecoveryRequired(format!(
                        "coordinated owners committed but recovery marker cleanup failed: {error}"
                    ))
                })?;
                Ok(result)
            }
            (None, None) => {
                let result = self.application.mutate(request).map_err(map_app_error)?;
                *self.lock_security()? = candidate;
                Ok(result)
            }
            _ => Err(ServerError::InternalInvariant(
                "security persistence and coordination store must be configured together"
                    .to_owned(),
            )),
        }
    }

    fn hydrate_parked_memory_locked(&self) -> Result<(), ServerError> {
        let Some(store) = &self.parked_store else {
            return Ok(());
        };
        let records = store.load_all().map_err(|error| {
            ServerError::RecoveryRequired(format!("cannot load parked operations: {error}"))
        })?;
        let security = self.lock_security()?.clone();
        let mut hydrated = BTreeMap::new();
        for (approval_key, record) in records {
            let approval_id = ApprovalId(approval_key.clone());
            let Some(gate) = security.parking.gate_for_approval(&approval_id) else {
                // Request-first persistence can leave an orphan if the process
                // crashes before canonical Security publication. It is safe to
                // discard because no approval/gate became visible.
                store.remove(&approval_key).map_err(|error| {
                    ServerError::RecoveryRequired(format!(
                        "cannot remove orphan parked operation {approval_key}: {error}"
                    ))
                })?;
                continue;
            };
            if gate.state != ParkedGateState::Parked {
                store.remove(&approval_key).map_err(|error| {
                    ServerError::RecoveryRequired(format!(
                        "cannot remove terminal parked operation {approval_key}: {error}"
                    ))
                })?;
                continue;
            }
            validate_parked_record(&record, &gate.binding)?;
            hydrated.insert(approval_key, record);
        }
        // Every canonical non-terminal gate must retain its exact replay request.
        for gate in security.parking.list_gates() {
            if gate.state == ParkedGateState::Parked
                && !hydrated.contains_key(&gate.binding.approval_id.0)
            {
                return Err(ServerError::RecoveryRequired(format!(
                    "parked request missing for approval {}",
                    gate.binding.approval_id.0
                )));
            }
        }
        *self.parked_memory.lock().map_err(|_| {
            ServerError::InternalInvariant("parked operation lock unavailable".to_owned())
        })? = hydrated;
        Ok(())
    }

    fn park_high_risk_locked(
        &self,
        authenticated_principal: &PrincipalRef,
        request: &OperationRequest,
    ) -> Result<OperationResult, ServerError> {
        const POLICY_GENERATION: i64 = 1;
        if let Some(existing) = self
            .lock_security()?
            .parking
            .gate_for_operation(&request.new_operation_id)
        {
            validate_gate_retry(request, &existing.binding)?;
            return match existing.state {
                ParkedGateState::Parked => {
                    let parked = self
                        .parked_record_locked(&existing.binding.approval_id.0)?
                        .ok_or_else(|| {
                            ServerError::RecoveryRequired(format!(
                                "parked request missing for approval {}",
                                existing.binding.approval_id.0
                            ))
                        })?;
                    if parked.request != *request {
                        return Err(ServerError::Conflict(
                            "parked request does not match exact retry".to_owned(),
                        ));
                    }
                    Err(ServerError::ApprovalRequired(ApprovalRequired {
                        approval_id: existing.binding.approval_id,
                        operation_id: request.new_operation_id.clone(),
                        command_key: request.payload.command_key.clone(),
                        target: existing.binding.target,
                        policy_generation: existing.binding.policy_generation,
                    }))
                }
                ParkedGateState::Denied => Err(ServerError::Conflict(
                    "parked operation was denied".to_owned(),
                )),
                ParkedGateState::Failed => Err(ServerError::Conflict(
                    existing
                        .failure_reason
                        .unwrap_or_else(|| "parked operation continuation failed".to_owned()),
                )),
                ParkedGateState::Continued => Err(ServerError::RecoveryRequired(
                    "continued operation has no durable Application binding".to_owned(),
                )),
            };
        }

        let target = target_label(&request.payload.canonical_target);
        let mut candidate = self.lock_security()?.clone();
        let approval_id = candidate
            .approvals
            .create_bound_approval(
                request.new_operation_id.clone(),
                ApprovalBinding {
                    action: request.payload.command_key.clone(),
                    target: target.clone(),
                    policy_generation: POLICY_GENERATION,
                },
                vec![authenticated_principal.clone()],
            )
            .map_err(map_security_error)?;
        let binding = ParkedGateBinding {
            operation_id: request.new_operation_id.clone(),
            approval_id: approval_id.clone(),
            request_digest: request.request_digest.0.clone(),
            command_key: request.payload.command_key.clone(),
            target: target.clone(),
            policy_generation: POLICY_GENERATION,
        };
        candidate
            .apply_delta(&SecurityDelta {
                membership: None,
                approval: None,
                parking: Some(ParkingDelta::Park {
                    binding: binding.clone(),
                }),
                audit_intent: Some(SecurityAuditIntent::new(
                    request.new_operation_id.clone(),
                    authenticated_principal.clone(),
                    format!("park:{}", request.payload.command_key),
                    target.clone(),
                )),
            })
            .map_err(map_security_error)?;

        let record = ParkedOperationRecord {
            approval_id: approval_id.0.clone(),
            request: request.clone(),
            policy_generation: POLICY_GENERATION,
        };
        self.insert_parked_record_locked(&record)?;
        if let Err(error) = self.persist_security_candidate(candidate) {
            let _ = self.remove_parked_record_locked(&approval_id.0);
            return Err(error);
        }

        Err(ServerError::ApprovalRequired(ApprovalRequired {
            approval_id,
            operation_id: request.new_operation_id.clone(),
            command_key: request.payload.command_key.clone(),
            target,
            policy_generation: POLICY_GENERATION,
        }))
    }

    fn drive_continuations_locked(&self) -> Result<(), ServerError> {
        let wakeups = self.lock_security()?.approval_wakeups();
        for wakeup in wakeups {
            let snapshot = self.lock_security()?.clone();
            let Some(gate) = snapshot.parking.gate_for_approval(&wakeup.approval_id) else {
                // Legacy/manual approvals have no parked operation and retain
                // their wakeup for their owning continuation mechanism.
                continue;
            };
            if gate.state != ParkedGateState::Parked {
                self.remove_parked_record_locked(&wakeup.approval_id.0)?;
                continue;
            }
            let approval = snapshot
                .approvals
                .get_approval(&wakeup.approval_id)
                .map_err(map_security_error)?;
            if approval.state != ApprovalState::Approved {
                continue;
            }
            let record = self
                .parked_record_locked(&wakeup.approval_id.0)?
                .ok_or_else(|| {
                    ServerError::RecoveryRequired(format!(
                        "approved operation {} has no parked request",
                        wakeup.operation_id.0
                    ))
                })?;
            validate_parked_record(&record, &gate.binding)?;

            if let Some(result) = self
                .application
                .lookup_binding(&record.request.command_id, &record.request.idempotency_key)
                .map_err(map_app_error)?
            {
                validate_exact_retry(&result, &record.request)?;
                self.mark_continued_without_application_locked(&wakeup.approval_id)?;
                self.remove_parked_record_locked(&wakeup.approval_id.0)?;
                continue;
            }

            if let Err(error) = self.revalidate_parked_request(&record.request) {
                self.mark_continuation_failed_locked(&wakeup.approval_id, error.to_string())?;
                self.remove_parked_record_locked(&wakeup.approval_id.0)?;
                continue;
            }

            let mut delta = match self
                .security_delta(&record.request.payload.principal_ref, &record.request)
            {
                Ok(delta) => delta,
                Err(error) if is_terminal_continuation_error(&error) => {
                    self.mark_continuation_failed_locked(&wakeup.approval_id, error.to_string())?;
                    self.remove_parked_record_locked(&wakeup.approval_id.0)?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            delta.parking = Some(ParkingDelta::Continue {
                approval_id: wakeup.approval_id.clone(),
            });
            let mut candidate = self.lock_security()?.clone();
            candidate.apply_delta(&delta).map_err(map_security_error)?;
            match self.commit_with_delta_locked(&record.request, &delta, candidate) {
                Ok(_) => self.remove_parked_record_locked(&wakeup.approval_id.0)?,
                Err(error) if is_terminal_continuation_error(&error) => {
                    self.mark_continuation_failed_locked(&wakeup.approval_id, error.to_string())?;
                    self.remove_parked_record_locked(&wakeup.approval_id.0)?;
                }
                Err(error) => return Err(error),
            }
        }
        self.cleanup_terminal_parked_locked()
    }

    fn revalidate_parked_request(&self, request: &OperationRequest) -> Result<(), ServerError> {
        let principal = &request.payload.principal_ref;
        if request.idempotency_key.principal_ref != *principal {
            return Err(ServerError::PermissionDenied(
                "parked idempotency principal binding mismatch".to_owned(),
            ));
        }
        self.authorize_payload_identity(principal, &request.payload)?;
        self.authorize_mutation(principal, &request.payload.canonical_target)?;
        self.authorize_sender_override(principal, &request.payload)?;
        application_contract::validate_materialized_cas(&request.payload)
            .map_err(|error| ServerError::Conflict(error.message))?;
        self.require_provider_admission(&request.payload)
    }

    fn mark_continued_without_application_locked(
        &self,
        approval_id: &ApprovalId,
    ) -> Result<(), ServerError> {
        let delta = SecurityDelta {
            membership: None,
            approval: None,
            parking: Some(ParkingDelta::Continue {
                approval_id: approval_id.clone(),
            }),
            audit_intent: None,
        };
        let mut candidate = self.lock_security()?.clone();
        candidate.apply_delta(&delta).map_err(map_security_error)?;
        self.persist_security_candidate(candidate)
    }

    fn mark_continuation_failed_locked(
        &self,
        approval_id: &ApprovalId,
        reason: String,
    ) -> Result<(), ServerError> {
        let delta = SecurityDelta {
            membership: None,
            approval: None,
            parking: Some(ParkingDelta::Fail {
                approval_id: approval_id.clone(),
                reason,
            }),
            audit_intent: None,
        };
        let mut candidate = self.lock_security()?.clone();
        candidate.apply_delta(&delta).map_err(map_security_error)?;
        self.persist_security_candidate(candidate)
    }

    fn insert_parked_record_locked(
        &self,
        record: &ParkedOperationRecord,
    ) -> Result<(), ServerError> {
        if let Some(store) = &self.parked_store {
            store.insert(record).map_err(|error| {
                ServerError::RecoveryRequired(format!("cannot persist parked operation: {error}"))
            })?;
        }
        let mut parked = self.parked_memory.lock().map_err(|_| {
            ServerError::InternalInvariant("parked operation lock unavailable".to_owned())
        })?;
        match parked.get(&record.approval_id) {
            Some(existing) if existing == record => Ok(()),
            Some(_) => Err(ServerError::Conflict(format!(
                "parked operation conflict for approval {}",
                record.approval_id
            ))),
            None => {
                parked.insert(record.approval_id.clone(), record.clone());
                Ok(())
            }
        }
    }

    fn parked_record_locked(
        &self,
        approval_id: &str,
    ) -> Result<Option<ParkedOperationRecord>, ServerError> {
        self.parked_memory
            .lock()
            .map_err(|_| {
                ServerError::InternalInvariant("parked operation lock unavailable".to_owned())
            })
            .map(|parked| parked.get(approval_id).cloned())
    }

    fn remove_parked_record_locked(&self, approval_id: &str) -> Result<(), ServerError> {
        if let Some(store) = &self.parked_store {
            store.remove(approval_id).map_err(|error| {
                ServerError::RecoveryRequired(format!(
                    "cannot remove terminal parked operation {approval_id}: {error}"
                ))
            })?;
        }
        self.parked_memory
            .lock()
            .map_err(|_| {
                ServerError::InternalInvariant("parked operation lock unavailable".to_owned())
            })?
            .remove(approval_id);
        Ok(())
    }

    fn cleanup_terminal_parked_locked(&self) -> Result<(), ServerError> {
        let terminal = self
            .lock_security()?
            .parking
            .list_gates()
            .into_iter()
            .filter(|record| record.state != ParkedGateState::Parked)
            .map(|record| record.binding.approval_id.0)
            .collect::<Vec<_>>();
        for approval_id in terminal {
            self.remove_parked_record_locked(&approval_id)?;
        }
        Ok(())
    }

    pub fn lookup_binding(
        &self,
        authenticated_principal: &PrincipalRef,
        command_id: &CommandId,
        key: &IdempotencyKey,
    ) -> Result<Option<OperationResult>, ServerError> {
        if key.principal_ref != *authenticated_principal {
            return Err(ServerError::PermissionDenied(
                "binding lookup principal mismatch".to_owned(),
            ));
        }
        let _coordination = self.coordination_guard()?;
        self.recover_pending_locked()?;
        self.require_active_principal(authenticated_principal)?;
        self.application
            .lookup_binding(command_id, key)
            .map_err(map_app_error)
    }

    fn require_provider_admission(&self, payload: &CommandPayload) -> Result<(), ServerError> {
        if !matches!(payload.command_key.as_str(), "bot-activate" | "task-submit") {
            return Ok(());
        }
        let providers = self.providers.lock().map_err(|_| {
            ServerError::InternalInvariant("provider host lock unavailable".to_owned())
        })?;
        let ready = providers
            .list_providers(Some(TASK_PROVIDER_CAPABILITY))
            .into_iter()
            .any(|provider| provider.status == ProviderStatus::Ready);
        if ready {
            Ok(())
        } else {
            Err(ServerError::ProviderUnavailable(format!(
                "{} requires a Ready provider with capability {TASK_PROVIDER_CAPABILITY}",
                payload.command_key
            )))
        }
    }

    fn security_delta(
        &self,
        authenticated_principal: &PrincipalRef,
        request: &OperationRequest,
    ) -> Result<SecurityDelta, ServerError> {
        let payload = &request.payload;
        match payload.command_key.as_str() {
            "approval-approve" | "approval-deny" => {
                let CanonicalTarget::Approval { id, revision } = &payload.canonical_target else {
                    return Err(ServerError::Conflict(
                        "approval decision requires Approval target".to_owned(),
                    ));
                };
                let expected_revision = payload
                    .cas
                    .as_ref()
                    .and_then(|cas| cas.if_revision)
                    .unwrap_or(*revision);
                let is_deny = payload.command_key == "approval-deny";
                let parking = if is_deny
                    && self
                        .lock_security()?
                        .parking
                        .gate_for_approval(id)
                        .is_some()
                {
                    Some(ParkingDelta::Deny {
                        approval_id: id.clone(),
                        reason: payload
                            .semantic_options
                            .get("reason")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    })
                } else {
                    None
                };
                Ok(SecurityDelta {
                    membership: None,
                    approval: Some(ApprovalDecisionDelta {
                        approval_id: id.clone(),
                        expected_revision,
                        decision: if is_deny {
                            ApprovalDecision::Deny
                        } else {
                            ApprovalDecision::Approve
                        },
                        by: authenticated_principal.clone(),
                    }),
                    parking,
                    audit_intent: Some(SecurityAuditIntent::new(
                        request.new_operation_id.clone(),
                        authenticated_principal.clone(),
                        payload.command_key.clone(),
                        format!("approval:{}", id.0),
                    )),
                })
            }
            "project-member-set" | "channel-member-set" => {
                let CanonicalTarget::Membership { scope, member_bot } = &payload.canonical_target
                else {
                    return Err(ServerError::Conflict(
                        "membership set requires Membership target".to_owned(),
                    ));
                };
                let snapshot = self.application.snapshot().map_err(map_app_error)?;
                let current = snapshot
                    .memberships
                    .values()
                    .find(|record| record.scope == *scope && record.member_bot == *member_bot);
                let generation = match current {
                    Some(record) => record.generation.checked_add(1).ok_or_else(|| {
                        ServerError::InternalInvariant(
                            "membership authority generation exhausted".to_owned(),
                        )
                    })?,
                    None => 1,
                };
                let role = payload
                    .semantic_options
                    .get("role_ref")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ServerError::Conflict("membership set requires role_ref".to_owned())
                    })?;
                let binding = MembershipAuthorityBinding {
                    binding_id: membership_subject_id(scope, member_bot)?,
                    scope: scope.clone(),
                    member_bot: member_bot.clone(),
                    role: role.to_owned(),
                    generation,
                    active: true,
                };
                Ok(SecurityDelta {
                    membership: Some(MembershipBindingDelta::Upsert { binding }),
                    approval: None,
                    parking: None,
                    audit_intent: Some(SecurityAuditIntent::new(
                        request.new_operation_id.clone(),
                        authenticated_principal.clone(),
                        payload.command_key.clone(),
                        target_label(&payload.canonical_target),
                    )),
                })
            }
            "project-member-remove" | "channel-member-remove" => {
                let CanonicalTarget::Membership { scope, member_bot } = &payload.canonical_target
                else {
                    return Err(ServerError::Conflict(
                        "membership remove requires Membership target".to_owned(),
                    ));
                };
                let snapshot = self.application.snapshot().map_err(map_app_error)?;
                let current = snapshot
                    .memberships
                    .values()
                    .find(|record| record.scope == *scope && record.member_bot == *member_bot)
                    .cloned()
                    .ok_or_else(|| ServerError::NotFound("membership does not exist".to_owned()))?;
                if !current.active {
                    return Err(ServerError::NotFound("membership is not active".to_owned()));
                }
                let expected = payload
                    .cas
                    .as_ref()
                    .and_then(|cas| cas.if_membership_generation)
                    .ok_or_else(|| {
                        ServerError::Conflict(
                            "membership remove requires generation CAS".to_owned(),
                        )
                    })?;
                if current.generation != expected {
                    return Err(ServerError::Conflict(format!(
                        "membership generation mismatch: expected {expected}, current {}",
                        current.generation
                    )));
                }
                let binding = MembershipAuthorityBinding {
                    binding_id: membership_subject_id(scope, member_bot)?,
                    scope: scope.clone(),
                    member_bot: member_bot.clone(),
                    role: current.role,
                    generation: current.generation,
                    active: false,
                };
                Ok(SecurityDelta {
                    membership: Some(MembershipBindingDelta::Upsert { binding }),
                    approval: None,
                    parking: None,
                    audit_intent: Some(SecurityAuditIntent::new(
                        request.new_operation_id.clone(),
                        authenticated_principal.clone(),
                        payload.command_key.clone(),
                        target_label(&payload.canonical_target),
                    )),
                })
            }
            "project-create" => {
                let project = payload
                    .semantic_options
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ServerError::Conflict("project-create requires name".to_owned())
                    })?;
                let owner = payload
                    .semantic_options
                    .get("owner_bot")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ServerError::Conflict("project-create requires owner_bot".to_owned())
                    })?;
                let scope = ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
                    project.to_owned(),
                )));
                let member_bot = BotSelector::CanonicalId(BotId(strip_ref(owner, "bot:")));
                let binding = MembershipAuthorityBinding {
                    binding_id: membership_subject_id(&scope, &member_bot)?,
                    scope,
                    member_bot,
                    role: "owner".to_owned(),
                    generation: 1,
                    active: true,
                };
                Ok(SecurityDelta {
                    membership: Some(MembershipBindingDelta::Upsert { binding }),
                    approval: None,
                    parking: None,
                    audit_intent: Some(SecurityAuditIntent::new(
                        request.new_operation_id.clone(),
                        authenticated_principal.clone(),
                        payload.command_key.clone(),
                        format!("project:{project}"),
                    )),
                })
            }
            _ => Ok(SecurityDelta {
                membership: None,
                approval: None,
                parking: None,
                audit_intent: None,
            }),
        }
    }

    fn recover_pending(&self) -> Result<(), ServerError> {
        let _guard = self.coordination_guard()?;
        self.recover_pending_locked()
    }

    fn recover_pending_locked(&self) -> Result<(), ServerError> {
        let (Some(security_store), Some(coordination_store)) =
            (&self.security_store, &self.coordination_store)
        else {
            return Ok(());
        };
        let Some(record) = coordination_store.load().map_err(|error| {
            ServerError::RecoveryRequired(format!(
                "cannot load security coordination marker: {error}"
            ))
        })?
        else {
            return Ok(());
        };
        let committed = match self
            .application
            .lookup_binding(&record.request.command_id, &record.request.idempotency_key)
            .map_err(map_app_error)?
        {
            Some(result) => {
                validate_exact_retry(&result, &record.request)?;
                true
            }
            None => false,
        };
        if committed {
            let mut candidate = self.lock_security()?.clone();
            candidate
                .apply_delta(&record.delta)
                .map_err(map_security_error)?;
            security_store.persist(&candidate).map_err(|error| {
                ServerError::RecoveryRequired(format!(
                    "cannot recover Security half of committed Application transaction: {error}"
                ))
            })?;
            *self.lock_security()? = candidate;
        }
        coordination_store.clear().map_err(|error| {
            ServerError::RecoveryRequired(format!(
                "cannot clear recovered security coordination marker: {error}"
            ))
        })
    }

    fn persist_security_candidate(&self, candidate: SecurityState) -> Result<(), ServerError> {
        if let Some(store) = &self.security_store {
            store.persist(&candidate).map_err(|error| {
                ServerError::RecoveryRequired(format!(
                    "cannot persist canonical Security state: {error}"
                ))
            })?;
        }
        *self.lock_security()? = candidate;
        Ok(())
    }

    fn coordination_guard(&self) -> Result<std::sync::MutexGuard<'_, ()>, ServerError> {
        self.coordination_lock
            .lock()
            .map_err(|_| ServerError::InternalInvariant("coordination lock unavailable".to_owned()))
    }

    fn lock_security(&self) -> Result<std::sync::MutexGuard<'_, SecurityState>, ServerError> {
        self.security.lock().map_err(|_| {
            ServerError::InternalInvariant("security state lock unavailable".to_owned())
        })
    }

    fn approval_preflight(
        &self,
        payload: &CommandPayload,
        raw_selector: Option<&Value>,
    ) -> Result<(CanonicalTarget, CasConditions), ServerError> {
        let selector = selector_value(raw_selector).or(match &payload.canonical_target {
            CanonicalTarget::Approval { id, .. } => Some(id.0.as_str()),
            _ => None,
        });
        let selector = selector.ok_or_else(|| {
            ServerError::Conflict(format!(
                "{} requires an approval selector",
                payload.command_key
            ))
        })?;
        let security = self.lock_security()?;
        let record = if let Some(operation) = selector.strip_prefix("operation:") {
            security
                .approvals
                .find_by_operation(&OperationId(operation.to_owned()))
                .map_err(map_security_error)?
        } else {
            let id = ApprovalId(strip_ref(selector, "approval:"));
            security
                .approvals
                .get_approval(&id)
                .map_err(map_security_error)?
        };
        let mut cas = payload.cas.unwrap_or_else(empty_cas);
        if cas.if_revision.is_none() {
            cas.if_revision = Some(record.revision);
        }
        Ok((
            CanonicalTarget::Approval {
                id: record.id,
                revision: record.revision,
            },
            cas,
        ))
    }

    fn approval_query(&self, payload: &CommandPayload) -> Result<Value, ServerError> {
        let security = self.lock_security()?;
        match payload.command_key.as_str() {
            "approval-list" => {
                let state_filter = payload
                    .semantic_options
                    .get("state")
                    .and_then(Value::as_str)
                    .map(str::to_ascii_lowercase);
                let target_scope = target_scope_label(&payload.canonical_target);
                let semantic_scope = payload
                    .semantic_options
                    .get("scope")
                    .and_then(Value::as_str);
                let scope_filter = semantic_scope.or(target_scope.as_deref());
                let records = security
                    .approvals
                    .list_approvals()
                    .into_iter()
                    .filter(|record| {
                        state_filter
                            .as_ref()
                            .is_none_or(|state| approval_state_name(record.state) == state.as_str())
                    })
                    .filter(|record| {
                        scope_filter.is_none_or(|scope| {
                            approval_scope_matches(&record.binding.target, scope)
                        })
                    })
                    .collect();
                page_approval_records(records, payload)
            }
            "approval-show" => {
                let CanonicalTarget::Approval { id, .. } = &payload.canonical_target else {
                    return Err(ServerError::Conflict(
                        "approval-show requires Approval target".to_owned(),
                    ));
                };
                let record = security
                    .approvals
                    .get_approval(id)
                    .map_err(map_security_error)?;
                Ok(approval_value(&record))
            }
            _ => Err(ServerError::InternalInvariant(
                "invalid approval query route".to_owned(),
            )),
        }
    }

    fn provider_preflight(
        &self,
        payload: &CommandPayload,
        raw_selector: Option<&Value>,
    ) -> Result<(CanonicalTarget, CasConditions), ServerError> {
        let selector = selector_value(raw_selector).or(match &payload.canonical_target {
            CanonicalTarget::Provider { id, .. } => Some(id.0.as_str()),
            _ => None,
        });
        let selector = selector.ok_or_else(|| {
            ServerError::Conflict(format!(
                "{} requires a provider selector",
                payload.command_key
            ))
        })?;
        let id = ProviderId(strip_ref(selector, "provider:"));
        let providers = self.providers.lock().map_err(|_| {
            ServerError::InternalInvariant("provider host lock unavailable".to_owned())
        })?;
        let info = providers.get_provider(&id).map_err(map_provider_error)?;
        let mut cas = payload.cas.unwrap_or_else(empty_cas);
        if cas.if_generation.is_none() {
            cas.if_generation = Some(info.generation);
        }
        Ok((
            CanonicalTarget::Provider {
                id: info.id,
                generation: info.generation,
            },
            cas,
        ))
    }

    fn provider_query(&self, payload: &CommandPayload) -> Result<Value, ServerError> {
        let providers = self.providers.lock().map_err(|_| {
            ServerError::InternalInvariant("provider host lock unavailable".to_owned())
        })?;
        match payload.command_key.as_str() {
            "provider-list" => {
                let capability = payload
                    .semantic_options
                    .get("capability")
                    .and_then(Value::as_str);
                page_provider_records(providers.list_providers(capability), payload)
            }
            "provider-show" => {
                let CanonicalTarget::Provider { id, .. } = &payload.canonical_target else {
                    return Err(ServerError::Conflict(
                        "provider-show requires Provider target".to_owned(),
                    ));
                };
                providers
                    .get_provider(id)
                    .map(|info| provider_value(&info))
                    .map_err(map_provider_error)
            }
            _ => Err(ServerError::InternalInvariant(
                "invalid provider query route".to_owned(),
            )),
        }
    }

    fn authorize_payload_identity(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
    ) -> Result<(), ServerError> {
        if payload.principal_ref != *authenticated_principal {
            return Err(ServerError::PermissionDenied(
                "request principal binding mismatch".to_owned(),
            ));
        }
        self.require_active_principal(authenticated_principal)
    }

    fn authorize_sender_override(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
    ) -> Result<(), ServerError> {
        if payload.command_key != "task-submit"
            || payload
                .semantic_options
                .get("requested_sender_bot")
                .is_none()
        {
            return Ok(());
        }
        let security = self.lock_security()?;
        let principal = active_principal(&security, authenticated_principal)?;
        if security
            .authority
            .check_global_authority(&principal.ref_, LOCAL_OPERATOR_ROLE)
            .map_err(map_security_error)?
        {
            Ok(())
        } else {
            Err(ServerError::PermissionDenied(
                "requested_sender_bot requires local operator authority".to_owned(),
            ))
        }
    }

    fn authorize_mutation(
        &self,
        authenticated_principal: &PrincipalRef,
        target: &CanonicalTarget,
    ) -> Result<(), ServerError> {
        self.require_local_operator_or_scope(authenticated_principal, target)
    }

    fn require_local_operator_or_scope(
        &self,
        authenticated_principal: &PrincipalRef,
        target: &CanonicalTarget,
    ) -> Result<(), ServerError> {
        let security = self.lock_security()?;
        let principal_state = active_principal(&security, authenticated_principal)?;
        if security
            .authority
            .check_global_authority(&principal_state.ref_, LOCAL_OPERATOR_ROLE)
            .map_err(map_security_error)?
        {
            return Ok(());
        }
        let scoped_authorized = match scope_for_target(target) {
            Some(scope) => security
                .authority
                .check_authority(&principal_state.ref_, &scope, OPERATION_ROLE)
                .map_err(map_security_error)?,
            None => false,
        };
        if scoped_authorized {
            Ok(())
        } else {
            Err(ServerError::PermissionDenied("not authorized".to_owned()))
        }
    }

    fn require_active_principal(&self, principal: &PrincipalRef) -> Result<(), ServerError> {
        let security = self.lock_security()?;
        active_principal(&security, principal).map(|_| ())
    }
}

fn is_high_risk(command_key: &str) -> Result<bool, ServerError> {
    application_contract::metadata_for_key(command_key)
        .map(|metadata| metadata.security_class == "high-risk")
        .ok_or_else(|| {
            ServerError::InternalInvariant(format!(
                "command registry metadata missing for {command_key}"
            ))
        })
}

fn validate_gate_retry(
    request: &OperationRequest,
    binding: &ParkedGateBinding,
) -> Result<(), ServerError> {
    if binding.operation_id != request.new_operation_id
        || binding.request_digest != request.request_digest.0
        || binding.command_key != request.payload.command_key
        || binding.target != target_label(&request.payload.canonical_target)
    {
        return Err(ServerError::Conflict(
            "parked operation identity does not match exact retry".to_owned(),
        ));
    }
    Ok(())
}

fn validate_parked_record(
    record: &ParkedOperationRecord,
    binding: &ParkedGateBinding,
) -> Result<(), ServerError> {
    validate_gate_retry(&record.request, binding)?;
    if record.approval_id != binding.approval_id.0
        || record.policy_generation != binding.policy_generation
    {
        return Err(ServerError::RecoveryRequired(format!(
            "parked operation binding mismatch for approval {}",
            binding.approval_id.0
        )));
    }
    Ok(())
}

fn is_terminal_continuation_error(error: &ServerError) -> bool {
    matches!(
        error,
        ServerError::PermissionDenied(_)
            | ServerError::NotFound(_)
            | ServerError::Conflict(_)
            | ServerError::ProviderUnavailable(_)
    )
}

fn validate_exact_retry(
    result: &OperationResult,
    request: &OperationRequest,
) -> Result<(), ServerError> {
    if result.command_id != request.command_id
        || result.operation_id != request.new_operation_id
        || result.instance_id != request.payload.instance_id
        || result.receipt.operation_id != request.new_operation_id.0
        || result.receipt.resolved_binding_digest != request.request_digest.0
    {
        return Err(ServerError::Conflict(
            "existing command binding does not match exact retry identity".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_local_operator(
    security: &mut SecurityState,
    principal: &PrincipalRef,
) -> Result<(), ServerError> {
    if security.principals.resolve_principal(principal).is_err() {
        security
            .principals
            .register_principal(principal.clone())
            .map_err(map_security_error)?;
    }
    security
        .authority
        .bind_global_authority(principal, LOCAL_OPERATOR_ROLE)
        .map_err(map_security_error)
}

fn active_principal(
    security: &SecurityState,
    principal: &PrincipalRef,
) -> Result<runtime_security::PrincipalState, ServerError> {
    let principal_state = security
        .principals
        .resolve_principal(principal)
        .map_err(|_| ServerError::PermissionDenied("unauthenticated principal".to_owned()))?;
    if principal_state.status != PrincipalStatus::Active {
        return Err(ServerError::PermissionDenied(
            "principal is not active".to_owned(),
        ));
    }
    Ok(principal_state)
}

fn page_approval_records(
    records: Vec<ApprovalRecord>,
    payload: &CommandPayload,
) -> Result<Value, ServerError> {
    let size = page_size(payload)?;
    let cursor = cursor(payload);
    let rows = records
        .into_iter()
        .map(|record| (record.id.0.clone(), approval_value(&record)))
        .collect::<Vec<_>>();
    Ok(page_rows(rows, size, cursor.as_deref()))
}

fn page_provider_records(
    records: Vec<ProviderInfo>,
    payload: &CommandPayload,
) -> Result<Value, ServerError> {
    let size = page_size(payload)?;
    let cursor = cursor(payload);
    let rows = records
        .into_iter()
        .map(|record| (record.id.0.clone(), provider_value(&record)))
        .collect::<Vec<_>>();
    Ok(page_rows(rows, size, cursor.as_deref()))
}

fn page_rows(mut rows: Vec<(String, Value)>, page_size: usize, cursor: Option<&str>) -> Value {
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    if let Some(cursor) = cursor {
        rows.retain(|(key, _)| key.as_str() > cursor);
    }
    let has_more = rows.len() > page_size;
    let rows = rows.into_iter().take(page_size).collect::<Vec<_>>();
    let next_cursor = has_more
        .then(|| rows.last().map(|(key, _)| key.clone()))
        .flatten();
    json!({
        "items": rows.into_iter().map(|(_, value)| value).collect::<Vec<_>>(),
        "next_cursor": next_cursor,
        "has_more": has_more,
    })
}

fn approval_value(record: &ApprovalRecord) -> Value {
    json!({
        "approval_ref": format!("approval:{}", record.id.0),
        "operation_ref": format!("operation:{}", record.operation_id.0),
        "action": record.binding.action,
        "target": record.binding.target,
        "policy_generation": record.binding.policy_generation,
        "revision": record.revision,
        "state": approval_state_name(record.state),
        "required_approvers": record.required_approvers.iter().map(|principal| principal.0.clone()).collect::<Vec<_>>(),
        "decisions": record.decisions.iter().map(|decision| json!({
            "by": decision.by.0,
            "decision": format!("{:?}", decision.decision).to_ascii_lowercase(),
        })).collect::<Vec<_>>(),
    })
}

fn approval_state_name(state: ApprovalState) -> &'static str {
    match state {
        ApprovalState::Pending => "pending",
        ApprovalState::Approved => "approved",
        ApprovalState::Denied => "denied",
        ApprovalState::Expired => "expired",
        ApprovalState::Revoked => "revoked",
    }
}

fn target_scope_label(target: &CanonicalTarget) -> Option<String> {
    match target {
        CanonicalTarget::Bot { id, .. } => Some(format!("bot:{}", id.0)),
        CanonicalTarget::Project { id, .. } => Some(format!("project:{}", id.0)),
        CanonicalTarget::Channel { id, .. } => Some(format!("channel:{}", id.0)),
        _ => None,
    }
}

fn approval_scope_matches(target: &str, scope: &str) -> bool {
    target == scope
        || target
            .strip_prefix(scope)
            .is_some_and(|suffix| suffix.starts_with('/') || suffix.starts_with(':'))
}

fn provider_value(record: &ProviderInfo) -> Value {
    json!({
        "provider_ref": format!("provider:{}", record.id.0),
        "capability": record.capability,
        "generation": record.generation,
        "status": format!("{:?}", record.status).to_ascii_lowercase(),
    })
}

fn page_size(payload: &CommandPayload) -> Result<usize, ServerError> {
    let value = match payload.semantic_options.get("page_size") {
        Some(Value::String(value)) => value
            .parse::<usize>()
            .map_err(|_| ServerError::Conflict("page_size must be an integer".to_owned()))?,
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| ServerError::Conflict("page_size is invalid".to_owned()))?,
        None => return Ok(DEFAULT_PAGE_SIZE),
        _ => return Err(ServerError::Conflict("page_size is invalid".to_owned())),
    };
    if !(1..=MAX_PAGE_SIZE).contains(&value) {
        return Err(ServerError::Conflict(format!(
            "page_size must be in 1..={MAX_PAGE_SIZE}"
        )));
    }
    Ok(value)
}

fn cursor(payload: &CommandPayload) -> Option<String> {
    payload
        .semantic_options
        .get("cursor")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn selector_value(selector: Option<&Value>) -> Option<&str> {
    selector
        .and_then(|value| value.get("value"))
        .and_then(Value::as_str)
}

fn strip_ref(value: &str, prefix: &str) -> String {
    value.strip_prefix(prefix).unwrap_or(value).to_owned()
}

fn membership_subject_id(
    scope: &ScopeSelector,
    member_bot: &BotSelector,
) -> Result<String, ServerError> {
    serde_json::to_string(&(scope, member_bot))
        .map(|encoded| format!("membership-subject:{encoded}"))
        .map_err(|error| {
            ServerError::InternalInvariant(format!(
                "cannot encode membership authority subject: {error}"
            ))
        })
}

fn target_label(target: &CanonicalTarget) -> String {
    match target {
        CanonicalTarget::Membership { scope, member_bot } => {
            format!("membership:{scope:?}:{member_bot:?}")
        }
        CanonicalTarget::Approval { id, .. } => format!("approval:{}", id.0),
        other => format!("{other:?}"),
    }
}

fn map_security_error(error: runtime_security::Error) -> ServerError {
    match error {
        runtime_security::Error::UnknownApproval(id) => {
            ServerError::NotFound(format!("approval {} not found", id.0))
        }
        runtime_security::Error::ApprovalAlreadyDecided(id)
        | runtime_security::Error::ApprovalRevisionExhausted(id) => {
            ServerError::Conflict(format!("approval {} is stale or already decided", id.0))
        }
        runtime_security::Error::InvalidApprover(principal) => {
            ServerError::PermissionDenied(format!("{} is not an approver", principal.0))
        }
        runtime_security::Error::DuplicatePrincipal(principal)
        | runtime_security::Error::UnknownPrincipal(principal) => {
            ServerError::PermissionDenied(format!("principal unavailable: {}", principal.0))
        }
        runtime_security::Error::UnknownAuthorityBinding(binding) => {
            ServerError::NotFound(format!("authority binding not found: {binding}"))
        }
        runtime_security::Error::StaleAuthorityBinding(binding) => {
            ServerError::Conflict(format!("stale authority binding: {binding}"))
        }
        runtime_security::Error::InvalidApprovalBinding => {
            ServerError::Conflict("approval binding is invalid".to_owned())
        }
        runtime_security::Error::ApprovalIdSpaceExhausted => {
            ServerError::InternalInvariant("approval id space exhausted".to_owned())
        }
        runtime_security::Error::AuditIntentConflict(key) => {
            ServerError::Conflict(format!("audit intent conflict: {key}"))
        }
        runtime_security::Error::InvalidParkedGate(operation) => {
            ServerError::Conflict(format!("invalid parked operation: {}", operation.0))
        }
        runtime_security::Error::ParkedGateConflict(operation) => {
            ServerError::Conflict(format!("parked operation conflict: {}", operation.0))
        }
        runtime_security::Error::UnknownParkedGate(approval) => {
            ServerError::NotFound(format!("parked approval not found: {}", approval.0))
        }
        runtime_security::Error::ParkedGateDenied(approval) => {
            ServerError::Conflict(format!("parked approval was denied: {}", approval.0))
        }
    }
}

fn map_provider_error(error: HarnessError) -> ServerError {
    match error {
        HarnessError::ProviderNotFound { id } => {
            ServerError::NotFound(format!("provider {} not found", id.0))
        }
        HarnessError::ProviderUnavailable { id } => {
            ServerError::ProviderUnavailable(format!("provider {} is unavailable", id.0))
        }
        other => ServerError::InternalInvariant(format!("provider owner error: {other:?}")),
    }
}

fn map_app_error(error: AppError) -> ServerError {
    match error {
        AppError::PermissionDenied(message) => ServerError::PermissionDenied(message),
        AppError::NotFound(message) => ServerError::NotFound(message),
        AppError::Conflict(message) => ServerError::Conflict(message),
        AppError::GapDetected(message) => ServerError::GapDetected(message),
        AppError::Timeout(message) => ServerError::Timeout(message),
        AppError::Internal(message) => ServerError::InternalInvariant(message),
    }
}

fn scope_for_target(target: &CanonicalTarget) -> Option<ScopeSelector> {
    match target {
        CanonicalTarget::Bot { id, .. } => {
            Some(ScopeSelector::Bot(BotSelector::CanonicalId(id.clone())))
        }
        CanonicalTarget::Project { id, .. } => Some(ScopeSelector::Project(
            ProjectSelector::CanonicalId(id.clone()),
        )),
        CanonicalTarget::Channel { id, .. } => Some(ScopeSelector::Channel(
            ChannelSelector::CanonicalId(id.clone()),
        )),
        CanonicalTarget::Membership { scope, .. } => Some(scope.clone()),
        CanonicalTarget::Instance(_)
        | CanonicalTarget::Conversation { .. }
        | CanonicalTarget::Thread { .. }
        | CanonicalTarget::Task { .. }
        | CanonicalTarget::Operation { .. }
        | CanonicalTarget::Approval { .. }
        | CanonicalTarget::Memory { .. }
        | CanonicalTarget::Provider { .. }
        | CanonicalTarget::Process { .. }
        | CanonicalTarget::SideEffect { .. } => None,
    }
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

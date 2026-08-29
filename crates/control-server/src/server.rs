//! Control endpoint server: trusted authentication boundary plus owner routing.
//!
//! Every local operation first authenticates the peer. Client-supplied Principal
//! fields are consistency bindings only; they never create authority. Application,
//! Security and Provider owners are routed behind the same authenticated boundary.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use application::ApplicationMutator;
use application::mutation::AppError;
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::*;
use provider_host::{HarnessError, ProviderHost, ProviderInfo};
use runtime_security::{
    ApprovalManager, ApprovalRecord, AuthorityManager, PrincipalManager, PrincipalStatus,
};
use serde_json::{Value, json};

const OPERATION_ROLE: &str = "mutator";
const LOCAL_OPERATOR_ROLE: &str = "operator";
const DEFAULT_PAGE_SIZE: usize = 50;
const MAX_PAGE_SIZE: usize = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    PermissionDenied(String),
    NotFound(String),
    Conflict(String),
    GapDetected(String),
    Timeout(String),
    InternalInvariant(String),
}

impl ServerError {
    pub fn to_dxbot_error(&self) -> DxbotError {
        let (code, category, message) = match self {
            Self::PermissionDenied(message) => {
                (ErrorCode::PermissionDenied, ErrorCategory::Permission, message.clone())
            }
            Self::NotFound(message) => (ErrorCode::NotFound, ErrorCategory::Input, message.clone()),
            Self::Conflict(message) => (ErrorCode::Conflict, ErrorCategory::Conflict, message.clone()),
            Self::GapDetected(message) => (
                ErrorCode::PartialOrResync,
                ErrorCategory::Recovery,
                message.clone(),
            ),
            Self::Timeout(message) => (ErrorCode::Timeout, ErrorCategory::Availability, message.clone()),
            Self::InternalInvariant(message) => (
                ErrorCode::InternalInvariant,
                ErrorCategory::Internal,
                message.clone(),
            ),
        };
        DxbotError {
            code,
            category,
            message,
            retryable: false,
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

#[derive(Debug, Clone, Default)]
pub struct SecurityState {
    pub principals: PrincipalManager,
    pub approvals: ApprovalManager,
    pub authority: AuthorityManager,
}

impl SecurityState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn ensure_local_operator(&mut self, principal: &PrincipalRef) -> Result<(), ServerError> {
        if self.principals.resolve_principal(principal).is_err() {
            self.principals
                .register_principal(principal.clone())
                .map_err(|error| {
                    ServerError::InternalInvariant(format!(
                        "cannot register authenticated local principal: {error}"
                    ))
                })?;
        }
        self.authority
            .bind_global_authority(principal, LOCAL_OPERATOR_ROLE)
            .map_err(|error| {
                ServerError::InternalInvariant(format!(
                    "cannot grant authenticated local operator role: {error}"
                ))
            })
    }
}

#[derive(Debug)]
pub struct ControlServer {
    security: Arc<Mutex<SecurityState>>,
    application: Arc<ApplicationMutator>,
    providers: Mutex<ProviderHost>,
}

impl ControlServer {
    pub fn new(
        security: Arc<Mutex<SecurityState>>,
        application: Arc<ApplicationMutator>,
    ) -> Self {
        Self {
            security,
            application,
            providers: Mutex::new(ProviderHost::new()),
        }
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
        }
    }

    pub fn register_local_operator(&self, principal: &PrincipalRef) -> Result<(), ServerError> {
        self.security
            .lock()
            .map_err(|_| {
                ServerError::InternalInvariant("security state lock unavailable".to_owned())
            })?
            .ensure_local_operator(principal)
    }

    pub fn preflight(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
        raw_selector: Option<&Value>,
    ) -> Result<(CanonicalTarget, CasConditions), ServerError> {
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
        self.application.mutate(request).map_err(map_app_error)
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
        self.require_active_principal(authenticated_principal)?;
        self.application
            .lookup_binding(command_id, key)
            .map_err(map_app_error)
    }

    fn approval_preflight(
        &self,
        payload: &CommandPayload,
        raw_selector: Option<&Value>,
    ) -> Result<(CanonicalTarget, CasConditions), ServerError> {
        let selector = selector_value(raw_selector).or_else(|| match &payload.canonical_target {
            CanonicalTarget::Approval { id, .. } => Some(id.0.as_str()),
            _ => None,
        });
        let selector = selector.ok_or_else(|| {
            ServerError::Conflict(format!("{} requires an approval selector", payload.command_key))
        })?;
        let id = ApprovalId(strip_ref(selector, "approval:"));
        let security = self.security.lock().map_err(|_| {
            ServerError::InternalInvariant("security state lock unavailable".to_owned())
        })?;
        let record = security.approvals.get_approval(&id).map_err(map_security_error)?;
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
        let security = self.security.lock().map_err(|_| {
            ServerError::InternalInvariant("security state lock unavailable".to_owned())
        })?;
        match payload.command_key.as_str() {
            "approval-list" => page_approval_records(security.approvals.list_approvals(), payload),
            "approval-show" => {
                let CanonicalTarget::Approval { id, .. } = &payload.canonical_target else {
                    return Err(ServerError::Conflict(
                        "approval-show requires Approval target".to_owned(),
                    ));
                };
                let record = security.approvals.get_approval(id).map_err(map_security_error)?;
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
        let selector = selector_value(raw_selector).or_else(|| match &payload.canonical_target {
            CanonicalTarget::Provider { id, .. } => Some(id.0.as_str()),
            _ => None,
        });
        let selector = selector.ok_or_else(|| {
            ServerError::Conflict(format!("{} requires a provider selector", payload.command_key))
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
        let security = self.security.lock().map_err(|_| {
            ServerError::InternalInvariant("security state lock unavailable".to_owned())
        })?;
        let principal_state = active_principal(&security, authenticated_principal)?;
        let is_local_operator = security
            .authority
            .check_global_authority(&principal_state.ref_, LOCAL_OPERATOR_ROLE)
            .map_err(|_| ServerError::PermissionDenied("global authority check failed".to_owned()))?;
        if is_local_operator {
            return Ok(());
        }
        let scoped_authorized = match scope_for_target(target) {
            Some(scope) => security
                .authority
                .check_authority(&principal_state.ref_, &scope, OPERATION_ROLE)
                .map_err(|_| {
                    ServerError::PermissionDenied("scope authority check failed".to_owned())
                })?,
            None => false,
        };
        if scoped_authorized {
            Ok(())
        } else {
            Err(ServerError::PermissionDenied("not authorized".to_owned()))
        }
    }

    fn require_active_principal(&self, principal: &PrincipalRef) -> Result<(), ServerError> {
        let security = self.security.lock().map_err(|_| {
            ServerError::InternalInvariant("security state lock unavailable".to_owned())
        })?;
        active_principal(&security, principal).map(|_| ())
    }
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

fn page_approval_records(records: Vec<ApprovalRecord>, payload: &CommandPayload) -> Result<Value, ServerError> {
    let size = page_size(payload)?;
    let cursor = cursor(payload);
    let rows = records
        .into_iter()
        .map(|record| (record.id.0.clone(), approval_value(&record)))
        .collect::<Vec<_>>();
    Ok(page_rows(rows, size, cursor.as_deref()))
}

fn page_provider_records(records: Vec<ProviderInfo>, payload: &CommandPayload) -> Result<Value, ServerError> {
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
    let next_cursor = if has_more {
        rows.last().map(|(key, _)| key.clone())
    } else {
        None
    };
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
        "revision": record.revision,
        "state": format!("{:?}", record.state).to_ascii_lowercase(),
        "required_approvers": record.required_approvers.iter().map(|principal| principal.0.clone()).collect::<Vec<_>>(),
        "decisions": record.decisions.iter().map(|decision| json!({
            "by": decision.by.0,
            "decision": format!("{:?}", decision.decision).to_ascii_lowercase(),
        })).collect::<Vec<_>>(),
    })
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
    match payload.semantic_options.get("page_size") {
        Some(Value::String(value)) => value
            .parse::<usize>()
            .map(|value| value.clamp(1, MAX_PAGE_SIZE))
            .map_err(|_| ServerError::Conflict("page_size must be an integer".to_owned())),
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map(|value| value.clamp(1, MAX_PAGE_SIZE))
            .ok_or_else(|| ServerError::Conflict("page_size is invalid".to_owned())),
        None => Ok(DEFAULT_PAGE_SIZE),
        _ => Err(ServerError::Conflict("page_size is invalid".to_owned())),
    }
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

fn map_security_error(error: runtime_security::Error) -> ServerError {
    match error {
        runtime_security::Error::UnknownApproval(id) => {
            ServerError::NotFound(format!("approval {} not found", id.0))
        }
        runtime_security::Error::ApprovalAlreadyDecided(id) => {
            ServerError::Conflict(format!("approval {} is stale or already decided", id.0))
        }
        runtime_security::Error::InvalidApprover(principal) => {
            ServerError::PermissionDenied(format!("{} is not an approver", principal.0))
        }
        runtime_security::Error::DuplicatePrincipal(principal)
        | runtime_security::Error::UnknownPrincipal(principal) => {
            ServerError::PermissionDenied(format!("principal unavailable: {}", principal.0))
        }
    }
}

fn map_provider_error(error: HarnessError) -> ServerError {
    match error {
        HarnessError::ProviderNotFound { id } => {
            ServerError::NotFound(format!("provider {} not found", id.0))
        }
        HarnessError::ProviderUnavailable { id } => {
            ServerError::Conflict(format!("provider {} is unavailable", id.0))
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

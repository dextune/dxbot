//! Control endpoint server: trusted authentication boundary plus binding-preserving delegation.
//!
//! Every local operation first authenticates the peer. Client-supplied Principal
//! fields are consistency bindings only; they never create authority. Application
//! preflight/query/mutation all reuse the same authenticated boundary.

use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use application::mutation::AppError;
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::*;
use runtime_security::{ApprovalManager, AuthorityManager, PrincipalManager, PrincipalStatus};
use serde_json::Value;

const OPERATION_ROLE: &str = "mutator";
const LOCAL_OPERATOR_ROLE: &str = "operator";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    PermissionDenied(String),
    NotFound(String),
    Conflict(String),
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
}

impl ControlServer {
    pub fn new(
        security: Arc<Mutex<SecurityState>>,
        application: Arc<ApplicationMutator>,
    ) -> Self {
        Self {
            security,
            application,
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
        self.application
            .preflight(payload, raw_selector)
            .map_err(map_app_error)
    }

    pub fn query(
        &self,
        authenticated_principal: &PrincipalRef,
        payload: &CommandPayload,
    ) -> Result<Value, ServerError> {
        self.authorize_payload_identity(authenticated_principal, payload)?;
        self.require_local_operator_or_scope(authenticated_principal, &payload.canonical_target)?;
        self.application.query(payload).map_err(map_app_error)
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

fn map_app_error(error: AppError) -> ServerError {
    match error {
        AppError::PermissionDenied(message) => ServerError::PermissionDenied(message),
        AppError::NotFound(message) => ServerError::NotFound(message),
        AppError::Conflict(message) => ServerError::Conflict(message),
        AppError::GapDetected(_) => {
            ServerError::InternalInvariant("subscription gap detected at boundary".to_owned())
        }
        AppError::Timeout(_) => {
            ServerError::InternalInvariant("subscription timeout at boundary".to_owned())
        }
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

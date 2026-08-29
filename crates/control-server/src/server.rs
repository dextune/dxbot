//! Control endpoint server: trusted authentication boundary plus binding-preserving delegation.
//!
//! Every authorized [`OperationRequest`] is passed intact to the application
//! layer. The control boundary never regenerates or discards command,
//! idempotency, request-digest, or operation identity, and it never trusts a
//! client-supplied principal as authentication evidence.

use std::sync::{Arc, Mutex};

use application::ApplicationMutator;
use application::mutation::AppError;
use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::*;
use runtime_security::{ApprovalManager, AuthorityManager, PrincipalManager, PrincipalStatus};

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

    /// Register and explicitly grant the P0 local operator role to the
    /// server-authenticated Instance+UID principal. Repeated authentication by
    /// the same local UID is idempotent; arbitrary registered principals still
    /// receive no authority.
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

/// The mutator already owns synchronized domain state, so ControlServer keeps
/// only an `Arc` rather than adding a second mutex around it.
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

    /// Handle a mutation for a principal already derived by the trusted local
    /// transport boundary (for P0: InstanceId + authenticated OS UID).
    ///
    /// `payload.principal_ref` and `idempotency_key.principal_ref` are treated
    /// only as consistency bindings. They must match the authenticated
    /// principal; neither is authentication evidence.
    pub fn handle_request(
        &self,
        authenticated_principal: &PrincipalRef,
        request: &OperationRequest,
    ) -> Result<OperationResult, ServerError> {
        if request.payload.principal_ref != *authenticated_principal
            || request.idempotency_key.principal_ref != *authenticated_principal
        {
            return Err(ServerError::PermissionDenied(
                "request principal binding mismatch".to_string(),
            ));
        }

        let security = self.security.lock().map_err(|_| {
            ServerError::InternalInvariant("security state lock unavailable".to_string())
        })?;

        let principal_state = security
            .principals
            .resolve_principal(authenticated_principal)
            .map_err(|_| ServerError::PermissionDenied("unauthenticated principal".to_string()))?;
        if principal_state.status != PrincipalStatus::Active {
            return Err(ServerError::PermissionDenied(
                "principal is not active".to_string(),
            ));
        }

        let is_local_operator = security
            .authority
            .check_global_authority(&principal_state.ref_, LOCAL_OPERATOR_ROLE)
            .map_err(|_| ServerError::PermissionDenied("global authority check failed".to_owned()))?;

        let scoped_authorized = match scope_for_target(&request.payload.canonical_target) {
            Some(scope) => security
                .authority
                .check_authority(&principal_state.ref_, &scope, OPERATION_ROLE)
                .map_err(|_| {
                    ServerError::PermissionDenied("scope authority check failed".to_owned())
                })?,
            None => false,
        };

        if !is_local_operator && !scoped_authorized {
            return Err(ServerError::PermissionDenied("not authorized".to_string()));
        }

        drop(security);
        self.application.mutate(request).map_err(map_app_error)
    }
}

fn map_app_error(error: AppError) -> ServerError {
    match error {
        AppError::PermissionDenied(message) => ServerError::PermissionDenied(message),
        AppError::NotFound(message) => ServerError::NotFound(message),
        AppError::Conflict(message) => ServerError::Conflict(message),
        AppError::GapDetected(_) => {
            ServerError::InternalInvariant("subscription gap detected at boundary".to_string())
        }
        AppError::Timeout(_) => {
            ServerError::InternalInvariant("subscription timeout at boundary".to_string())
        }
        AppError::Internal(message) => ServerError::InternalInvariant(message),
    }
}

/// Project only targets whose P0 authority scope is unambiguous from the
/// canonical target itself. Instance-global local operator authority is checked
/// separately; unknown ownership never fabricates a fallback scope.
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

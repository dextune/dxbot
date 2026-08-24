//! Control endpoint server: trusted authentication boundary plus binding-preserving delegation.
//!
//! Every authorized [`OperationRequest`] is passed intact to the application
//! layer. The control boundary never regenerates or discards command,
//! idempotency, request-digest, or operation identity, and it never trusts a
//! client-supplied principal as authentication evidence.

use std::sync::{Arc, Mutex};

use application::mutation::AppError;
use application::ApplicationMutator;
use dxbot_core::types::*;
use runtime_security::{
    ApprovalManager, AuthorityManager, PrincipalManager, PrincipalStatus,
};

const OPERATION_ROLE: &str = "mutator";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    PermissionDenied(String),
    NotFound(String),
    Conflict(String),
    InternalInvariant(String),
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

        let scope = scope_for_target(&request.payload.canonical_target).ok_or_else(|| {
            ServerError::PermissionDenied(
                "target does not expose an authorizable P0 scope".to_string(),
            )
        })?;
        let authorized = security
            .authority
            .check_authority(&principal_state.ref_, &scope, OPERATION_ROLE)
            .map_err(|_| ServerError::PermissionDenied("authority check failed".to_string()))?;
        if !authorized {
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
/// canonical target itself. Unknown ownership fails closed instead of being
/// routed through a fabricated fallback scope.
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

//! Control endpoint server: the security boundary and fail-closed path.
//!
//! [`ControlServer`] sits in front of the application mutator. Every operation
//! request passes through an ordered security pipeline:
//!
//! 1. **Authenticate** the peer and resolve its [`PrincipalRef`].
//! 2. **Validate** the idempotency key is bound to the same principal scope.
//! 3. **Authorize** the requested operation against the resolved principal.
//! 4. **Enforce information-flow boundaries** — fail closed.
//! 5. **Delegate** to the application mutator.
//!
//! The boundary is fail-closed and non-disclosing: any security check that
//! fails returns [`ServerError::PermissionDenied`] — never an internal error —
//! and permission failures do not reveal whether a referenced target exists.

use std::sync::{Arc, Mutex};

use application::mutation::AppError;
use application::ApplicationMutator;
use dxbot_core::types::*;
use runtime_security::{ApprovalManager, AuthorityManager, PrincipalManager};

/// The single role required to mutate domain targets at the control boundary.
const OPERATION_ROLE: &str = "mutator";

/// Error surfaced by the control server boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerError {
    /// The operation was refused by the security boundary. Used for every
    /// authentication / authorization / information-flow failure (fail-closed).
    PermissionDenied(String),
    /// The operation was authorized but its target does not exist.
    NotFound(String),
    /// The operation was authorized but hit a CAS/conflict condition.
    Conflict(String),
    /// An internal invariant or state failure on the authorized path.
    InternalInvariant(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "control-server error: {self:?}")
    }
}

impl std::error::Error for ServerError {}

/// Security state bundled behind the control boundary.
#[derive(Debug, Clone, Default)]
pub struct SecurityState {
    /// Principal registry and lifecycle.
    pub principals: PrincipalManager,
    /// High-risk operation approvals and pending continuation.
    pub approvals: ApprovalManager,
    /// Principal-to-scope role bindings.
    pub authority: AuthorityManager,
}

impl SecurityState {
    /// Create an empty security state (deny-unknown).
    pub fn new() -> Self {
        Self::default()
    }
}

/// The control endpoint server enforcing the security boundary.
#[derive(Debug)]
pub struct ControlServer {
    security: Arc<Mutex<SecurityState>>,
    application: Arc<Mutex<ApplicationMutator>>,
}

impl ControlServer {
    /// Create a control server over the given security state and application
    /// mutator, both shared under `Arc<Mutex<_>>`.
    pub fn new(
        security: Arc<Mutex<SecurityState>>,
        application: Arc<Mutex<ApplicationMutator>>,
    ) -> Self {
        Self {
            security,
            application,
        }
    }

    /// Handle an operation request through the fail-closed security pipeline.
    pub fn handle_request(
        &self,
        request: &OperationRequest,
    ) -> Result<OperationResult, ServerError> {
        let security = self
            .security
            .lock()
            .map_err(|_| ServerError::InternalInvariant("security state lock unavailable".into()))?;

        // 1. Authenticate the peer and resolve the principal.
        let principal = &request.payload.principal_ref;
        let principal_state = security
            .principals
            .resolve_principal(principal)
            .map_err(|_| ServerError::PermissionDenied("unauthenticated principal".into()))?;

        // 2. Validate the idempotency key is scoped to the resolved principal.
        if request.idempotency_key.principal_ref != *principal {
            return Err(ServerError::PermissionDenied(
                "idempotency key principal scope mismatch".into(),
            ));
        }

        // 3. Authorize the requested operation.
        let scope = scope_for_target(&request.payload.canonical_target);
        let authorized = security
            .authority
            .check_authority(
                &principal_state.ref_,
                &scope,
                OPERATION_ROLE,
            )
            .map_err(|_| ServerError::PermissionDenied("authority check failed".into()))?;
        if !authorized {
            // 4. Information-flow boundary: fail closed. Non-disclosing: we do
            // not reach the mutator, so the request cannot reveal whether the
            // referenced target exists.
            return Err(ServerError::PermissionDenied("not authorized".into()));
        }

        // 5. Delegate to the application mutator (drop the security lock first
        // so the mutator's own state lock is the only one held during mutation).
        drop(security);

        self.application
            .lock()
            .map_err(|_| {
                ServerError::InternalInvariant("application mutator lock unavailable".into())
            })?
            .mutate(&request.payload)
            .map_err(map_app_error)
    }
}

/// Map an application-mutation error onto the boundary error surface.
fn map_app_error(error: AppError) -> ServerError {
    match error {
        AppError::PermissionDenied(msg) => ServerError::PermissionDenied(msg),
        AppError::NotFound(msg) => ServerError::NotFound(msg),
        AppError::Conflict(msg) => ServerError::Conflict(msg),
        AppError::GapDetected(_) => ServerError::InternalInvariant("subscription gap detected at boundary".into()),
        AppError::Timeout(_) => ServerError::InternalInvariant("subscription timeout at boundary".into()),
        AppError::Internal(msg) => ServerError::InternalInvariant(msg),
    }
}

/// Derive the authorization scope from a canonical target.
///
/// Non-mutating or uncategorized targets fall back to a scope that carries no
/// existing authority grant, preserving deny-unknown.
fn scope_for_target(target: &CanonicalTarget) -> ScopeSelector {
    match target {
        CanonicalTarget::Bot { id, .. } => {
            ScopeSelector::Bot(BotSelector::CanonicalId(id.clone()))
        }
        CanonicalTarget::Project { id, .. } => {
            ScopeSelector::Project(ProjectSelector::CanonicalId(id.clone()))
        }
        CanonicalTarget::Channel { id, .. } => {
            ScopeSelector::Channel(ChannelSelector::CanonicalId(id.clone()))
        }
        _ => ScopeSelector::Bot(BotSelector::ScopedExact("global-fallback".into())),
    }
}
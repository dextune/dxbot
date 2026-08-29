//! Live directive controls (`AT-CLI-010`): create canonical `task-cancel`,
//! `task-suspend`, `task-resume`, and `task-redirect` command payloads and
//! wait for a directive to be observed as applied.
//!
//! The CLI sits at the boundary and only projects into the canonical
//! [`CommandPayload`] surface. It never drives a Runtime directly. Because
//! there is no live Runtime in this crate, the "waits until applied" behaviour
//! is modelled as an observable applied-slot: a caller (a real Runtime
//! observer) records the applied observation, and
//! [`DirectiveController::wait_for_applied`] blocks on that slot until it is
//! observed or a *local* deadline elapses.
//!
//! A local timeout never issues an implicit cancel: `wait_for_applied` only
//! observes and returns; it never projects a cancellation of the underlying
//! Runtime operation.
//!
//! [`CommandPayload`]: dxbot_core::types::CommandPayload

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use dxbot_core::types::{
    CanonicalTarget, CasConditions, CommandPayload, ContentSource, InstanceId, PrincipalRef,
    TaskId, TaskSelector,
};
use serde_json::json;

/// The canonical instance id this projection targets by default.
const DEFAULT_INSTANCE_ID: &str = "default";
/// The canonical local principal used by the CLI projection.
const DEFAULT_PRINCIPAL: &str = "cli";
/// Poll interval while blocking for an applied observation.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Command keys produced by this module.
pub const CMD_TASK_CANCEL: &str = "task-cancel";
pub const CMD_TASK_SUSPEND: &str = "task-suspend";
pub const CMD_TASK_RESUME: &str = "task-resume";
pub const CMD_TASK_REDIRECT: &str = "task-redirect";

/// The outcome of waiting for a directive to be observed as applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaitResult {
    /// The directive was observed as applied at the given monitored time.
    Applied { at: i64 },
    /// The local wait deadline elapsed without an applied observation.
    Timeout { elapsed: Duration },
    /// The wait was interrupted (observation source lost). This is local only
    /// and never cancels the Runtime operation.
    Interrupted,
}

/// Errors produced by the directive projection and applied-wait.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveError {
    /// The supplied task selector is empty and cannot form a canonical target.
    InvalidSelector { selector: String },
    /// A redirect requires non-empty replacement content.
    ReplacementRequired,
    /// The applied-observation slot could not be observed.
    ObserverLost,
}

/// Projects live task directives onto the canonical [`CommandPayload`] surface
/// and blocks on an applied-observation slot (`AT-CLI-010`).
///
/// All directive builders are pure and deterministic. The sole non-deterministic
/// behaviour is [`Self::wait_for_applied`], which blocks on the observation
/// slot shared with a Runtime observer.
#[derive(Debug, Clone)]
pub struct DirectiveController {
    instance_id: InstanceId,
    principal_ref: PrincipalRef,
    /// Shared applied-observation slot, set by a Runtime observer (or test
    /// fixture) once the directive is observed as applied.
    applied_at: Arc<Mutex<Option<i64>>>,
}

impl PartialEq for DirectiveController {
    fn eq(&self, other: &Self) -> bool {
        self.instance_id == other.instance_id
            && self.principal_ref == other.principal_ref
            && Arc::ptr_eq(&self.applied_at, &other.applied_at)
    }
}

impl Default for DirectiveController {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectiveController {
    /// Creates a directive controller bound to the default boundary identity.
    pub fn new() -> Self {
        Self {
            instance_id: InstanceId(DEFAULT_INSTANCE_ID.to_string()),
            principal_ref: PrincipalRef(DEFAULT_PRINCIPAL.to_string()),
            applied_at: Arc::new(Mutex::new(None)),
        }
    }

    /// Creates a `task-cancel` directive on `task_selector`.
    ///
    /// `if_revision` and `if_execution_generation` are carried as CAS
    /// conditions so the Runtime can re-verify the claimed target. Raises
    /// [`DirectiveError::InvalidSelector`] for an empty selector.
    pub fn cancel_task(
        &self,
        task_selector: &str,
        if_revision: Option<i64>,
        if_execution_generation: Option<i64>,
        reason: Option<&str>,
    ) -> Result<CommandPayload, DirectiveError> {
        self.task_directive(
            CMD_TASK_CANCEL,
            task_selector,
            if_revision,
            if_execution_generation,
            None,
            reason.map(str::to_string),
        )
    }

    /// Creates a `task-suspend` directive on `task_selector`.
    pub fn suspend_task(
        &self,
        task_selector: &str,
        if_revision: Option<i64>,
        if_execution_generation: Option<i64>,
        reason: Option<&str>,
    ) -> Result<CommandPayload, DirectiveError> {
        self.task_directive(
            CMD_TASK_SUSPEND,
            task_selector,
            if_revision,
            if_execution_generation,
            None,
            reason.map(str::to_string),
        )
    }

    /// Creates a `task-resume` directive on `task_selector`.
    pub fn resume_task(
        &self,
        task_selector: &str,
        if_revision: Option<i64>,
    ) -> Result<CommandPayload, DirectiveError> {
        self.task_directive(
            CMD_TASK_RESUME,
            task_selector,
            if_revision,
            None,
            None,
            None,
        )
    }

    /// Creates a `task-redirect` directive on `task_selector`, replacing the
    /// task's specification with a new `replacement` content source.
    ///
    /// Raises [`DirectiveError::ReplacementRequired`] if the replacement is an
    /// empty text content source — a redirect must carry non-empty replacement
    /// content.
    pub fn redirect_task(
        &self,
        task_selector: &str,
        if_revision: Option<i64>,
        replacement: &ContentSource,
    ) -> Result<CommandPayload, DirectiveError> {
        if replacement_content_empty(replacement) {
            return Err(DirectiveError::ReplacementRequired);
        }
        self.task_directive(
            CMD_TASK_REDIRECT,
            task_selector,
            if_revision,
            None,
            Some(replacement),
            None,
        )
    }

    /// Records that the tracked directive was observed as applied.
    ///
    /// This is the observation side of the applied-wait: a Runtime observer (or
    /// the test/directive-observation fixture) calls this to make
    /// [`Self::wait_for_applied`] return [`WaitResult::Applied`]. Passing a
    /// lower time than a prior observation preserves the first observation
    /// (monotonic), so a late duplicate report cannot move the applied-at
    /// backwards.
    pub fn observe_applied(&self, at: i64) {
        if let Ok(mut guard) = self.applied_at.lock() {
            if guard.is_none_or(|prev| at < prev) {
                *guard = Some(at);
            }
        }
    }

    /// Blocks until the directive is observed as applied, the local `timeout`
    /// elapses, or the observation is interrupted.
    ///
    /// Semantics:
    /// - [`WaitResult::Applied`] once [`Self::observe_applied`] has been called.
    /// - [`WaitResult::Timeout`] when `timeout` elapses with no observation.
    /// - [`WaitResult::Interrupted`] when the observation slot is lost.
    ///
    /// A local timeout or interruption is observation-only and **never** issues
    /// an implicit cancel: no cancellation is projected as a side effect.
    pub fn wait_for_applied(&self, timeout: Duration) -> Result<WaitResult, DirectiveError> {
        let start = Instant::now();
        let deadline = start.checked_add(timeout).unwrap_or(start);
        loop {
            {
                let guard = match self.applied_at.lock() {
                    Ok(guard) => guard,
                    Err(_) => return Ok(WaitResult::Interrupted),
                };
                if let Some(at) = *guard {
                    return Ok(WaitResult::Applied { at });
                }
            }
            let now = Instant::now();
            if now >= deadline {
                return Ok(WaitResult::Timeout {
                    elapsed: start.elapsed(),
                });
            }
            let remaining = deadline.saturating_duration_since(now);
            std::thread::sleep(POLL_INTERVAL.min(remaining));
        }
    }

    /// Shared projection for all task directives.
    fn task_directive(
        &self,
        command_key: &str,
        task_selector: &str,
        if_revision: Option<i64>,
        if_execution_generation: Option<i64>,
        content: Option<&ContentSource>,
        reason: Option<String>,
    ) -> Result<CommandPayload, DirectiveError> {
        if task_selector.trim().is_empty() {
            return Err(DirectiveError::InvalidSelector {
                selector: task_selector.to_string(),
            });
        }
        let task_id = task_name(&parse_task_selector(task_selector));
        let cas = CasConditions {
            if_revision,
            if_generation: None,
            if_host_generation: None,
            if_execution_generation,
            if_source_revision: None,
            if_scope_revision: None,
            if_project_revision: None,
            if_channel_revision: None,
            if_membership_generation: None,
            if_proposal_revision: None,
            if_target_scope_revision: None,
            if_receipt_revision: None,
        };
        let target = CanonicalTarget::Task {
            id: TaskId(task_id),
            revision: if_revision.unwrap_or(0),
            execution_generation: if_execution_generation,
        };
        let mut opts = serde_json::Map::new();
        opts.insert("task".to_string(), json!(task_selector));
        if let Some(reason) = reason {
            opts.insert("reason".to_string(), json!(reason));
        }
        Ok(CommandPayload {
            command_key: command_key.to_string(),
            principal_ref: self.principal_ref.clone(),
            instance_id: self.instance_id.clone(),
            canonical_target: target,
            cas: Some(cas),
            content: content.cloned(),
            semantic_options: serde_json::Value::Object(opts),
        })
    }
}

/// True when a redirect content source carries no usable content. Only text
/// content can be empty; file/stdin/artifact sources always carry content.
fn replacement_content_empty(source: &ContentSource) -> bool {
    match source {
        ContentSource::Text { value } => value.trim().is_empty(),
        ContentSource::InputFile { .. }
        | ContentSource::Stdin
        | ContentSource::ArtifactRef { .. } => false,
    }
}

/// Parses a task selector string into a [`TaskSelector`]. A `task:`-prefixed
/// value is treated as a canonical id; anything else is a scoped-exact name.
fn parse_task_selector(selector: &str) -> TaskSelector {
    if let Some(rest) = selector.strip_prefix("task:") {
        TaskSelector::CanonicalId(TaskId(rest.to_string()))
    } else {
        TaskSelector::ScopedExact(selector.to_string())
    }
}

/// The underlying canonical id value of a task selector.
fn task_name(selector: &TaskSelector) -> String {
    match selector {
        TaskSelector::CanonicalId(id) => id.0.clone(),
        TaskSelector::ScopedExact(name) => name.clone(),
    }
}

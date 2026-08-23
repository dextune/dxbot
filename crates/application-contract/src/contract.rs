//! Generated contract: `CommandPayload`, `PreflightPlan` and target materialization.
//!
//! This is the wire-side projection (per `DXB-IFC-040`/`DXB-IFC-042`): it turns a
//! `CliInput` into a canonical `dxbot_core::types::CommandPayload`. Canonical target and
//! required CAS are materialized from the user selector; a `PreflightPlan` records which
//! revision/generation queries must run *before* the payload can be fully committed.

use dxbot_core::types::*;
use dxbot_core::DxbotError;

use crate::cli_input::CliInput;
use crate::util;

/// The canonical wire payload projected from a `CliInput`.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandPayload {
    pub inner: dxbot_core::types::CommandPayload,
}

/// The bounded preflight needed before a `CommandPayload` can be fully materialized.
#[derive(Debug, Clone, PartialEq)]
pub struct PreflightPlan {
    pub queries: Vec<String>,
    pub required_cas: CasConditions,
}

/// Resolved canonical target plus the CAS required to write it.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetMaterialization {
    pub canonical_target: CanonicalTarget,
    pub cas: CasConditions,
}

impl CommandPayload {
    /// Project the canonical `CommandPayload` from a parsed `CliInput`.
    pub fn from_cli_input(input: &CliInput) -> Result<CommandPayload, DxbotError> {
        let tm = TargetMaterialization::from_cli_input(input)?;
        Ok(CommandPayload {
            inner: dxbot_core::types::CommandPayload {
                command_key: input.command_key.clone(),
                principal_ref: input.principal_ref(),
                instance_id: input.instance_id(),
                canonical_target: tm.canonical_target,
                cas: Some(tm.cas),
                content: input.content.clone(),
                semantic_options: input.fields.clone(),
            },
        })
    }
}

impl TargetMaterialization {
    /// Resolve the canonical target and required CAS for a `CliInput`.
    ///
    /// Without a runtime we can only materialize canonically-addressable selectors
    /// directly; scoped-exact selectors or missing revisions surface as preflight queries
    /// via [`Self::plan_preflight`].
    pub fn from_cli_input(input: &CliInput) -> Result<TargetMaterialization, DxbotError> {
        let user_cas = input.cas.unwrap_or_else(util::empty_cas);
        let target = canonical_target(&input.command_key, input, &user_cas)?;
        Ok(TargetMaterialization {
            canonical_target: target,
            cas: user_cas,
        })
    }

    /// Describe the queries needed before this payload can be fully materialized.
    pub fn plan_preflight(&self, input: &CliInput) -> PreflightPlan {
        PreflightPlan {
            queries: preflight_queries(&input.command_key, input, &self.cas),
            required_cas: self.cas,
        }
    }
}

fn canonical_target(
    cmd: &str,
    input: &CliInput,
    cas: &CasConditions,
) -> Result<CanonicalTarget, DxbotError> {
    let id = input.selector_value().unwrap_or_default();
    let revision = cas.if_revision.unwrap_or(0);
    let generated = cas.if_generation.unwrap_or(0);

    let target = match cmd {
        // Created/listed namespaces address the instance directly.
        "runtime-start"
        | "runtime-status"
        | "runtime-stop-graceful"
        | "runtime-stop-host"
        | "runtime-doctor"
        | "version"
        | "bot-create"
        | "bot-list"
        | "project-list"
        | "provider-list"
        | "task-list" => CanonicalTarget::Instance(input.instance_id()),

        "bot-activate" | "bot-deactivate" | "bot-archive" | "bot-restore" | "bot-show" => {
            CanonicalTarget::Bot {
                id: BotId(id_for(inst(input), &id)),
                revision,
            }
        }
        "conversation-send" | "conversation-show" | "conversation-history" => {
            CanonicalTarget::Conversation {
                id: ConversationId(id_for(inst(input), &id)),
                revision,
            }
        }
        "thread-create" => CanonicalTarget::Conversation {
            id: ConversationId(id_for(inst(input), &id)),
            revision,
        },
        "thread-send" | "thread-show" | "thread-history" | "thread-branch" => {
            CanonicalTarget::Thread {
                id: ThreadId(id_for(inst(input), &id)),
                parent_id: None,
                revision,
            }
        }
        "task-submit" | "task-show" | "task-watch" | "task-cancel" | "task-suspend"
        | "task-resume" | "task-redirect" | "task-result" => CanonicalTarget::Task {
            id: TaskId(id_for(inst(input), &id)),
            revision,
            execution_generation: cas.if_execution_generation,
        },
        "memory-get" | "memory-history" | "memory-search" | "memory-propose"
        | "memory-promote" => CanonicalTarget::Memory {
            id: MemoryId(id_for(inst(input), &id)),
            revision,
        },
        "project-create" | "project-show" | "project-archive" | "project-restore"
        | "project-member-set" | "project-member-remove" | "project-member-list" => {
            CanonicalTarget::Project {
                id: ProjectId(id_for(inst(input), &id)),
                revision,
            }
        }
        "channel-create" | "channel-list" => CanonicalTarget::Project {
            id: ProjectId(id_for(inst(input), &id)),
            revision,
        },
        "channel-show" | "channel-member-set" | "channel-member-remove"
        | "channel-member-list" | "channel-send" | "channel-history" => CanonicalTarget::Channel {
            id: ChannelId(id_for(inst(input), &id)),
            project_id: ProjectId(".".to_string()),
            revision,
        },
        "process-show" | "process-watch" => CanonicalTarget::Process {
            id: ProcessId(id_for(inst(input), &id)),
        },
        "operation-show" | "operation-reconcile" => CanonicalTarget::Operation {
            operation_id: OperationId(id_for(inst(input), &id)),
            command_id: CommandId(".".to_string()),
        },
        "approval-list" | "approval-show" | "approval-approve" | "approval-deny" => {
            CanonicalTarget::Approval {
                id: ApprovalId(id_for(inst(input), &id)),
                revision,
            }
        }
        "provider-show" => CanonicalTarget::Provider {
            id: ProviderId(id_for(inst(input), &id)),
            generation: generated,
        },
        "side-effect-reconcile" => CanonicalTarget::SideEffect {
            id: id_for(inst(input), &id),
            revision,
        },
        other => {
            return Err(util::invariant(format!(
                "no canonical target defined for command '{other}'"
            )));
        }
    };
    Ok(target)
}

fn inst(input: &CliInput) -> String {
    input.instance_id().0
}

/// Use the selector value when provided, otherwise fall back to the instance id so the
/// target is always deterministically addressable during projection.
fn id_for(instance: String, id: &str) -> String {
    if id.is_empty() {
        instance
    } else {
        id.to_string()
    }
}

fn preflight_queries(cmd: &str, input: &CliInput, cas: &CasConditions) -> Vec<String> {
    let id = input.selector_value().unwrap_or_default();
    let mut queries = Vec::new();

    // Scoped/exact selectors must be resolved to a canonical id before commit.
    if !id.is_empty() && !is_canonical_like(&id) {
        queries.push(format!("resolve-target:{cmd} {id}"));
    }

    // Revision-guarded mutations need the "current" revision if the user did not supply one.
    if needs_revision(cmd) && cas.if_revision.is_none() {
        queries.push(format!("resolve-revision:{cmd} {id}"));
    }
    if matches!(cmd, "task-cancel" | "task-suspend") && cas.if_execution_generation.is_none() {
        queries.push(format!("resolve-execution-generation:{cmd} {id}"));
    }

    queries.sort();
    queries.dedup();
    queries
}

/// Heuristic: canonical ids are stable tokens; anything with a `/` scope separator or a
/// space is treated as a scoped-exact name that requires resolution.
fn is_canonical_like(id: &str) -> bool {
    !id.contains('/') && !id.contains(' ') && !id.is_empty()
}

fn needs_revision(cmd: &str) -> bool {
    matches!(
        cmd,
        "bot-activate"
            | "bot-deactivate"
            | "bot-archive"
            | "bot-restore"
            | "conversation-send"
            | "thread-send"
            | "thread-branch"
            | "task-submit"
            | "task-cancel"
            | "task-suspend"
            | "task-resume"
            | "task-redirect"
            | "memory-propose"
            | "memory-promote"
            | "project-archive"
            | "project-restore"
            | "project-member-set"
            | "project-member-remove"
            | "channel-create"
            | "channel-member-set"
            | "channel-member-remove"
            | "channel-send"
            | "operation-reconcile"
            | "approval-approve"
            | "approval-deny"
            | "side-effect-reconcile"
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn sl(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn projects_bot_command_payload() {
        let input = CliInput::parse(&sl(&["bot-show", "--bot", "bot-42"])).unwrap();
        let payload = CommandPayload::from_cli_input(&input).unwrap();
        assert_eq!(payload.inner.command_key, "bot-show");
        match &payload.inner.canonical_target {
            CanonicalTarget::Bot { id, .. } => assert_eq!(id.0, "bot-42"),
            other => panic!("expected Bot target, got {other:?}"),
        }
        assert!(payload.inner.cas.is_some());
    }

    #[test]
    fn scoped_selector_forces_preflight_query() {
        let input = CliInput::parse(&sl(&["task-cancel", "--task", "tasks/alpha"])).unwrap();
        let tm = TargetMaterialization::from_cli_input(&input).unwrap();
        let plan = tm.plan_preflight(&input);
        assert!(!plan.queries.is_empty());
        assert!(plan.queries.iter().any(|q| q.starts_with("resolve-target:")));
    }
}
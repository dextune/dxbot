//! `CliInput` to canonical payload / bounded preflight projection.
//!
//! This layer owns interface projection only. Scoped names may still require a
//! Runtime preflight; no local alias or profile hint becomes canonical identity.

use dxbot_core::DxbotError;
use dxbot_core::types::*;

use crate::cli_input::CliInput;
use crate::util;

#[derive(Debug, Clone, PartialEq)]
pub struct CommandPayload {
    pub inner: dxbot_core::types::CommandPayload,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreflightPlan {
    pub queries: Vec<String>,
    pub required_cas: CasConditions,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetMaterialization {
    pub canonical_target: CanonicalTarget,
    pub cas: CasConditions,
}

impl CommandPayload {
    pub fn from_cli_input(input: &CliInput) -> Result<CommandPayload, DxbotError> {
        let target = TargetMaterialization::from_cli_input(input)?;
        Ok(CommandPayload {
            inner: dxbot_core::types::CommandPayload {
                command_key: input.command_key.clone(),
                principal_ref: input.principal_ref(),
                instance_id: input.instance_id(),
                canonical_target: target.canonical_target,
                cas: Some(target.cas),
                content: input.content.clone(),
                semantic_options: input.fields.clone(),
            },
        })
    }
}

impl TargetMaterialization {
    pub fn from_cli_input(input: &CliInput) -> Result<TargetMaterialization, DxbotError> {
        let cas = input.cas.unwrap_or_else(util::empty_cas);
        Ok(TargetMaterialization {
            canonical_target: canonical_target(&input.command_key, input, &cas)?,
            cas,
        })
    }

    pub fn plan_preflight(&self, input: &CliInput) -> PreflightPlan {
        PreflightPlan {
            queries: preflight_queries(&input.command_key, input, &self.cas),
            required_cas: self.cas,
        }
    }
}

/// Server-side guard proving that a mutable payload crossed the preflight
/// boundary with every operation-owned CAS field materialized. A custom client
/// may not bypass optimistic concurrency by omitting guards that the CLI would
/// normally resolve automatically.
pub fn validate_materialized_cas(
    payload: &dxbot_core::types::CommandPayload,
) -> Result<(), DxbotError> {
    let cas = payload.cas.unwrap_or_else(util::empty_cas);
    if required_revision_missing(&payload.command_key, &cas) {
        return Err(util::input_error(format!(
            "{} is missing required materialized CAS",
            payload.command_key
        )));
    }
    if matches!(payload.command_key.as_str(), "task-cancel" | "task-suspend")
        && cas.if_execution_generation.is_none()
    {
        return Err(util::input_error(format!(
            "{} requires materialized execution generation",
            payload.command_key
        )));
    }
    if matches!(
        payload.command_key.as_str(),
        "project-member-remove" | "channel-member-remove"
    ) && cas.if_membership_generation.is_none()
    {
        return Err(util::input_error(format!(
            "{} requires materialized membership generation",
            payload.command_key
        )));
    }
    Ok(())
}

fn canonical_target(
    command: &str,
    input: &CliInput,
    cas: &CasConditions,
) -> Result<CanonicalTarget, DxbotError> {
    let selector = input.selector_value().unwrap_or_default();
    let revision = cas.if_revision.unwrap_or(0);
    let generation = cas.if_generation.unwrap_or(0);
    let target = match command {
        "runtime-start"
        | "runtime-status"
        | "runtime-stop-graceful"
        | "runtime-stop-host"
        | "runtime-doctor"
        | "version"
        | "bot-create"
        | "bot-list"
        | "project-create"
        | "project-list"
        | "provider-list" => CanonicalTarget::Instance(input.instance_id()),

        "approval-list" if !selector.is_empty() => {
            scope_target(&selector, cas.if_scope_revision.unwrap_or(0))
        }
        "approval-list" => CanonicalTarget::Instance(input.instance_id()),

        "bot-show" | "bot-activate" | "bot-deactivate" | "bot-archive" | "bot-restore" => {
            CanonicalTarget::Bot {
                id: BotId(selector_id(&selector)),
                revision,
            }
        }
        "conversation-show" | "conversation-send" | "conversation-history" | "thread-create" => {
            CanonicalTarget::Conversation {
                id: ConversationId(selector_id(&selector)),
                revision,
            }
        }
        "thread-list" => CanonicalTarget::Conversation {
            id: ConversationId(selector_id(&selector)),
            revision,
        },
        "thread-show" | "thread-send" | "thread-history" | "thread-branch" => {
            CanonicalTarget::Thread {
                id: ThreadId(selector_id(&selector)),
                parent_id: None,
                revision,
            }
        }
        "task-submit" | "task-list" => scope_target(&selector, cas.if_scope_revision.unwrap_or(0)),
        "task-show" | "task-watch" | "task-cancel" | "task-suspend" | "task-resume"
        | "task-redirect" | "task-result" => CanonicalTarget::Task {
            id: TaskId(selector_id(&selector)),
            revision,
            execution_generation: cas.if_execution_generation,
        },
        "memory-search" | "memory-propose" => {
            scope_target(&selector, cas.if_scope_revision.unwrap_or(0))
        }
        "memory-get" | "memory-history" | "memory-promote" => CanonicalTarget::Memory {
            id: MemoryId(selector_id(&selector)),
            revision: cas.if_proposal_revision.or(cas.if_revision).unwrap_or(0),
        },
        "project-show" | "project-archive" | "project-restore" | "project-member-list" => {
            CanonicalTarget::Project {
                id: ProjectId(selector_id(&selector)),
                revision: cas.if_project_revision.or(cas.if_revision).unwrap_or(0),
            }
        }
        "project-member-set" | "project-member-remove" => CanonicalTarget::Membership {
            scope: ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(selector_id(
                &selector,
            )))),
            member_bot: BotSelector::CanonicalId(BotId(field_string(input, "member_bot")?)),
        },
        "channel-create" | "channel-list" => CanonicalTarget::Project {
            id: ProjectId(selector_id(&selector)),
            revision: cas.if_project_revision.unwrap_or(0),
        },
        "channel-show" | "channel-send" | "channel-history" | "channel-member-list" => {
            CanonicalTarget::Channel {
                id: ChannelId(selector_id(&selector)),
                project_id: ProjectId(field_string_or(input, "project", ".")),
                revision: cas.if_channel_revision.or(cas.if_revision).unwrap_or(0),
            }
        }
        "channel-member-set" | "channel-member-remove" => CanonicalTarget::Membership {
            scope: ScopeSelector::Channel(ChannelSelector::CanonicalId(ChannelId(selector_id(
                &selector,
            )))),
            member_bot: BotSelector::CanonicalId(BotId(field_string(input, "member_bot")?)),
        },
        "process-show" | "process-watch" => CanonicalTarget::Process {
            id: ProcessId(selector_id(&selector)),
        },
        "operation-show" | "operation-reconcile" => CanonicalTarget::Operation {
            operation_id: OperationId(selector_id(&selector)),
            command_id: CommandId(field_string_or(input, "command_id", ".")),
        },
        "approval-show" | "approval-approve" | "approval-deny" => CanonicalTarget::Approval {
            id: ApprovalId(selector_id(&selector)),
            revision,
        },
        "provider-show" => CanonicalTarget::Provider {
            id: ProviderId(selector_id(&selector)),
            generation,
        },
        "side-effect-reconcile" => CanonicalTarget::SideEffect {
            id: selector_id(&selector),
            revision,
        },
        other => {
            return Err(util::invariant(format!(
                "no canonical target projection for command '{other}'"
            )));
        }
    };
    Ok(target)
}

fn scope_target(value: &str, revision: i64) -> CanonicalTarget {
    if let Some(id) = value.strip_prefix("project:") {
        CanonicalTarget::Project {
            id: ProjectId(id.to_owned()),
            revision,
        }
    } else if let Some(id) = value.strip_prefix("channel:") {
        CanonicalTarget::Channel {
            id: ChannelId(id.to_owned()),
            project_id: ProjectId(".".to_owned()),
            revision,
        }
    } else if let Some(id) = value.strip_prefix("bot:") {
        CanonicalTarget::Bot {
            id: BotId(id.to_owned()),
            revision,
        }
    } else {
        CanonicalTarget::Bot {
            id: BotId(value.to_owned()),
            revision,
        }
    }
}

fn selector_id(value: &str) -> String {
    [
        "bot:",
        "conversation:",
        "thread:",
        "task:",
        "project:",
        "channel:",
        "memory:",
        "operation:",
        "approval:",
        "provider:",
        "process:",
        "side-effect:",
    ]
    .iter()
    .find_map(|prefix| value.strip_prefix(prefix))
    .unwrap_or(value)
    .to_owned()
}

fn field_string(input: &CliInput, name: &str) -> Result<String, DxbotError> {
    input
        .fields
        .get(name)
        .and_then(ValueExt::as_string)
        .map(str::to_owned)
        .ok_or_else(|| util::input_error(format!("{} requires field '{name}'", input.command_key)))
}

fn field_string_or(input: &CliInput, name: &str, default: &str) -> String {
    input
        .fields
        .get(name)
        .and_then(ValueExt::as_string)
        .unwrap_or(default)
        .to_owned()
}

trait ValueExt {
    fn as_string(&self) -> Option<&str>;
}

impl ValueExt for serde_json::Value {
    fn as_string(&self) -> Option<&str> {
        self.as_str()
    }
}

fn preflight_queries(command: &str, input: &CliInput, cas: &CasConditions) -> Vec<String> {
    let selector = input.selector_value().unwrap_or_default();
    let mut queries = Vec::new();
    if !selector.is_empty() && !is_canonical_like(&selector) {
        queries.push(format!("resolve-target:{command}:{selector}"));
    }
    if required_revision_missing(command, cas) {
        queries.push(format!("resolve-revision:{command}:{selector}"));
    }
    if matches!(command, "task-cancel" | "task-suspend") && cas.if_execution_generation.is_none() {
        queries.push(format!("resolve-execution-generation:{command}:{selector}"));
    }
    if matches!(command, "project-member-remove" | "channel-member-remove")
        && cas.if_membership_generation.is_none()
    {
        queries.push(format!(
            "resolve-membership-generation:{command}:{selector}"
        ));
    }
    queries.sort();
    queries.dedup();
    queries
}

fn is_canonical_like(value: &str) -> bool {
    !value.is_empty() && !value.contains('/') && !value.contains(' ')
}

fn required_revision_missing(command: &str, cas: &CasConditions) -> bool {
    match command {
        "bot-activate"
        | "bot-deactivate"
        | "bot-archive"
        | "bot-restore"
        | "conversation-send"
        | "thread-create"
        | "thread-send"
        | "task-cancel"
        | "task-suspend"
        | "task-resume"
        | "task-redirect"
        | "approval-approve"
        | "approval-deny"
        | "side-effect-reconcile" => cas.if_revision.is_none(),
        "thread-branch" => cas.if_source_revision.is_none(),
        "task-submit" | "memory-propose" => cas.if_scope_revision.is_none(),
        "memory-promote" => {
            cas.if_proposal_revision.is_none() || cas.if_target_scope_revision.is_none()
        }
        "project-archive" | "project-restore" => cas.if_revision.is_none(),
        "project-member-set" | "project-member-remove" | "channel-create" => {
            cas.if_project_revision.is_none()
        }
        "channel-member-set" | "channel-member-remove" => cas.if_channel_revision.is_none(),
        "channel-send" => cas.if_revision.is_none(),
        "operation-reconcile" => cas.if_receipt_revision.is_none(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::parse_bound_input;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn project_create_targets_instance_not_unborn_project() {
        let input = parse_bound_input(&args(&[
            "project-create",
            "--name",
            "alpha",
            "--owner-bot",
            "bot-a",
        ]))
        .expect("input parses");
        let target = TargetMaterialization::from_cli_input(&input).expect("target projects");
        assert!(matches!(
            target.canonical_target,
            CanonicalTarget::Instance(_)
        ));
    }

    #[test]
    fn task_submit_named_owner_targets_owner_scope() {
        let input = parse_bound_input(&args(&[
            "task-submit",
            "--owner",
            "project:alpha",
            "--text",
            "x",
        ]))
        .expect("input parses");
        let target = TargetMaterialization::from_cli_input(&input).expect("target projects");
        assert!(matches!(
            target.canonical_target,
            CanonicalTarget::Project { .. }
        ));
    }

    #[test]
    fn scoped_selector_forces_preflight_query() {
        let input = parse_bound_input(&args(&["task-cancel", "--task", "tasks/alpha"]))
            .expect("input parses");
        let target = TargetMaterialization::from_cli_input(&input).expect("target projects");
        let plan = target.plan_preflight(&input);
        assert!(
            plan.queries
                .iter()
                .any(|query| query.starts_with("resolve-target:"))
        );
    }

    #[test]
    fn thread_create_requires_materialized_conversation_revision() {
        let payload = dxbot_core::types::CommandPayload {
            command_key: "thread-create".to_owned(),
            principal_ref: PrincipalRef("p".to_owned()),
            instance_id: InstanceId("i".to_owned()),
            canonical_target: CanonicalTarget::Conversation {
                id: ConversationId("c".to_owned()),
                revision: 1,
            },
            cas: Some(util::empty_cas()),
            content: None,
            semantic_options: serde_json::json!({}),
        };
        assert!(validate_materialized_cas(&payload).is_err());
    }

    #[test]
    fn member_remove_requires_generation_in_addition_to_scope_revision() {
        let mut cas = util::empty_cas();
        cas.if_project_revision = Some(1);
        let payload = dxbot_core::types::CommandPayload {
            command_key: "project-member-remove".to_owned(),
            principal_ref: PrincipalRef("p".to_owned()),
            instance_id: InstanceId("i".to_owned()),
            canonical_target: CanonicalTarget::Membership {
                scope: ScopeSelector::Project(ProjectSelector::CanonicalId(ProjectId(
                    "project-a".to_owned(),
                ))),
                member_bot: BotSelector::CanonicalId(BotId("bot-a".to_owned())),
            },
            cas: Some(cas),
            content: None,
            semantic_options: serde_json::json!({}),
        };
        assert!(validate_materialized_cas(&payload).is_err());
    }
}

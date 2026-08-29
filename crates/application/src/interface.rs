//! Headless Application preflight and bounded query owner used by local Control.
//!
//! This module reads canonical DomainState only. It resolves exact visible
//! selectors to canonical targets and materializes current CAS before Prepared;
//! query pages use stable canonical-id ordering and explicit local ceilings.

use dxbot_core::types::*;
use serde_json::{Value, json};

use crate::membership::MembershipRecord;
use crate::mutation::{AppError, ApplicationMutator};
use crate::query::scope_owner;
use crate::state::{ConversationOwner, DomainState, MemoryAssertionStatus, ProcessState};

pub const DEFAULT_PAGE_SIZE: usize = 50;
pub const MAX_PAGE_SIZE: usize = 1000;

impl ApplicationMutator {
    pub fn preflight(
        &self,
        payload: &CommandPayload,
        raw_selector: Option<&Value>,
    ) -> Result<(CanonicalTarget, CasConditions), AppError> {
        let state = self.snapshot()?;
        let raw = raw_selector
            .and_then(|selector| selector.get("value"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        let target = resolve_target(&state, payload, raw)?;
        let mut cas = payload.cas.unwrap_or_else(empty_cas);
        materialize_cas(&state, &payload.command_key, &target, &mut cas)?;
        Ok((target, cas))
    }

    pub fn query(&self, payload: &CommandPayload) -> Result<Value, AppError> {
        let state = self.snapshot()?;
        query_state(&state, payload)
    }
}

fn resolve_target(
    state: &DomainState,
    payload: &CommandPayload,
    raw: &str,
) -> Result<CanonicalTarget, AppError> {
    match &payload.canonical_target {
        CanonicalTarget::Instance(instance) => Ok(CanonicalTarget::Instance(instance.clone())),
        CanonicalTarget::Bot { id, revision } => {
            let bot = state
                .bots
                .get(id)
                .or_else(|| state.bots.values().find(|bot| bot.name == raw))
                .ok_or_else(|| AppError::NotFound(format!("bot not found: {raw}")))?;
            Ok(CanonicalTarget::Bot {
                id: bot.id.clone(),
                revision: *revision,
            })
        }
        CanonicalTarget::Conversation { id, revision } => {
            let conversation = state
                .conversations
                .get(id)
                .or_else(|| {
                    let bot = state
                        .bots
                        .get(&BotId(raw.to_owned()))
                        .or_else(|| state.bots.values().find(|bot| bot.name == raw))?;
                    state
                        .conversations
                        .get(&ConversationId(format!("{}:main", bot.id.0)))
                })
                .ok_or_else(|| AppError::NotFound(format!("conversation not found: {raw}")))?;
            Ok(CanonicalTarget::Conversation {
                id: conversation.id.clone(),
                revision: *revision,
            })
        }
        CanonicalTarget::Thread { id, parent_id, revision } => {
            let candidates = state
                .threads
                .values()
                .filter(|thread| thread.id == *id || thread.title == raw)
                .collect::<Vec<_>>();
            let thread = exactly_one(candidates, "thread", raw)?;
            Ok(CanonicalTarget::Thread {
                id: thread.id.clone(),
                parent_id: parent_id.clone().or_else(|| Some(thread.conversation_id.clone())),
                revision: *revision,
            })
        }
        CanonicalTarget::Task { id, revision, execution_generation } => {
            let task = state
                .tasks
                .get(id)
                .ok_or_else(|| AppError::NotFound(format!("task not found: {raw}")))?;
            Ok(CanonicalTarget::Task {
                id: task.id.clone(),
                revision: *revision,
                execution_generation: *execution_generation,
            })
        }
        CanonicalTarget::Project { id, revision } => {
            let project = state
                .projects
                .get(id)
                .or_else(|| state.projects.values().find(|project| project.name == raw))
                .ok_or_else(|| AppError::NotFound(format!("project not found: {raw}")))?;
            Ok(CanonicalTarget::Project {
                id: project.id.clone(),
                revision: *revision,
            })
        }
        CanonicalTarget::Channel { id, project_id, revision } => {
            let candidates = state
                .channels
                .values()
                .filter(|channel| {
                    channel.id == *id
                        || channel.name == raw
                        || format!("{}/{}", channel.project_id.0, channel.name) == raw
                })
                .collect::<Vec<_>>();
            let channel = exactly_one(candidates, "channel", raw)?;
            Ok(CanonicalTarget::Channel {
                id: channel.id.clone(),
                project_id: if project_id.0 == "." {
                    channel.project_id.clone()
                } else {
                    project_id.clone()
                },
                revision: *revision,
            })
        }
        CanonicalTarget::Memory { id, revision } => {
            let memory = state
                .memories
                .get(id)
                .ok_or_else(|| AppError::NotFound(format!("memory not found: {raw}")))?;
            Ok(CanonicalTarget::Memory {
                id: memory.id.clone(),
                revision: *revision,
            })
        }
        CanonicalTarget::Operation { operation_id, command_id } => {
            if let Some(result) = state.results.get(operation_id) {
                return Ok(CanonicalTarget::Operation {
                    operation_id: result.operation_id.clone(),
                    command_id: result.command_id.clone(),
                });
            }
            if let Some(operation_id) = state.command_bindings.get(&CommandId(raw.to_owned())) {
                return Ok(CanonicalTarget::Operation {
                    operation_id: operation_id.clone(),
                    command_id: CommandId(raw.to_owned()),
                });
            }
            Ok(CanonicalTarget::Operation {
                operation_id: operation_id.clone(),
                command_id: command_id.clone(),
            })
        }
        CanonicalTarget::Membership { scope, member_bot } => Ok(CanonicalTarget::Membership {
            scope: resolve_scope(state, scope)?,
            member_bot: resolve_bot_selector(state, member_bot)?,
        }),
        CanonicalTarget::SideEffect { id, revision } => Ok(CanonicalTarget::SideEffect {
            id: id.clone(),
            revision: *revision,
        }),
        CanonicalTarget::Process { id } => {
            let row = state
                .processes
                .get(id)
                .or_else(|| state.processes.get(&ProcessId(raw.to_owned())))
                .ok_or_else(|| AppError::NotFound(format!("process not found: {raw}")))?;
            Ok(CanonicalTarget::Process { id: row.id.clone() })
        }
        CanonicalTarget::Approval { .. } | CanonicalTarget::Provider { .. } => {
            Ok(payload.canonical_target.clone())
        }
    }
}

fn resolve_scope(state: &DomainState, scope: &ScopeSelector) -> Result<ScopeSelector, AppError> {
    match scope {
        ScopeSelector::Bot(selector) => Ok(ScopeSelector::Bot(resolve_bot_selector(state, selector)?)),
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => {
            let row = state
                .projects
                .get(id)
                .or_else(|| state.projects.values().find(|project| project.name == id.0))
                .ok_or_else(|| AppError::NotFound(format!("project not found: {}", id.0)))?;
            Ok(ScopeSelector::Project(ProjectSelector::CanonicalId(row.id.clone())))
        }
        ScopeSelector::Project(ProjectSelector::VisibleExact(name)) => {
            let row = state
                .projects
                .values()
                .find(|project| project.name == *name)
                .ok_or_else(|| AppError::NotFound(format!("project not found: {name}")))?;
            Ok(ScopeSelector::Project(ProjectSelector::CanonicalId(row.id.clone())))
        }
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => {
            let row = state
                .channels
                .get(id)
                .ok_or_else(|| AppError::NotFound(format!("channel not found: {}", id.0)))?;
            Ok(ScopeSelector::Channel(ChannelSelector::CanonicalId(row.id.clone())))
        }
        ScopeSelector::Channel(ChannelSelector::ProjectExact { project, name }) => {
            let candidates = state
                .channels
                .values()
                .filter(|channel| {
                    channel.name == *name
                        && (channel.project_id.0 == *project
                            || state
                                .projects
                                .get(&channel.project_id)
                                .is_some_and(|row| row.name == *project))
                })
                .collect::<Vec<_>>();
            let row = exactly_one(candidates, "channel", &format!("{project}/{name}"))?;
            Ok(ScopeSelector::Channel(ChannelSelector::CanonicalId(row.id.clone())))
        }
    }
}

fn resolve_bot_selector(state: &DomainState, selector: &BotSelector) -> Result<BotSelector, AppError> {
    match selector {
        BotSelector::CanonicalId(id) => {
            let bot = state
                .bots
                .get(id)
                .or_else(|| state.bots.values().find(|bot| bot.name == id.0))
                .ok_or_else(|| AppError::NotFound(format!("bot not found: {}", id.0)))?;
            Ok(BotSelector::CanonicalId(bot.id.clone()))
        }
        BotSelector::ScopedExact(name) => {
            let bot = state
                .bots
                .values()
                .find(|bot| bot.name == *name)
                .ok_or_else(|| AppError::NotFound(format!("bot not found: {name}")))?;
            Ok(BotSelector::CanonicalId(bot.id.clone()))
        }
    }
}

fn materialize_cas(
    state: &DomainState,
    command: &str,
    target: &CanonicalTarget,
    cas: &mut CasConditions,
) -> Result<(), AppError> {
    match target {
        CanonicalTarget::Bot { id, .. } => {
            let revision = state.bots.get(id).map(|row| row.revision).unwrap_or(0);
            if matches!(command, "task-submit" | "memory-propose") {
                cas.if_scope_revision.get_or_insert(revision);
            } else {
                cas.if_revision.get_or_insert(revision);
            }
        }
        CanonicalTarget::Conversation { id, .. } => {
            let revision = state.conversations.get(id).map(|row| row.revision).unwrap_or(0);
            cas.if_revision.get_or_insert(revision);
        }
        CanonicalTarget::Thread { id, .. } => {
            let revision = state.threads.get(id).map(|row| row.revision).unwrap_or(0);
            if command == "thread-branch" {
                cas.if_source_revision.get_or_insert(revision);
            } else {
                cas.if_revision.get_or_insert(revision);
            }
        }
        CanonicalTarget::Task { id, .. } => {
            let task = state
                .tasks
                .get(id)
                .ok_or_else(|| AppError::NotFound(format!("task not found: {}", id.0)))?;
            cas.if_revision.get_or_insert(task.revision);
            if matches!(command, "task-cancel" | "task-suspend") {
                cas.if_execution_generation.get_or_insert(task.execution_generation);
            }
        }
        CanonicalTarget::Project { id, .. } => {
            let revision = state.projects.get(id).map(|row| row.revision).unwrap_or(0);
            if matches!(command, "task-submit" | "memory-propose") {
                cas.if_scope_revision.get_or_insert(revision);
            } else if matches!(command, "channel-create" | "project-member-set" | "project-member-remove") {
                cas.if_project_revision.get_or_insert(revision);
            } else {
                cas.if_revision.get_or_insert(revision);
            }
        }
        CanonicalTarget::Channel { id, .. } => {
            let channel = state
                .channels
                .get(id)
                .ok_or_else(|| AppError::NotFound(format!("channel not found: {}", id.0)))?;
            if matches!(command, "task-submit" | "memory-propose") {
                cas.if_scope_revision.get_or_insert(channel.revision);
            } else if matches!(command, "channel-member-set" | "channel-member-remove") {
                cas.if_channel_revision.get_or_insert(channel.revision);
            } else if command == "channel-send" {
                let revision = state
                    .conversations
                    .get(&channel.conversation_id)
                    .map(|row| row.revision)
                    .unwrap_or(0);
                cas.if_revision.get_or_insert(revision);
            }
        }
        CanonicalTarget::Memory { id, .. } => {
            let revision = state.memories.get(id).map(|row| row.revision).unwrap_or(0);
            if command == "memory-promote" {
                cas.if_proposal_revision.get_or_insert(revision);
            } else {
                cas.if_revision.get_or_insert(revision);
            }
        }
        CanonicalTarget::Membership { scope, member_bot } => {
            let key = membership_key(scope, member_bot);
            if let Some(row) = state.memberships.get(&key) {
                cas.if_membership_generation.get_or_insert(row.generation);
            }
            let revision = scope_revision(state, scope)?;
            match scope {
                ScopeSelector::Project(_) => {
                    cas.if_project_revision.get_or_insert(revision);
                }
                ScopeSelector::Channel(_) => {
                    cas.if_channel_revision.get_or_insert(revision);
                }
                ScopeSelector::Bot(_) => {
                    cas.if_scope_revision.get_or_insert(revision);
                }
            }
        }
        CanonicalTarget::Operation { operation_id, .. } => {
            if let Some(result) = state.results.get(operation_id) {
                cas.if_receipt_revision.get_or_insert(result.receipt.last_progress);
            }
        }
        CanonicalTarget::SideEffect { id, .. } => {
            if let Some(row) = state.side_effects.get(id) {
                cas.if_revision.get_or_insert(row.revision);
            }
        }
        _ => {}
    }
    Ok(())
}

fn query_state(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let page_size = page_size(payload)?;
    let cursor = optional_string(payload, "cursor");
    match payload.command_key.as_str() {
        "bot-list" => page_values(state.bots.values().map(|row| (row.id.0.clone(), json!({"bot_ref": format!("bot:{}", row.id.0), "name": row.name, "revision": row.revision, "lifecycle": format!("{:?}", row.lifecycle).to_lowercase()}))).collect(), page_size, cursor.as_deref()),
        "bot-show" => {
            let CanonicalTarget::Bot { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
            let row = state.bots.get(id).ok_or_else(|| AppError::NotFound(format!("bot {} does not exist", id.0)))?;
            Ok(json!({"bot_ref": format!("bot:{}", id.0), "name": row.name, "revision": row.revision, "lifecycle": format!("{:?}", row.lifecycle).to_lowercase()}))
        }
        "conversation-show" => conversation_value(state, payload),
        "conversation-history" => message_page_for_conversation(state, payload, page_size, cursor.as_deref()),
        "thread-list" => {
            let CanonicalTarget::Conversation { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
            page_values(state.threads.values().filter(|row| row.conversation_id == *id).map(|row| (row.id.0.clone(), json!({"thread_ref": format!("thread:{}", row.id.0), "conversation_ref": format!("conversation:{}", row.conversation_id.0), "revision": row.revision, "title": row.title}))).collect(), page_size, cursor.as_deref())
        }
        "thread-show" => {
            let CanonicalTarget::Thread { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
            let row = state.threads.get(id).ok_or_else(|| AppError::NotFound(format!("thread {} does not exist", id.0)))?;
            Ok(json!({"thread_ref": format!("thread:{}", id.0), "conversation_ref": format!("conversation:{}", row.conversation_id.0), "revision": row.revision, "title": row.title, "parent_message_ref": row.parent_message_id.as_ref().map(|id| format!("message:{}", id.0))}))
        }
        "thread-history" => message_page_for_thread(state, payload, page_size, cursor.as_deref()),
        "task-list" => {
            let scope = scope_from_target(&payload.canonical_target)?;
            let owner = scope_owner(&scope);
            page_values(state.tasks.values().filter(|row| row.owner == owner).map(|row| (row.id.0.clone(), task_value(row))).collect(), page_size, cursor.as_deref())
        }
        "task-show" => task_query(state, payload),
        "task-result" => {
            let CanonicalTarget::Task { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
            let row = state.tasks.get(id).ok_or_else(|| AppError::NotFound(format!("task {} does not exist", id.0)))?;
            Ok(json!({"task_ref": format!("task:{}", id.0), "state": format!("{:?}", row.status).to_lowercase(), "result": row.result}))
        }
        "memory-get" => memory_query(state, payload),
        "memory-search" => memory_search(state, payload, page_size, cursor.as_deref()),
        "memory-history" => memory_history(state, payload, page_size, cursor.as_deref()),
        "project-list" => page_values(state.projects.values().map(|row| (row.id.0.clone(), json!({"project_ref": format!("project:{}", row.id.0), "name": row.name, "revision": row.revision, "lifecycle": format!("{:?}", row.lifecycle).to_lowercase(), "owner_bot_ref": format!("bot:{}", row.owner_bot.0)}))).collect(), page_size, cursor.as_deref()),
        "project-show" => project_query(state, payload),
        "project-member-list" | "channel-member-list" => membership_page(state, payload, page_size, cursor.as_deref()),
        "channel-list" => {
            let CanonicalTarget::Project { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
            page_values(state.channels.values().filter(|row| row.project_id == *id).map(|row| (row.id.0.clone(), channel_value(row))).collect(), page_size, cursor.as_deref())
        }
        "channel-show" => channel_query(state, payload),
        "channel-history" => channel_history(state, payload, page_size, cursor.as_deref()),
        "process-show" => process_query(state, payload),
        "operation-show" => operation_query(state, payload),
        other => Err(AppError::NotFound(format!("no Application query owner for command {other}"))),
    }
}

fn conversation_value(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Conversation { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.conversations.get(id).ok_or_else(|| AppError::NotFound(format!("conversation {} does not exist", id.0)))?;
    let owner = match &row.owner { ConversationOwner::Bot { bot_id } => json!({"kind":"bot","ref":format!("bot:{}",bot_id.0)}), ConversationOwner::Channel { project_id, channel_id } => json!({"kind":"channel","project_ref":format!("project:{}",project_id.0),"channel_ref":format!("channel:{}",channel_id.0)}) };
    Ok(json!({"conversation_ref": format!("conversation:{}", id.0), "revision": row.revision, "owner": owner, "message_count": row.messages.len()}))
}

fn message_page_for_conversation(state: &DomainState, payload: &CommandPayload, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    let CanonicalTarget::Conversation { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.conversations.get(id).ok_or_else(|| AppError::NotFound(format!("conversation {} does not exist", id.0)))?;
    message_page(state, &row.messages, page_size, cursor)
}

fn message_page_for_thread(state: &DomainState, payload: &CommandPayload, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    let CanonicalTarget::Thread { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.threads.get(id).ok_or_else(|| AppError::NotFound(format!("thread {} does not exist", id.0)))?;
    message_page(state, &row.messages, page_size, cursor)
}

fn message_page(state: &DomainState, ids: &[MessageId], page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    page_values(ids.iter().filter_map(|id| state.messages.get(id)).map(|row| (row.id.0.clone(), json!({"message_ref": format!("message:{}", row.id.0), "sequence": row.sequence, "content": row.content}))).collect(), page_size, cursor)
}

fn task_query(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Task { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.tasks.get(id).ok_or_else(|| AppError::NotFound(format!("task {} does not exist", id.0)))?;
    Ok(task_value(row))
}
fn task_value(row: &crate::state::TaskState) -> Value { json!({"task_ref": format!("task:{}", row.id.0), "owner": row.owner, "revision": row.revision, "execution_generation": row.execution_generation, "state": format!("{:?}", row.status).to_lowercase()}) }

fn process_query(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Process { id } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.processes.get(id).ok_or_else(|| AppError::NotFound(format!("process {} does not exist", id.0)))?;
    Ok(process_value(row))
}

fn process_value(row: &ProcessState) -> Value {
    json!({
        "process_ref": format!("process:{}", row.id.0),
        "definition_ref": row.definition_id,
        "definition_version": row.definition_version,
        "revision": row.revision,
        "scope_ref": row.scope_ref,
        "initiator_ref": row.initiator_ref,
        "state": format!("{:?}", row.lifecycle).to_ascii_lowercase(),
        "current_step_ref": row.current_step_ref,
        "waiting_condition_ref": row.waiting_condition_ref,
        "child_refs": row.child_refs,
        "progress": row.progress,
        "terminal_reason": row.terminal_reason,
    })
}

fn memory_query(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Memory { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.memories.get(id).ok_or_else(|| AppError::NotFound(format!("memory {} does not exist", id.0)))?;
    Ok(memory_value(row))
}
fn memory_value(row: &crate::state::MemoryState) -> Value { json!({"memory_ref": format!("memory:{}", row.id.0), "scope": row.scope_key, "revision": row.revision, "state": format!("{:?}", row.status).to_lowercase(), "statement": row.statement, "evidence": row.evidence}) }

fn memory_search(state: &DomainState, payload: &CommandPayload, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    let scope = scope_from_target(&payload.canonical_target)?;
    let owner = scope_owner(&scope);
    let query = required_string(payload, "query")?.to_ascii_lowercase();
    page_values(state.memories.values().filter(|row| row.scope_key == owner && row.status == MemoryAssertionStatus::Accepted && content_contains(&row.statement, &query)).map(|row| (row.id.0.clone(), memory_value(row))).collect(), page_size, cursor)
}

fn memory_history(state: &DomainState, payload: &CommandPayload, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    let CanonicalTarget::Memory { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.memories.get(id).ok_or_else(|| AppError::NotFound(format!("memory {} does not exist", id.0)))?;
    page_values(row.history.iter().map(|revision| (format!("{:020}", revision.revision), json!({"revision": revision.revision, "state": format!("{:?}", revision.status).to_lowercase(), "statement": revision.statement, "evidence": revision.evidence}))).collect(), page_size, cursor)
}

fn project_query(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Project { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.projects.get(id).ok_or_else(|| AppError::NotFound(format!("project {} does not exist", id.0)))?;
    Ok(json!({"project_ref": format!("project:{}", row.id.0), "name": row.name, "revision": row.revision, "lifecycle": format!("{:?}", row.lifecycle).to_lowercase(), "owner_bot_ref": format!("bot:{}", row.owner_bot.0)}))
}

fn channel_query(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Channel { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.channels.get(id).ok_or_else(|| AppError::NotFound(format!("channel {} does not exist", id.0)))?;
    Ok(channel_value(row))
}
fn channel_value(row: &crate::state::ChannelState) -> Value { json!({"channel_ref": format!("channel:{}", row.id.0), "project_ref": format!("project:{}", row.project_id.0), "name": row.name, "revision": row.revision, "conversation_ref": format!("conversation:{}", row.conversation_id.0)}) }

fn channel_history(state: &DomainState, payload: &CommandPayload, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    let CanonicalTarget::Channel { id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.channels.get(id).ok_or_else(|| AppError::NotFound(format!("channel {} does not exist", id.0)))?;
    let conversation = state.conversations.get(&row.conversation_id).ok_or_else(|| AppError::Internal("channel conversation missing".to_owned()))?;
    message_page(state, &conversation.messages, page_size, cursor)
}

fn membership_page(state: &DomainState, payload: &CommandPayload, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    let scope = match &payload.canonical_target {
        CanonicalTarget::Project { id, .. } => ScopeSelector::Project(ProjectSelector::CanonicalId(id.clone())),
        CanonicalTarget::Channel { id, .. } => ScopeSelector::Channel(ChannelSelector::CanonicalId(id.clone())),
        _ => return Err(target_error(payload)),
    };
    page_values(state.memberships.values().filter(|row| row.scope == scope).map(|row| (row.id.clone(), membership_value(row))).collect(), page_size, cursor)
}
fn membership_value(row: &MembershipRecord) -> Value { json!({"member_bot": row.member_bot, "role": row.role, "generation": row.generation}) }

fn operation_query(state: &DomainState, payload: &CommandPayload) -> Result<Value, AppError> {
    let CanonicalTarget::Operation { operation_id, .. } = &payload.canonical_target else { return Err(target_error(payload)); };
    let row = state.results.get(operation_id).ok_or_else(|| AppError::NotFound(format!("operation {} does not exist", operation_id.0)))?;
    serde_json::to_value(row).map_err(|error| AppError::Internal(format!("cannot encode operation result: {error}")))
}

fn page_values(mut rows: Vec<(String, Value)>, page_size: usize, cursor: Option<&str>) -> Result<Value, AppError> {
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    if let Some(cursor) = cursor { rows.retain(|(key, _)| key.as_str() > cursor); }
    let has_more = rows.len() > page_size;
    let rows = rows.into_iter().take(page_size).collect::<Vec<_>>();
    let next_cursor = if has_more { rows.last().map(|(key, _)| key.clone()) } else { None };
    Ok(json!({"items": rows.into_iter().map(|(_, value)| value).collect::<Vec<_>>(), "next_cursor": next_cursor, "has_more": has_more}))
}

fn page_size(payload: &CommandPayload) -> Result<usize, AppError> {
    match payload.semantic_options.get("page_size") {
        Some(Value::String(value)) => value.parse::<usize>().map_err(|_| AppError::Conflict("page_size must be an integer".to_owned())).map(|size| size.clamp(1, MAX_PAGE_SIZE)),
        Some(Value::Number(value)) => value.as_u64().and_then(|value| usize::try_from(value).ok()).ok_or_else(|| AppError::Conflict("page_size is invalid".to_owned())).map(|size| size.clamp(1, MAX_PAGE_SIZE)),
        None => Ok(DEFAULT_PAGE_SIZE),
        _ => Err(AppError::Conflict("page_size is invalid".to_owned())),
    }
}
fn optional_string(payload: &CommandPayload, name: &str) -> Option<String> { payload.semantic_options.get(name).and_then(Value::as_str).map(str::to_owned) }
fn required_string<'a>(payload: &'a CommandPayload, name: &str) -> Result<&'a str, AppError> { payload.semantic_options.get(name).and_then(Value::as_str).ok_or_else(|| AppError::Conflict(format!("{} requires field '{name}'", payload.command_key))) }
fn content_contains(content: &ContentSource, query: &str) -> bool { match content { ContentSource::Text { value } => value.to_ascii_lowercase().contains(query), ContentSource::ArtifactRef { artifact_id, digest } => artifact_id.to_ascii_lowercase().contains(query) || digest.to_ascii_lowercase().contains(query), ContentSource::InputFile { .. } | ContentSource::Stdin => false } }
fn exactly_one<'a, T>(values: Vec<&'a T>, kind: &str, selector: &str) -> Result<&'a T, AppError> { match values.as_slice() { [one] => Ok(*one), [] => Err(AppError::NotFound(format!("{kind} not found: {selector}"))), _ => Err(AppError::Conflict(format!("ambiguous {kind} selector: {selector}"))) } }
fn target_error(payload: &CommandPayload) -> AppError { AppError::Conflict(format!("invalid target for {}: {:?}", payload.command_key, payload.canonical_target)) }

fn scope_from_target(target: &CanonicalTarget) -> Result<ScopeSelector, AppError> { match target { CanonicalTarget::Bot { id, .. } => Ok(ScopeSelector::Bot(BotSelector::CanonicalId(id.clone()))), CanonicalTarget::Project { id, .. } => Ok(ScopeSelector::Project(ProjectSelector::CanonicalId(id.clone()))), CanonicalTarget::Channel { id, .. } => Ok(ScopeSelector::Channel(ChannelSelector::CanonicalId(id.clone()))), _ => Err(AppError::Conflict("target is not a scope".to_owned())) } }
fn scope_revision(state: &DomainState, scope: &ScopeSelector) -> Result<i64, AppError> { match scope { ScopeSelector::Bot(BotSelector::CanonicalId(id)) => state.bots.get(id).map(|row| row.revision), ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => state.projects.get(id).map(|row| row.revision), ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => state.channels.get(id).map(|row| row.revision), _ => None }.ok_or_else(|| AppError::NotFound(format!("scope not found: {scope:?}"))) }
fn membership_key(scope: &ScopeSelector, bot: &BotSelector) -> String { format!("{}|{}", scope_owner(scope), match bot { BotSelector::CanonicalId(id) => id.0.clone(), BotSelector::ScopedExact(name) => format!("exact:{name}") }) }
fn empty_cas() -> CasConditions { CasConditions { if_revision: None, if_generation: None, if_host_generation: None, if_execution_generation: None, if_source_revision: None, if_scope_revision: None, if_project_revision: None, if_channel_revision: None, if_membership_generation: None, if_proposal_revision: None, if_target_scope_revision: None, if_receipt_revision: None } }

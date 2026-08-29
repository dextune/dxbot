//! Domain query: bounded pagination, cursor continuation, `--all` bounding and
//! resync over the canonical [`DomainState`].

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use dxbot_core::types::{
    BotId, BotSelector, ChannelSelector, ConversationId, ProjectSelector, ScopeSelector, TaskId,
    ThreadId,
};

use crate::mutation::AppError;
use crate::state::{
    BotState, ConversationOwner, ConversationState, DomainState, LifecycleState, TaskState,
    TaskStatus, ThreadState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotSummary {
    pub id: BotId,
    pub name: String,
    pub lifecycle: LifecycleState,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSummary {
    pub id: ConversationId,
    pub bot_id: BotId,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSummary {
    pub id: ThreadId,
    pub conversation_id: ConversationId,
    pub revision: i64,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSummary {
    pub id: TaskId,
    pub owner: String,
    pub status: TaskStatus,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AllLoopResult<T> {
    Complete {
        items: Vec<T>,
    },
    Partial {
        items: Vec<T>,
        next_cursor: Option<String>,
    },
}

#[derive(Debug)]
pub struct ApplicationQuery {
    state: Arc<Mutex<DomainState>>,
}

impl ApplicationQuery {
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    pub fn list_bots(
        &self,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<BotSummary>, AppError> {
        let guard = self.lock()?;
        Ok(paginate(
            guard.bots.values().cloned().collect(),
            |bot| bot.id.0.clone(),
            to_bot_summary,
            page_size,
            cursor.as_deref(),
        ))
    }

    pub fn list_conversations(
        &self,
        bot_id: &BotId,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<ConversationSummary>, AppError> {
        let guard = self.lock()?;
        let conversations = guard
            .conversations
            .values()
            .filter(|conversation| {
                matches!(
                    &conversation.owner,
                    ConversationOwner::Bot { bot_id: owner } if owner == bot_id
                )
            })
            .cloned()
            .collect();
        Ok(paginate(
            conversations,
            |conversation| conversation.id.0.clone(),
            to_conversation_summary,
            page_size,
            cursor.as_deref(),
        ))
    }

    pub fn list_threads(
        &self,
        conversation_id: &ConversationId,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<ThreadSummary>, AppError> {
        let guard = self.lock()?;
        let threads = guard
            .threads
            .values()
            .filter(|thread| thread.conversation_id == *conversation_id)
            .cloned()
            .collect();
        Ok(paginate(
            threads,
            |thread| thread.id.0.clone(),
            to_thread_summary,
            page_size,
            cursor.as_deref(),
        ))
    }

    pub fn list_tasks(
        &self,
        scope: &ScopeSelector,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<TaskSummary>, AppError> {
        let owner = scope_owner(scope);
        let guard = self.lock()?;
        let tasks = guard
            .tasks
            .values()
            .filter(|task| task.owner == owner)
            .cloned()
            .collect();
        Ok(paginate(
            tasks,
            |task| task.id.0.clone(),
            to_task_summary,
            page_size,
            cursor.as_deref(),
        ))
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

pub fn all_loop<T: Debug + Clone + PartialEq>(
    mut fetch: impl FnMut(usize, Option<String>) -> Result<Page<T>, AppError>,
    page_size: usize,
    local_ceiling: usize,
) -> Result<AllLoopResult<T>, AppError> {
    drain_all(&mut fetch, page_size, local_ceiling, None)
}

pub fn resync<T: Debug + Clone + PartialEq>(
    mut fetch: impl FnMut(usize, Option<String>) -> Result<Page<T>, AppError>,
    from_cursor: Option<String>,
    page_size: usize,
    local_ceiling: usize,
) -> Result<AllLoopResult<T>, AppError> {
    drain_all(&mut fetch, page_size, local_ceiling, from_cursor)
}

fn drain_all<T: Debug + Clone + PartialEq>(
    fetch: &mut impl FnMut(usize, Option<String>) -> Result<Page<T>, AppError>,
    page_size: usize,
    local_ceiling: usize,
    mut cursor: Option<String>,
) -> Result<AllLoopResult<T>, AppError> {
    let page_size = page_size.max(1);
    let mut items = Vec::with_capacity(local_ceiling.min(page_size));
    if local_ceiling == 0 {
        return Ok(AllLoopResult::Partial {
            items,
            next_cursor: cursor,
        });
    }

    loop {
        let remaining = local_ceiling.saturating_sub(items.len());
        if remaining == 0 {
            return Ok(AllLoopResult::Partial {
                items,
                next_cursor: cursor,
            });
        }
        let request_size = page_size.min(remaining);
        let request_cursor = cursor.clone();
        let page = fetch(request_size, request_cursor.clone())?;
        if page.items.len() > request_size {
            return Err(AppError::Internal(format!(
                "paged fetch exceeded requested bound: requested {request_size}, received {}",
                page.items.len()
            )));
        }
        if page.has_more && page.items.is_empty() {
            return Err(AppError::Internal(
                "paged fetch reported has_more without making item progress".to_owned(),
            ));
        }
        let has_more = page.has_more;
        let next_cursor = page.next_cursor;
        items.extend(page.items);
        if !has_more {
            return Ok(AllLoopResult::Complete { items });
        }
        let next_cursor = next_cursor.ok_or_else(|| {
            AppError::Internal("paged fetch reported has_more without next_cursor".to_owned())
        })?;
        if request_cursor.as_ref() == Some(&next_cursor) {
            return Err(AppError::Internal(
                "paged fetch returned a non-advancing cursor".to_owned(),
            ));
        }
        cursor = Some(next_cursor);
        if items.len() == local_ceiling {
            return Ok(AllLoopResult::Partial {
                items,
                next_cursor: cursor,
            });
        }
    }
}

pub(crate) fn scope_owner(scope: &ScopeSelector) -> String {
    match scope {
        ScopeSelector::Bot(BotSelector::CanonicalId(id)) => format!("bot:{}", id.0),
        ScopeSelector::Bot(BotSelector::ScopedExact(name)) => format!("bot:exact:{name}"),
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => format!("project:{}", id.0),
        ScopeSelector::Project(ProjectSelector::VisibleExact(name)) => {
            format!("project:exact:{name}")
        }
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => format!("channel:{}", id.0),
        ScopeSelector::Channel(ChannelSelector::ProjectExact { project, name }) => {
            format!("channel:{project}:{name}")
        }
    }
}

fn to_bot_summary(bot: BotState) -> BotSummary {
    BotSummary {
        id: bot.id,
        name: bot.name,
        lifecycle: bot.lifecycle,
        revision: bot.revision,
    }
}

fn to_conversation_summary(conversation: ConversationState) -> ConversationSummary {
    let ConversationOwner::Bot { bot_id } = conversation.owner else {
        unreachable!("list_conversations filters channel-owned conversations")
    };
    ConversationSummary {
        id: conversation.id,
        bot_id,
        revision: conversation.revision,
    }
}

fn to_thread_summary(thread: ThreadState) -> ThreadSummary {
    ThreadSummary {
        id: thread.id,
        conversation_id: thread.conversation_id,
        revision: thread.revision,
        title: thread.title,
    }
}

fn to_task_summary(task: TaskState) -> TaskSummary {
    TaskSummary {
        id: task.id,
        owner: task.owner,
        status: task.status,
        revision: task.revision,
    }
}

pub(crate) fn paginate<S, T>(
    mut source: Vec<S>,
    key_of: impl Fn(&S) -> String,
    map: impl Fn(S) -> T,
    page_size: usize,
    cursor: Option<&str>,
) -> Page<T> {
    let page_size = page_size.max(1);
    source.sort_by_key(|item| key_of(item));
    if let Some(cursor) = cursor {
        source.retain(|item| key_of(item).as_str() > cursor);
    }
    let has_more = source.len() > page_size;
    let next_cursor = if has_more {
        source.get(page_size - 1).map(&key_of)
    } else {
        None
    };
    let items = source.into_iter().take(page_size).map(map).collect();
    Page {
        items,
        next_cursor,
        has_more,
    }
}

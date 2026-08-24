//! Domain query: bounded pagination, cursor continuation, `--all` bounding and
//! resync over the in-memory [`DomainState`].
//!
//! Every listing uses a stable canonical-id keyset order. Each page is a
//! bounded observation of current state; continuation resumes strictly after
//! the prior key and never relies on an offset that would shift under mutation.

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use dxbot_core::types::{
    BotId, BotSelector, ChannelSelector, ConversationId, ProjectSelector, ScopeSelector,
    ThreadId, TaskId,
};

use crate::mutation::AppError;
use crate::state::{
    BotState, ConversationState, DomainState, LifecycleState, TaskState, TaskStatus, ThreadState,
};

/// A single bounded, cursor-ordered page of results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// The cursor at which the *next* page resumes, when [`Self::has_more`].
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// Snapshot summary of a bot identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotSummary {
    pub id: BotId,
    pub name: String,
    pub lifecycle: LifecycleState,
    pub revision: i64,
}

/// Snapshot summary of a conversation identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSummary {
    pub id: ConversationId,
    pub bot_id: BotId,
    pub revision: i64,
}

/// Snapshot summary of a thread identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSummary {
    pub id: ThreadId,
    pub conversation_id: ConversationId,
    pub revision: i64,
    pub title: String,
}

/// Snapshot summary of a task identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSummary {
    pub id: TaskId,
    pub owner: String,
    pub status: TaskStatus,
    pub revision: i64,
}

/// Result of a bounded `--all` traversal.
#[derive(Debug, Clone, PartialEq)]
pub enum AllLoopResult<T> {
    /// The source was exhausted without exceeding the local ceiling.
    Complete { items: Vec<T> },
    /// The local ceiling was reached before the source was exhausted; the
    /// caller should resume from `next_cursor`.
    Partial {
        items: Vec<T>,
        next_cursor: Option<String>,
    },
}

/// Presents canonical domain state as cursor-based paged query access.
#[derive(Debug)]
pub struct ApplicationQuery {
    state: Arc<Mutex<DomainState>>,
}

impl ApplicationQuery {
    /// Create a query over a shared in-memory domain state store.
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    /// List bots as a bounded, cursor-ordered page.
    pub fn list_bots(
        &self,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<BotSummary>, AppError> {
        let guard = self.lock()?;
        let bots: Vec<BotState> = guard.bots.values().cloned().collect();
        Ok(paginate(
            bots,
            |bot| bot.id.0.clone(),
            to_bot_summary,
            page_size,
            cursor.as_deref(),
        ))
    }

    /// List conversations scoped to one bot as a bounded, cursor-ordered page.
    pub fn list_conversations(
        &self,
        bot_id: &BotId,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<ConversationSummary>, AppError> {
        let guard = self.lock()?;
        let conversations: Vec<ConversationState> = guard
            .conversations
            .values()
            .filter(|conversation| conversation.bot_id == *bot_id)
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

    /// List threads scoped to one conversation as a bounded, cursor-ordered page.
    pub fn list_threads(
        &self,
        conversation_id: &ConversationId,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<ThreadSummary>, AppError> {
        let guard = self.lock()?;
        let threads: Vec<ThreadState> = guard
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

    /// List tasks scoped to a bot/project/channel as a bounded, cursor-ordered page.
    pub fn list_tasks(
        &self,
        scope: &ScopeSelector,
        page_size: usize,
        cursor: Option<String>,
    ) -> Result<Page<TaskSummary>, AppError> {
        let owner = scope_owner(scope)
            .ok_or_else(|| AppError::Internal("scope has no owner key".to_owned()))?;
        let guard = self.lock()?;
        let tasks: Vec<TaskState> = guard
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

/// Bounded `--all` traversal over a paged fetch.
///
/// Every request is clamped to the remaining local capacity, so a remote page
/// can never cause the accumulator to overshoot `local_ceiling`.
pub fn all_loop<T: Debug + Clone + PartialEq>(
    mut fetch: impl FnMut(usize, Option<String>) -> Result<Page<T>, AppError>,
    page_size: usize,
    local_ceiling: usize,
) -> Result<AllLoopResult<T>, AppError> {
    drain_all(&mut fetch, page_size, local_ceiling, None)
}

/// Restarts a bounded `--all` traversal from a previously captured cursor.
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

/// Maps an owner key out of a scope selector for task scoping.
fn scope_owner(scope: &ScopeSelector) -> Option<String> {
    match scope {
        ScopeSelector::Bot(BotSelector::CanonicalId(id)) => Some(id.0.clone()),
        ScopeSelector::Bot(BotSelector::ScopedExact(name)) => Some(name.clone()),
        ScopeSelector::Project(ProjectSelector::CanonicalId(id)) => Some(id.0.clone()),
        ScopeSelector::Project(ProjectSelector::VisibleExact(name)) => Some(name.clone()),
        ScopeSelector::Channel(ChannelSelector::CanonicalId(id)) => Some(id.0.clone()),
        ScopeSelector::Channel(ChannelSelector::ProjectExact { project, name }) => {
            Some(format!("{project}:{name}"))
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
    ConversationSummary {
        id: conversation.id,
        bot_id: conversation.bot_id,
        revision: conversation.revision,
    }
}

fn to_thread_summary(thread: ThreadState) -> ThreadSummary {
    let title = thread.id.0.clone();
    ThreadSummary {
        id: thread.id,
        conversation_id: thread.conversation_id,
        revision: thread.revision,
        title,
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

/// Orders `source` by a stable sort key, applies cursor continuation, and cuts
/// a single page of at most `page_size` items.
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
    let items: Vec<T> = source.into_iter().take(page_size).map(map).collect();
    Page {
        items,
        next_cursor,
        has_more,
    }
}

//! Domain query: bounded pagination, cursor continuation, `--all` bounding and
//! resync over the in-memory [`DomainState`].
//!
//! Every listing here is a *snapshot page*: a stable sort key (the canonical
//! id) gives a deterministic, foreign-cursor-safe ordering, and an opaque
//! cursor (the last id of the previous page) resumes exactly where the caller
//! left off, so concurrent mutation between pages cannot silently drop or
//! duplicate rows the way an offset-based query would.

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
    /// The full snapshot fit within the local ceiling.
    Complete { items: Vec<T> },
    /// The local ceiling was reached before the source was exhausted; the
    /// caller should resync from [`AllLoopResult::Partial`]'s next_cursor.
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
        let page = paginate(bots, |b| b.id.0.clone(), to_bot_summary, page_size, cursor.as_deref());
        Ok(page)
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
            .filter(|c| c.bot_id == *bot_id)
            .cloned()
            .collect();
        let page = paginate(
            conversations,
            |c| c.id.0.clone(),
            to_conversation_summary,
            page_size,
            cursor.as_deref(),
        );
        Ok(page)
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
            .filter(|t| t.conversation_id == *conversation_id)
            .cloned()
            .collect();
        let page = paginate(
            threads,
            |t| t.id.0.clone(),
            to_thread_summary,
            page_size,
            cursor.as_deref(),
        );
        Ok(page)
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
            .filter(|t| t.owner == owner)
            .cloned()
            .collect();
        let page = paginate(
            tasks,
            |t| t.id.0.clone(),
            to_task_summary,
            page_size,
            cursor.as_deref(),
        );
        Ok(page)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

/// Bounded `--all` traversal over a paged fetch.
///
/// Drains pages until either the source reports no more items — yielding
/// [`AllLoopResult::Complete`] — or the number of collected items reaches the
/// local ceiling, in which case it yields [`AllLoopResult::Partial`] carrying
/// the cursor to resume/resync from, so an unbounded remote never runs a local
/// process out of memory.
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
    let mut items = Vec::new();
    loop {
        let page = fetch(page_size, cursor)?;
        let next_cursor = page.next_cursor;
        let has_more = page.has_more;
        items.extend(page.items);
        if items.len() >= local_ceiling {
            return Ok(AllLoopResult::Partial {
                items,
                next_cursor,
            });
        }
        if !has_more {
            return Ok(AllLoopResult::Complete { items });
        }
        cursor = next_cursor;
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
    source.sort_by_key(|a| key_of(a));
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
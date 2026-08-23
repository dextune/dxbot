//! AT-APP-007: application subscription over in-memory event streams.
//!
//! A [`SubscriptionManager`] exposes cursor-based, at-least-once event
//! observation of task and process activity backed by the in-memory
//! [`DomainState`] fixture (no external I/O). Events for a target are appended
//! with a monotonically increasing cursor; a subscription records the last
//! cursor it delivered, so disconnected consumers can reconnect from that
//! cursor and resume without gaps.
//!
//! Contract:
//!
//! - **Cursor**: every retained event carries a cursor that strictly increases
//!   across the target's lifetime, so positions remain stable across reconnect.
//! - **Gap detection**: a request for a cursor behind the oldest retained event
//!   fails with [`AppError::GapDetected`] and a resync hint; the caller must
//!   resync from the current state instead of guessing.
//! - **Explicit resync**: subscribing (or re-subscribing) with no cursor replays
//!   the entire current retained window from the current state.
//! - **Bounded buffer**: at most [`MAX_EVENTS_PER_STREAM`] events are retained
//!   per target; the oldest are pruned first.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dxbot_core::types::{ProcessId, TaskId};

use crate::mutation::AppError;
use crate::state::DomainState;

/// Maximum number of events retained per target stream. Oldest events are
/// pruned once a target exceeds this bound.
pub const MAX_EVENTS_PER_STREAM: usize = 1000;

/// A structured event delivered on a subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    /// Human-oriented progress hint with an optional completion percentage.
    Progress { message: String, percent: Option<u8> },
    /// A transition of the target's status from one state to another.
    StatusChange { from: String, to: String },
    /// A chunk of process/task output.
    Output { data: String },
    /// The terminal result of a task.
    Complete { result: TaskResult },
    /// An error surfaced while the target was observed.
    Error { message: String },
}

/// The terminal result of a task observed through a subscription.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub status: String,
    pub output: Option<String>,
}

/// The public handle returned to a subscriber.
#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    pub id: String,
    pub target_id: String,
    /// The cursor the subscription resumes from, if the caller supplied one.
    pub cursor: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

// ── Internal registry stored in DomainState ──

/// Per-target retained event window. Shared by every subscription to the same
/// target; cursors are total-ordered within the target so reconnect and gap
/// detection operate on stable, cross-subscription positions.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamState {
    pub target_id: String,
    /// Cursor to assign to the next appended event (monotonic from `1`).
    next_cursor: u64,
    /// Retained entries, oldest first, cursor ascending.
    entries: VecDeque<StreamEntry>,
}

impl StreamState {
    fn new(target_id: &str) -> Self {
        Self {
            target_id: target_id.to_owned(),
            next_cursor: 1,
            entries: VecDeque::new(),
        }
    }

    /// The cursor of the oldest retained event, if any.
    fn oldest_cursor(&self) -> Option<u64> {
        self.entries.front().map(|e| e.cursor)
    }
}

/// One retained event alongside its stream cursor.
#[derive(Debug, Clone, PartialEq)]
struct StreamEntry {
    cursor: u64,
    event: StreamEvent,
}

/// A subscription's position within a target stream.
#[derive(Debug, Clone, PartialEq)]
struct SubscriptionRecord {
    id: String,
    target_id: String,
    created_at: i64,
    expires_at: Option<i64>,
    /// The last delivered cursor (`0` means nothing delivered yet).
    delivered_cursor: u64,
}

/// AT-APP-007 registry of target streams and subscription positions.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubscriptionRegistry {
    /// `target_id` -> retained event stream.
    streams: HashMap<String, StreamState>,
    /// `subscription_id` -> delivery position.
    by_id: HashMap<String, SubscriptionRecord>,
    next_subscription_seq: u64,
}

/// Application subscription manager over a shared in-memory domain state.
#[derive(Debug)]
pub struct SubscriptionManager {
    state: Arc<Mutex<DomainState>>,
}

impl SubscriptionManager {
    /// Create a manager over a shared in-memory domain state store.
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    /// Subscribe to a task's event stream, resuming from `cursor` when given.
    pub fn subscribe_task(
        &self,
        task_id: &TaskId,
        cursor: Option<String>,
    ) -> Result<Subscription, AppError> {
        self.subscribe(&task_id.0, cursor.as_deref())
    }

    /// Subscribe to a process's event stream, resuming from `cursor` when given.
    pub fn subscribe_process(
        &self,
        process_id: &ProcessId,
        cursor: Option<String>,
    ) -> Result<Subscription, AppError> {
        self.subscribe(&process_id.0, cursor.as_deref())
    }

    /// Subscribe for resync: re-subscribing with no cursor replays the whole
    /// current retained window from the target's current state.
    pub fn explicit_resync(&self, target_id: &str) -> Result<Subscription, AppError> {
        self.subscribe(target_id, None)
    }

    /// Emit an event to a target stream, returning the assigned cursor.
    pub fn emit(&self, target_id: &str, event: StreamEvent) -> Result<String, AppError> {
        let mut guard = self.lock()?;
        let stream = guard
            .subscriptions
            .streams
            .entry(target_id.to_owned())
            .or_insert_with(|| StreamState::new(target_id));

        let cursor = stream.next_cursor;
        stream.entries.push_back(StreamEntry { cursor, event });
        stream.next_cursor += 1;

        // Bounded buffer: prune the oldest events once the window overflows.
        while stream.entries.len() > MAX_EVENTS_PER_STREAM {
            stream.entries.pop_front();
        }

        Ok(cursor.to_string())
    }

    /// Emit a task event, returning the assigned cursor.
    pub fn emit_task(&self, task_id: &TaskId, event: StreamEvent) -> Result<String, AppError> {
        self.emit(&task_id.0, event)
    }

    /// Emit a process event, returning the assigned cursor.
    pub fn emit_process(
        &self,
        process_id: &ProcessId,
        event: StreamEvent,
    ) -> Result<String, AppError> {
        self.emit(&process_id.0, event)
    }

    /// Reconnect a subscription from `last_cursor`, resuming without gaps.
    ///
    /// The subscription resumes delivering events strictly after `last_cursor`.
    /// If `last_cursor` is behind the oldest retained event, the call fails
    /// with [`AppError::GapDetected`] carrying a resync hint.
    pub fn reconnect(
        &self,
        subscription_id: &str,
        last_cursor: &str,
    ) -> Result<Subscription, AppError> {
        let mut guard = self.lock()?;
        let registry = &mut guard.subscriptions;

        let (target_id, created_at, expires_at) = {
            let record = registry
                .by_id
                .get_mut(subscription_id)
                .ok_or_else(|| AppError::NotFound(format!("no subscription {subscription_id}")))?;
            (record.target_id.clone(), record.created_at, record.expires_at)
        };

        let requested = parse_cursor(Some(last_cursor))?;
        let stream = registry
            .streams
            .get(&target_id)
            .ok_or_else(|| AppError::NotFound(format!("no stream for target {target_id}")))?;

        check_gap(&target_id, requested, stream)?;

        if let Some(record) = registry.by_id.get_mut(subscription_id) {
            record.delivered_cursor = requested;
        }

        Ok(Subscription {
            id: subscription_id.to_owned(),
            target_id,
            cursor: Some(last_cursor.to_owned()),
            created_at,
            expires_at,
        })
    }

    /// Return the next event for `subscription`, waiting up to `timeout`.
    ///
    /// Polls the shared state so events emitted concurrently are observed; if
    /// none arrives before `timeout` elapses, returns [`AppError::Timeout`].
    #[allow(clippy::needless_pass_by_value)]
    pub fn next_event(
        &mut self,
        subscription: &Subscription,
        timeout: Duration,
    ) -> Result<StreamEvent, AppError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(event) = self.try_next_event(subscription)? {
                return Ok(event);
            }
            if Instant::now() >= deadline {
                return Err(AppError::Timeout(format!(
                    "no event on subscription {} within {timeout:?}",
                    subscription.id
                )));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// The cursor of the last event delivered to `subscription`, if any.
    pub fn last_delivered_cursor(&self, subscription: &Subscription) -> Option<String> {
        let guard = self
            .state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
            .ok()?;
        match guard.subscriptions.by_id.get(&subscription.id) {
            Some(rec) if rec.delivered_cursor > 0 => Some(rec.delivered_cursor.to_string()),
            _ => None,
        }
    }

    fn try_next_event(
        &self,
        subscription: &Subscription,
    ) -> Result<Option<StreamEvent>, AppError> {
        let mut guard = self.lock()?;

        // Resolve the target under a scoped, mutable borrow of the record, then
        // release it before touching the stream so the two registry maps don't
        // alias mutably at the same time.
        let target_id = {
            let record = guard.subscriptions.by_id.get_mut(&subscription.id).ok_or_else(|| {
                AppError::NotFound(format!("no subscription {}", subscription.id))
            })?;
            record.target_id.clone()
        };

        let next = {
            let delivered = guard
                .subscriptions
                .by_id
                .get(&subscription.id)
                .map(|r| r.delivered_cursor)
                .unwrap_or(0);
            let stream = guard
                .subscriptions
                .streams
                .get_mut(&target_id)
                .ok_or_else(|| AppError::NotFound(format!("no stream for target {target_id}")))?;
            stream.entries.iter().find(|e| e.cursor > delivered)
                .map(|entry| (entry.cursor, entry.event.clone()))
        };

        if let Some((cursor, event)) = next {
            if let Some(record) = guard.subscriptions.by_id.get_mut(&subscription.id) {
                record.delivered_cursor = cursor;
            }
            return Ok(Some(event));
        }

        Ok(None)
    }

    fn subscribe(
        &self,
        target_id: &str,
        cursor: Option<&str>,
    ) -> Result<Subscription, AppError> {
        let mut guard = self.lock()?;
        let registry = &mut guard.subscriptions;

        let stream = registry
            .streams
            .entry(target_id.to_owned())
            .or_insert_with(|| StreamState::new(target_id));

        let requested = parse_cursor(cursor)?;
        check_gap(target_id, requested, stream)?;

        let id = format!("sub-{}", registry.next_subscription_seq);
        registry.next_subscription_seq += 1;
        let now = now_epoch();
        let expires_at = now.saturating_add(3600); // 1h default lease

        registry.by_id.insert(
            id.clone(),
            SubscriptionRecord {
                id: id.clone(),
                target_id: target_id.to_owned(),
                created_at: now,
                expires_at: Some(expires_at),
                delivered_cursor: requested,
            },
        );

        Ok(Subscription {
            id,
            target_id: target_id.to_owned(),
            cursor: cursor.map(str::to_owned),
            created_at: now,
            expires_at: Some(expires_at),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, DomainState>, AppError> {
        self.state
            .lock()
            .map_err(|_| AppError::Internal("domain state mutex poisoned".to_owned()))
    }
}

/// Reject a cursor that falls behind the oldest retained event.
///
/// A cursor of `0` (a fresh/full resync) is always allowed: the caller wants a
/// complete replay of the retained window. Any other cursor older than the
/// oldest retained event has been lost to pruning and must resync instead.
fn check_gap(
    target_id: &str,
    requested: u64,
    stream: &StreamState,
) -> Result<(), AppError> {
    let oldest = stream.oldest_cursor();
    if requested > 0 {
        if let Some(oldest) = oldest {
            if requested < oldest {
                return Err(AppError::GapDetected(resync_hint(
                    target_id,
                    oldest,
                    stream.next_cursor,
                )));
            }
        }
    }
    Ok(())
}

fn resync_hint(target_id: &str, oldest: u64, next: u64) -> String {
    format!(
        "cursor gap on {target_id}: oldest retained is {oldest} (next {next}); \
         subscribe with no cursor to resync from current state"
    )
}

fn parse_cursor(cursor: Option<&str>) -> Result<u64, AppError> {
    match cursor {
        None => Ok(0),
        Some(c) => c.parse::<u64>().map_err(|_| {
            AppError::Internal(format!("invalid subscription cursor: {c}"))
        }),
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
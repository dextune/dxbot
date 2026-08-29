//! AT-APP-007: application subscription over bounded in-memory event streams.
//!
//! Subscriptions use per-target monotonic cursors. Falling behind retained
//! history fails explicitly with a resync hint. Production local-control watch
//! uses stateless target+cursor reads so reconnect does not accumulate server
//! subscription handles; the handle API remains for component users.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use dxbot_core::types::{ProcessId, TaskId};
use serde::{Deserialize, Serialize};

use crate::mutation::AppError;
use crate::state::DomainState;

pub const MAX_EVENTS_PER_STREAM: usize = 1000;
const SUBSCRIPTION_LEASE_SECONDS: i64 = 3600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum StreamEvent {
    Progress { message: String, percent: Option<u8> },
    StatusChange { from: String, to: String },
    Output { data: String },
    Complete { result: TaskResult },
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub status: String,
    pub output: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    pub id: String,
    pub target_id: String,
    pub cursor: Option<String>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StreamState {
    pub target_id: String,
    next_cursor: u64,
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

    fn oldest_cursor(&self) -> Option<u64> {
        self.entries.front().map(|entry| entry.cursor)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct StreamEntry {
    cursor: u64,
    event: StreamEvent,
}

#[derive(Debug, Clone, PartialEq)]
struct SubscriptionRecord {
    target_id: String,
    created_at: i64,
    expires_at: Option<i64>,
    delivered_cursor: u64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SubscriptionRegistry {
    streams: HashMap<String, StreamState>,
    by_id: HashMap<String, SubscriptionRecord>,
    next_subscription_seq: u64,
}

#[derive(Debug)]
pub struct SubscriptionManager {
    state: Arc<Mutex<DomainState>>,
}

impl SubscriptionManager {
    pub fn new(state: Arc<Mutex<DomainState>>) -> Self {
        Self { state }
    }

    pub fn subscribe_task(
        &self,
        task_id: &TaskId,
        cursor: Option<String>,
    ) -> Result<Subscription, AppError> {
        self.subscribe(&task_id.0, cursor.as_deref())
    }

    pub fn subscribe_process(
        &self,
        process_id: &ProcessId,
        cursor: Option<String>,
    ) -> Result<Subscription, AppError> {
        self.subscribe(&process_id.0, cursor.as_deref())
    }

    pub fn explicit_resync(&self, target_id: &str) -> Result<Subscription, AppError> {
        self.subscribe(target_id, None)
    }

    pub fn emit(&self, target_id: &str, event: StreamEvent) -> Result<String, AppError> {
        let mut guard = self.lock()?;
        emit_in_state(&mut guard, target_id, event)
    }

    pub fn emit_task(&self, task_id: &TaskId, event: StreamEvent) -> Result<String, AppError> {
        self.emit(&task_id.0, event)
    }

    pub fn emit_process(
        &self,
        process_id: &ProcessId,
        event: StreamEvent,
    ) -> Result<String, AppError> {
        self.emit(&process_id.0, event)
    }

    /// Stateless production watch primitive. `cursor` is the last event already
    /// observed by the caller; the returned cursor is the event delivered now.
    /// No server-side subscription handle is retained between calls.
    pub fn next_event_from(
        &self,
        target_id: &str,
        cursor: Option<&str>,
        timeout: Duration,
    ) -> Result<(StreamEvent, String), AppError> {
        let delivered = parse_cursor(cursor)?;
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            AppError::Internal("subscription timeout exceeds monotonic clock range".to_owned())
        })?;
        loop {
            if let Some(next) = self.try_next_event_from(target_id, delivered)? {
                return Ok(next);
            }
            if Instant::now() >= deadline {
                return Err(AppError::Timeout(format!(
                    "no event on target {target_id} within {timeout:?}"
                )));
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

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
                .get(subscription_id)
                .ok_or_else(|| AppError::NotFound(format!("no subscription {subscription_id}")))?;
            ensure_lease_active(subscription_id, record.expires_at)?;
            (
                record.target_id.clone(),
                record.created_at,
                record.expires_at,
            )
        };

        let requested = parse_cursor(Some(last_cursor))?;
        let stream = registry
            .streams
            .get(&target_id)
            .ok_or_else(|| AppError::NotFound(format!("no stream for target {target_id}")))?;
        validate_resume_cursor(&target_id, requested, stream)?;

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

    pub fn next_event(
        &mut self,
        subscription: &Subscription,
        timeout: Duration,
    ) -> Result<StreamEvent, AppError> {
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            AppError::Internal("subscription timeout exceeds monotonic clock range".to_owned())
        })?;
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

    pub fn last_delivered_cursor(&self, subscription: &Subscription) -> Option<String> {
        let guard = self.state.lock().ok()?;
        let record = guard.subscriptions.by_id.get(&subscription.id)?;
        if ensure_lease_active(&subscription.id, record.expires_at).is_err() {
            return None;
        }
        (record.delivered_cursor > 0).then(|| record.delivered_cursor.to_string())
    }

    fn try_next_event_from(
        &self,
        target_id: &str,
        delivered: u64,
    ) -> Result<Option<(StreamEvent, String)>, AppError> {
        let guard = self.lock()?;
        let stream = guard
            .subscriptions
            .streams
            .get(target_id)
            .ok_or_else(|| AppError::NotFound(format!("no stream for target {target_id}")))?;
        validate_resume_cursor(target_id, delivered, stream)?;
        Ok(stream
            .entries
            .iter()
            .find(|entry| entry.cursor > delivered)
            .map(|entry| (entry.event.clone(), entry.cursor.to_string())))
    }

    fn try_next_event(
        &self,
        subscription: &Subscription,
    ) -> Result<Option<StreamEvent>, AppError> {
        let mut guard = self.lock()?;
        let (target_id, delivered) = {
            let record = guard
                .subscriptions
                .by_id
                .get(&subscription.id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("no subscription {}", subscription.id))
                })?;
            ensure_lease_active(&subscription.id, record.expires_at)?;
            (record.target_id.clone(), record.delivered_cursor)
        };

        let next = {
            let stream = guard
                .subscriptions
                .streams
                .get(&target_id)
                .ok_or_else(|| AppError::NotFound(format!("no stream for target {target_id}")))?;
            validate_resume_cursor(&target_id, delivered, stream)?;
            stream
                .entries
                .iter()
                .find(|entry| entry.cursor > delivered)
                .map(|entry| (entry.cursor, entry.event.clone()))
        };

        if let Some((cursor, event)) = next {
            let record = guard
                .subscriptions
                .by_id
                .get_mut(&subscription.id)
                .ok_or_else(|| {
                    AppError::NotFound(format!("no subscription {}", subscription.id))
                })?;
            record.delivered_cursor = cursor;
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

        let delivered_cursor = match cursor {
            Some(value) => {
                let requested = parse_cursor(Some(value))?;
                validate_resume_cursor(target_id, requested, stream)?;
                requested
            }
            None => stream
                .oldest_cursor()
                .map_or(0, |oldest| oldest.saturating_sub(1)),
        };

        let sequence = registry.next_subscription_seq;
        registry.next_subscription_seq = sequence.checked_add(1).ok_or_else(|| {
            AppError::Internal("subscription id space exhausted".to_owned())
        })?;
        let id = format!("sub-{sequence}");
        let now = now_epoch();
        let expires_at = now.saturating_add(SUBSCRIPTION_LEASE_SECONDS);

        registry.by_id.insert(
            id.clone(),
            SubscriptionRecord {
                target_id: target_id.to_owned(),
                created_at: now,
                expires_at: Some(expires_at),
                delivered_cursor,
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

pub(crate) fn emit_in_state(
    state: &mut DomainState,
    target_id: &str,
    event: StreamEvent,
) -> Result<String, AppError> {
    let stream = state
        .subscriptions
        .streams
        .entry(target_id.to_owned())
        .or_insert_with(|| StreamState::new(target_id));
    let cursor = stream.next_cursor;
    stream.next_cursor = cursor.checked_add(1).ok_or_else(|| {
        AppError::Internal(format!("cursor space exhausted for target {target_id}"))
    })?;
    stream.entries.push_back(StreamEntry { cursor, event });
    while stream.entries.len() > MAX_EVENTS_PER_STREAM {
        stream.entries.pop_front();
    }
    Ok(cursor.to_string())
}

fn validate_resume_cursor(
    target_id: &str,
    requested: u64,
    stream: &StreamState,
) -> Result<(), AppError> {
    if requested >= stream.next_cursor && requested != 0 {
        return Err(AppError::Conflict(format!(
            "cursor {requested} is ahead of target {target_id} (next {})",
            stream.next_cursor
        )));
    }

    if let Some(oldest) = stream.oldest_cursor() {
        if requested.saturating_add(1) < oldest {
            return Err(AppError::GapDetected(resync_hint(
                target_id,
                oldest,
                stream.next_cursor,
            )));
        }
    }
    Ok(())
}

fn ensure_lease_active(subscription_id: &str, expires_at: Option<i64>) -> Result<(), AppError> {
    if expires_at.is_some_and(|expires_at| expires_at <= now_epoch()) {
        return Err(AppError::Conflict(format!(
            "subscription {subscription_id} lease expired; resubscribe"
        )));
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
        Some(cursor) => cursor
            .parse::<u64>()
            .map_err(|_| AppError::Conflict(format!("invalid subscription cursor: {cursor}"))),
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

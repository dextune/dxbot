//! Exact, ambiguity-safe resource selector resolution (AT-CLI-009).
//!
//! Users address resources with either a *canonical ID* or a *scoped exact*
//! display name. This module resolves that input to a single domain selector,
//! never guessing and never letting a fuzzy match become a mutation target.
//!
//! Resolution rules:
//!
//! 1. Exact canonical ID match → resolved immediately.
//! 2. Exactly one scoped exact match → resolved immediately.
//! 3. More than one match → [`Error::Ambiguous`] with the visible canonical
//!    refs of every matched candidate, so a caller can show the user what to
//!    choose among without ever auto-selecting.
//! 4. Zero matches → [`Error::NotFound`].
//!
//! A fuzzy (edit-distance) match is available only via
//! [`SelectorResolver::suggest_candidates`] as a *help* suggestion. It is never
//! returned by [`SelectorResolver::resolve_bot`]/`_conversation`/`_thread`/
//! `_task`, so it can never be used as a mutation execution target.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};
use dxbot_core::types::*;

/// A resolved selector-producing outcome before it is mapped to a domain type.
enum ResolveOutcome {
    /// Index of the single canonical-ID match.
    Canonical(usize),
    /// Index of the single scoped-exact match.
    Scoped(usize),
    /// Multiple scoped matches; carries their visible canonical refs.
    Ambiguous(Vec<String>),
    NotFound,
}

/// Common projection shared by every summary type a selector can address.
///
/// A summary is a lightweight, visible projection used only for local
/// resolution and help; it is never an authority for canonical state.
trait Resolvable {
    /// The canonical resource ref, prefixed and unique.
    fn canonical_ref(&self) -> &str;
    /// The scoped exact display name a user may type.
    fn scoped_name(&self) -> &str;
}

// ── Summary projections ──

/// Visible, resolution-only projection of a Bot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotSummary {
    pub id: BotId,
    /// Scoped exact display name (unique within its scope).
    pub name: String,
}

impl Resolvable for BotSummary {
    fn canonical_ref(&self) -> &str {
        &self.id.0
    }
    fn scoped_name(&self) -> &str {
        &self.name
    }
}

/// Visible, resolution-only projection of a Conversation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationSummary {
    pub id: ConversationId,
    /// Scoped exact display name.
    pub name: String,
    /// Owning Bot's canonical ref when this is a Bot main conversation.
    pub parent_bot: Option<String>,
}

impl Resolvable for ConversationSummary {
    fn canonical_ref(&self) -> &str {
        &self.id.0
    }
    fn scoped_name(&self) -> &str {
        &self.name
    }
}

/// Visible, resolution-only projection of a Thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSummary {
    pub id: ThreadId,
    /// Scoped exact display name.
    pub name: String,
}

impl Resolvable for ThreadSummary {
    fn canonical_ref(&self) -> &str {
        &self.id.0
    }
    fn scoped_name(&self) -> &str {
        &self.name
    }
}

/// Visible, resolution-only projection of a Task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskSummary {
    pub id: TaskId,
    /// Scoped exact display name.
    pub name: String,
}

impl Resolvable for TaskSummary {
    fn canonical_ref(&self) -> &str {
        &self.id.0
    }
    fn scoped_name(&self) -> &str {
        &self.name
    }
}

/// Errors produced by exact selector resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// More than one candidate matched a scoped input; `candidates` holds the
    /// visible canonical refs of every matching resource.
    Ambiguous { candidates: Vec<String> },
    /// No candidate matched the input at all.
    NotFound,
}

impl Error {
    /// Projects this error onto the canonical [`DxbotError`] surface so the CLI
    /// can fall through to the standard error/exit-code contract.
    pub fn to_dxbot_error(&self) -> DxbotError {
        match self {
            Self::Ambiguous { candidates } => DxbotError {
                code: ErrorCode::AmbiguousTarget,
                category: ErrorCategory::Conflict,
                message: format!(
                    "multiple resources matched; specify a canonical ID or disambiguate: {candidates:?}"
                ),
                retryable: false,
                operation_ref: None,
                target_refs: candidates.clone(),
                field_violations: Vec::new(),
                current_revision: None,
                current_generation: None,
                resume_cursor: None,
                next_actions: Vec::new(),
            },
            Self::NotFound => DxbotError {
                code: ErrorCode::NotFound,
                category: ErrorCategory::Input,
                message: "no matching resource found; a fuzzy suggestion is for help only".to_string(),
                retryable: false,
                operation_ref: None,
                target_refs: Vec::new(),
                field_violations: Vec::new(),
                current_revision: None,
                current_generation: None,
                resume_cursor: None,
                next_actions: Vec::new(),
            },
        }
    }
}

/// Resolves user-supplied resource inputs to exact, unambiguous selectors.
///
/// `PartialEq` only (no `Eq`) because it carries an `f64` fuzzy-distance ratio.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorResolver {
    /// Fuzzy-suggestion distance fraction relative to the input length.
    fuzzy_distance_ratio: f64,
}

impl Default for SelectorResolver {
    fn default() -> Self {
        Self {
            fuzzy_distance_ratio: 0.4,
        }
    }
}

impl SelectorResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves a Bot selector. See module docs for the ambiguity rules.
    pub fn resolve_bot(
        &self,
        input: &str,
        candidates: &[BotSummary],
    ) -> Result<BotSelector, Error> {
        match resolve_exact(input, candidates) {
            ResolveOutcome::Canonical(i) => {
                Ok(BotSelector::CanonicalId(candidates[i].id.clone()))
            }
            ResolveOutcome::Scoped(i) => {
                Ok(BotSelector::ScopedExact(candidates[i].name.clone()))
            }
            ResolveOutcome::Ambiguous(c) => Err(Error::Ambiguous { candidates: c }),
            ResolveOutcome::NotFound => Err(Error::NotFound),
        }
    }

    /// Resolves a Conversation selector.
    pub fn resolve_conversation(
        &self,
        input: &str,
        candidates: &[ConversationSummary],
    ) -> Result<ConversationSelector, Error> {
        match resolve_exact(input, candidates) {
            ResolveOutcome::Canonical(i) | ResolveOutcome::Scoped(i) => {
                Ok(ConversationSelector::ConversationId(candidates[i].id.clone()))
            }
            ResolveOutcome::Ambiguous(c) => Err(Error::Ambiguous { candidates: c }),
            ResolveOutcome::NotFound => Err(Error::NotFound),
        }
    }

    /// Resolves a Thread selector.
    pub fn resolve_thread(
        &self,
        input: &str,
        candidates: &[ThreadSummary],
    ) -> Result<ThreadSelector, Error> {
        match resolve_exact(input, candidates) {
            ResolveOutcome::Canonical(i) => {
                Ok(ThreadSelector::CanonicalId(candidates[i].id.clone()))
            }
            ResolveOutcome::Scoped(i) => {
                Ok(ThreadSelector::ScopedExact(candidates[i].name.clone()))
            }
            ResolveOutcome::Ambiguous(c) => Err(Error::Ambiguous { candidates: c }),
            ResolveOutcome::NotFound => Err(Error::NotFound),
        }
    }

    /// Resolves a Task selector.
    pub fn resolve_task(
        &self,
        input: &str,
        candidates: &[TaskSummary],
    ) -> Result<TaskSelector, Error> {
        match resolve_exact(input, candidates) {
            ResolveOutcome::Canonical(i) => {
                Ok(TaskSelector::CanonicalId(candidates[i].id.clone()))
            }
            ResolveOutcome::Scoped(i) => {
                Ok(TaskSelector::ScopedExact(candidates[i].name.clone()))
            }
            ResolveOutcome::Ambiguous(c) => Err(Error::Ambiguous { candidates: c }),
            ResolveOutcome::NotFound => Err(Error::NotFound),
        }
    }

    /// Distance-thresholded suggestions for *help only*.
    ///
    /// Candidates are plain visible refs (e.g. scoped names or canonical
    /// refs). Exact matches are excluded (they resolve, not suggest), and any
    /// string within the distance threshold relative to `input` length is
    /// returned sorted by ascending distance. The returned values must never be
    /// used as a mutation target — see module docs.
    pub fn suggest_candidates(&self, input: &str, candidates: &[String]) -> Vec<String> {
        let threshold = self.distance_threshold(input);
        let mut scored: Vec<(usize, &String)> = candidates
            .iter()
            .map(|c| (edit_distance(input, c), c))
            .filter(|(dist, _)| *dist > 0 && *dist <= threshold)
            .collect();
        scored.sort_by_key(|(dist, _)| *dist);
        scored.into_iter().map(|(_, c)| c.clone()).collect()
    }

    fn distance_threshold(&self, input: &str) -> usize {
        let len = input.chars().count();
        ((len as f64) * self.fuzzy_distance_ratio).ceil() as usize
    }
}

/// Shared exact-resolution core implementing the AT-CLI-009 ambiguity rules.
fn resolve_exact<T: Resolvable>(input: &str, candidates: &[T]) -> ResolveOutcome {
    for (i, c) in candidates.iter().enumerate() {
        if c.canonical_ref() == input {
            return ResolveOutcome::Canonical(i);
        }
    }
    let matched: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.scoped_name() == input)
        .map(|(i, _)| i)
        .collect();
    match matched.len() {
        1 => ResolveOutcome::Scoped(matched[0]),
        0 => ResolveOutcome::NotFound,
        _ => {
            let refs: Vec<String> = matched
                .iter()
                .map(|&i| candidates[i].canonical_ref().to_string())
                .collect();
            ResolveOutcome::Ambiguous(refs)
        }
    }
}

/// Classic Levenshtein edit distance over Unicode scalar values.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![0usize; b.len() + 1];
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (cur[j - 1] + 1).min(prev[j] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}


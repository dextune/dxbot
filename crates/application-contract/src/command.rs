//! Canonical P0 command metadata and user-facing path resolution.
//!
//! The frozen registry remains the single semantic source. This module only
//! projects that registry into CLI-facing views so the binary does not maintain
//! a second command inventory or hand-written reachability table.

use dxbot_core::DxbotError;

use crate::registry::{REGISTRY, Reg};
use crate::util;

/// Read-only public view of one frozen P0 command row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandMetadata {
    pub command_key: &'static str,
    pub input_schema: &'static str,
    pub target_schema: &'static str,
    pub kind: &'static str,
    pub output_schema: &'static str,
    pub security_class: &'static str,
    pub wait_default: &'static str,
    pub wait_allowed: &'static str,
    pub typed_fields: &'static str,
}

/// Result of resolving a user-facing command path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedCommand {
    pub command_key: &'static str,
    /// Number of leading argv tokens consumed by the command path itself.
    pub consumed_path_tokens: usize,
}

impl From<&'static Reg> for CommandMetadata {
    fn from(row: &'static Reg) -> Self {
        Self {
            command_key: row.command_key,
            input_schema: row.input_schema,
            target_schema: row.target_schema,
            kind: row.kind,
            output_schema: row.output_schema,
            security_class: row.security_class,
            wait_default: row.wait_default,
            wait_allowed: row.wait_allowed,
            typed_fields: row.typed_fields,
        }
    }
}

/// Iterate over the 63 frozen command rows without allocating a second registry.
pub fn command_metadata() -> impl ExactSizeIterator<Item = CommandMetadata> {
    REGISTRY.iter().map(CommandMetadata::from)
}

/// Return metadata for one command key.
pub fn metadata_for_key(command_key: &str) -> Option<CommandMetadata> {
    REGISTRY
        .iter()
        .find(|row| row.command_key == command_key)
        .map(CommandMetadata::from)
}

/// Resolve the canonical user-facing path from argv tokens.
///
/// Most paths are the command key with `-` translated to path separators. The
/// three non-mechanical paths are intentionally centralized here rather than in
/// the binary:
/// - `runtime-stop-graceful` -> `runtime stop`
/// - `runtime-stop-host` -> `runtime stop --host-stop`
/// - `side-effect-reconcile` -> `side-effect reconcile`
pub fn resolve_cli_path(tokens: &[String]) -> Result<ResolvedCommand, DxbotError> {
    if tokens.is_empty() {
        return Err(util::input_error("missing command"));
    }

    if tokens.first().is_some_and(|token| token == "runtime")
        && tokens.get(1).is_some_and(|token| token == "stop")
    {
        let command_key = if tokens.iter().skip(2).any(|token| token == "--host-stop") {
            "runtime-stop-host"
        } else {
            "runtime-stop-graceful"
        };
        return Ok(ResolvedCommand {
            command_key,
            consumed_path_tokens: 2,
        });
    }

    let mut best: Option<ResolvedCommand> = None;
    for row in &REGISTRY {
        if matches!(
            row.command_key,
            "runtime-stop-graceful" | "runtime-stop-host"
        ) {
            continue;
        }
        let path = cli_path_tokens(row.command_key);
        if path.len() > tokens.len() {
            continue;
        }
        if path
            .iter()
            .zip(tokens.iter())
            .all(|(expected, actual)| *expected == actual)
        {
            let candidate = ResolvedCommand {
                command_key: row.command_key,
                consumed_path_tokens: path.len(),
            };
            if best
                .is_none_or(|current| candidate.consumed_path_tokens > current.consumed_path_tokens)
            {
                best = Some(candidate);
            }
        }
    }

    best.ok_or_else(|| {
        let path = tokens.join(" ");
        let message = suggest_command_path(tokens).map_or_else(
            || format!("unknown command path: {path}"),
            |suggestion| format!("unknown command path: {path}; did you mean '{suggestion}'?"),
        );
        util::input_error(message)
    })
}

/// Return one close, unambiguous command path from the frozen registry.
///
/// Suggestions are help-only: resolution never executes a fuzzy match. The
/// registry is fixed at 63 rows, candidate paths are deduplicated, and both
/// edit distance and accepted distance are bounded by the short command path.
fn suggest_command_path(tokens: &[String]) -> Option<String> {
    let mut candidates: Vec<(usize, String)> = Vec::new();
    for row in &REGISTRY {
        let path_tokens = cli_path_tokens(row.command_key);
        let path = path_tokens.join(" ");
        if candidates.iter().any(|(_, existing)| existing == &path) {
            continue;
        }
        let supplied = tokens
            .iter()
            .take(path_tokens.len())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" ");
        let distance = edit_distance(&supplied, &path);
        let threshold = match path.chars().count() {
            0..=4 => 1,
            5..=12 => 2,
            _ => 3,
        };
        if distance <= threshold {
            candidates.push((distance, path));
        }
    }
    candidates.sort();
    let (best_distance, best_path) = candidates.first()?;
    if candidates
        .iter()
        .skip(1)
        .any(|(distance, _)| distance == best_distance)
    {
        return None;
    }
    Some(best_path.clone())
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right_chars: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right_chars.len()).collect();
    let mut current = vec![0; right_chars.len() + 1];
    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = previous[right_index] + usize::from(left_char != *right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right_chars.len()]
}

/// User-facing path tokens for a registry command.
pub fn cli_path_tokens(command_key: &'static str) -> Vec<&'static str> {
    match command_key {
        "runtime-stop-graceful" | "runtime-stop-host" => vec!["runtime", "stop"],
        "side-effect-reconcile" => vec!["side-effect", "reconcile"],
        _ => command_key.split('-').collect(),
    }
}

/// Commands whose first path token is `group`, sorted by canonical path.
pub fn commands_in_group(group: &str) -> Vec<CommandMetadata> {
    let mut commands: Vec<_> = REGISTRY
        .iter()
        .filter(|row| {
            cli_path_tokens(row.command_key)
                .first()
                .is_some_and(|path_group| *path_group == group)
        })
        .map(CommandMetadata::from)
        .collect();
    commands.sort_by_key(|metadata| cli_path_tokens(metadata.command_key).join(" "));
    commands
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn every_registry_row_has_a_resolvable_cli_path() {
        assert_eq!(command_metadata().len(), 63);
        for metadata in command_metadata() {
            let mut path: Vec<String> = cli_path_tokens(metadata.command_key)
                .into_iter()
                .map(str::to_owned)
                .collect();
            if metadata.command_key == "runtime-stop-host" {
                path.push("--host-stop".to_owned());
            }
            let resolved = resolve_cli_path(&path).expect("registry path must resolve");
            assert_eq!(resolved.command_key, metadata.command_key);
        }
    }

    #[test]
    fn longest_path_wins_for_nested_member_commands() {
        let resolved = resolve_cli_path(&args(&["project", "member", "set", "--bot", "a"]))
            .expect("nested command must resolve");
        assert_eq!(resolved.command_key, "project-member-set");
        assert_eq!(resolved.consumed_path_tokens, 3);
    }

    #[test]
    fn host_stop_mode_is_distinct_from_graceful_stop() {
        let graceful =
            resolve_cli_path(&args(&["runtime", "stop"])).expect("graceful stop must resolve");
        let host = resolve_cli_path(&args(&["runtime", "stop", "--host-stop"]))
            .expect("host stop must resolve");
        assert_eq!(graceful.command_key, "runtime-stop-graceful");
        assert_eq!(host.command_key, "runtime-stop-host");
    }

    #[test]
    fn hyphenated_group_is_not_split() {
        let resolved = resolve_cli_path(&args(&["side-effect", "reconcile"]))
            .expect("side-effect path must resolve");
        assert_eq!(resolved.command_key, "side-effect-reconcile");
        assert_eq!(resolved.consumed_path_tokens, 2);
    }

    #[test]
    fn close_unknown_path_suggests_registry_command_without_executing_it() {
        let error = resolve_cli_path(&args(&["taks", "submit"]))
            .expect_err("fuzzy paths must never execute");
        assert!(error.message.contains("did you mean 'task submit'?"));
    }

    #[test]
    fn distant_unknown_path_omits_suggestion() {
        let error =
            resolve_cli_path(&args(&["completely-unrelated"])).expect_err("unknown path must fail");
        assert!(!error.message.contains("did you mean"));
    }
}

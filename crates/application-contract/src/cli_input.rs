//! User-facing CLI input types parsed from argv.
//!
//! `CliInput` is the parser-side projection (per `DXB-IFC-042`): it holds the raw user
//! selector, optional `--if-*` CAS guards, the content source selection and command
//! specific fields. It deliberately does **not** own canonical materialization — that is
//! the responsibility of `crate::contract`.

use dxbot_core::DxbotError;
use dxbot_core::types::*;
use serde_json::{Value, json};

use crate::command::metadata_for_key;
use crate::field_spec::is_primary_selector_flag;
use crate::registry::{command_count, is_known_command};
use crate::util;

/// A fully parsed CLI invocation (excluding any canonical materialization).
#[derive(Debug, Clone, PartialEq)]
pub struct CliInput {
    pub command_key: String,
    pub global_options: CliGlobalOptions,
    pub selector: Option<Value>,
    pub cas: Option<CasConditions>,
    pub content: Option<ContentSource>,
    pub fields: Value,
}

/// Number of registered command keys.
pub const COMMAND_COUNT: usize = 63;

impl CliInput {
    /// Parse a `CliInput` from argv-style tokens.
    ///
    /// `args[0]` is the `command_key` (e.g. `bot-create`); the remaining tokens are global
    /// options, an optional primary selector, optional `--if-*` CAS guards, a content source
    /// and command-specific fields. Secondary selector-typed fields remain ordinary fields.
    pub fn parse(args: &[String]) -> Result<CliInput, DxbotError> {
        let _ = command_count; // keep the registry scope explicit for diagnostics
        if args.is_empty() {
            return Err(util::input_error("missing command key"));
        }
        let command_key = args[0].clone();
        if !is_known_command(&command_key) {
            return Err(util::input_error(format!(
                "unknown command key: '{command_key}'"
            )));
        }
        let metadata = metadata_for_key(&command_key).ok_or_else(|| {
            util::invariant(format!("known command '{command_key}' has no metadata row"))
        })?;

        let mut global = CliGlobalOptions::default();
        let mut cas_builder = util::empty_cas();
        let mut cas_present = false;
        let mut selector: Option<Value> = None;
        let mut content: Option<ContentSource> = None;
        let mut fields = serde_json::Map::new();
        let mut i = 1usize;

        while i < args.len() {
            let arg = args[i].as_str();

            match arg {
                "--profile" => {
                    global.profile = Some(take_value(args, &mut i, "--profile")?);
                }
                "--instance" => {
                    global.instance = Some(take_value(args, &mut i, "--instance")?);
                }
                "--format" => {
                    let v = take_value(args, &mut i, "--format")?;
                    global.format = match v.as_str() {
                        "human" => OutputFormat::Human,
                        "json" => OutputFormat::Json,
                        "jsonl" => OutputFormat::Jsonl,
                        _ => {
                            return Err(util::input_error(format!("invalid --format: '{v}'")));
                        }
                    };
                }
                "--color" => {
                    let v = take_value(args, &mut i, "--color")?;
                    global.color = match v.as_str() {
                        "auto" => ColorMode::Auto,
                        "always" => ColorMode::Always,
                        "never" => ColorMode::Never,
                        _ => return Err(util::input_error(format!("invalid --color: '{v}'"))),
                    };
                }
                "--wait" => {
                    global.wait = Some(take_value(args, &mut i, "--wait")?);
                }
                "--timeout" => {
                    global.timeout = Some(take_value(args, &mut i, "--timeout")?);
                }
                "--yes" => {
                    i += 1;
                    global.yes = true;
                }
                "--text" => {
                    let v = take_value(args, &mut i, "--text")?;
                    set_content(&mut content, ContentSource::Text { value: v })?;
                }
                "--input-file" => {
                    let v = take_value(args, &mut i, "--input-file")?;
                    set_content(&mut content, ContentSource::InputFile { path: v })?;
                }
                "--stdin" => {
                    i += 1;
                    set_content(&mut content, ContentSource::Stdin)?;
                }
                "--artifact-ref" => {
                    let v = take_value(args, &mut i, "--artifact-ref")?;
                    let (artifact_id, digest) = v.split_once(':').ok_or_else(|| {
                        util::input_error("--artifact-ref expects 'artifact-id:digest'")
                    })?;
                    set_content(
                        &mut content,
                        ContentSource::ArtifactRef {
                            artifact_id: artifact_id.to_string(),
                            digest: digest.to_string(),
                        },
                    )?;
                }
                "--" => {
                    for positional in &args[i + 1..] {
                        append_positional(&mut fields, positional);
                    }
                    break;
                }
                _ if is_primary_selector_flag(metadata.typed_fields, arg) => {
                    let kind = selector_kind(arg).ok_or_else(|| {
                        util::invariant(format!(
                            "primary selector option {arg} has no selector kind for {command_key}"
                        ))
                    })?;
                    if selector.is_some() {
                        return Err(util::input_error(
                            "multiple primary selectors are not allowed",
                        ));
                    }
                    let value = take_value(args, &mut i, arg)?;
                    selector = Some(json!({ "kind": kind, "value": value }));
                }
                _ if is_cas_flag(arg) => {
                    let v = take_value(args, &mut i, arg)?;
                    let n = parse_int(&v, arg)?;
                    apply_cas(&mut cas_builder, arg, n)?;
                    cas_present = true;
                }
                _ if arg.starts_with("--") => {
                    // Generic command-specific flag. Invocation validation will
                    // reject names/types that are not present in typed_fields.
                    let key = arg.trim_start_matches("--").to_string();
                    let has_value = args
                        .get(i + 1)
                        .map(|next| !next.starts_with("--"))
                        .unwrap_or(false);
                    let val = if has_value {
                        let value = args
                            .get(i + 1)
                            .cloned()
                            .ok_or_else(|| util::input_error(format!("{arg} requires a value")))?;
                        i += 2;
                        Value::String(value)
                    } else {
                        i += 1;
                        Value::Bool(true)
                    };
                    put_field(&mut fields, &key, val);
                }
                _ => {
                    append_positional(&mut fields, arg);
                    i += 1;
                }
            }
        }

        let cas = cas_present.then_some(cas_builder);

        Ok(CliInput {
            command_key,
            global_options: global,
            selector,
            cas,
            content,
            fields: Value::Object(fields),
        })
    }

    /// Resolve the local instance id for isolated projection compatibility.
    /// Production orchestration must override this through verified discovery.
    pub fn instance_id(&self) -> InstanceId {
        if let Some(v) = &self.global_options.instance {
            InstanceId(v.clone())
        } else if let Some(v) = &self.global_options.profile {
            InstanceId(format!("profile:{v}"))
        } else {
            InstanceId("local".to_string())
        }
    }

    /// Resolve a parser-side principal compatibility ref. Production execution
    /// must replace this with the authenticated transport-derived principal.
    pub fn principal_ref(&self) -> PrincipalRef {
        let base = if let Some(v) = &self.global_options.profile {
            v.clone()
        } else if let Some(v) = &self.global_options.instance {
            v.clone()
        } else {
            "local".to_string()
        };
        PrincipalRef(format!("principal:{base}"))
    }

    /// The primary target selector value (if any), as a plain string.
    pub fn selector_value(&self) -> Option<String> {
        self.selector
            .as_ref()
            .and_then(|selector| selector.get("value"))
            .and_then(Value::as_str)
            .map(str::to_owned)
    }
}

fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, DxbotError> {
    let value_index = i
        .checked_add(1)
        .ok_or_else(|| util::input_error(format!("{flag} index overflow")))?;
    let value = args
        .get(value_index)
        .cloned()
        .ok_or_else(|| util::input_error(format!("{flag} requires a value")))?;
    *i = value_index
        .checked_add(1)
        .ok_or_else(|| util::input_error(format!("{flag} index overflow")))?;
    Ok(value)
}

fn parse_int(v: &str, flag: &str) -> Result<i64, DxbotError> {
    v.parse::<i64>()
        .map_err(|_| util::input_error(format!("{flag} expects an integer, got '{v}'")))
}

fn set_content(slot: &mut Option<ContentSource>, content: ContentSource) -> Result<(), DxbotError> {
    if slot.is_some() {
        return Err(util::input_error(
            "multiple content sources are not allowed",
        ));
    }
    *slot = Some(content);
    Ok(())
}

fn append_positional(fields: &mut serde_json::Map<String, Value>, arg: &str) {
    let positional = fields
        .entry("positional".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(values) = positional {
        values.push(Value::String(arg.to_string()));
    }
}

fn put_field(fields: &mut serde_json::Map<String, Value>, key: &str, value: Value) {
    match fields.get_mut(key) {
        Some(Value::Array(values)) => values.push(value),
        Some(other) => {
            let previous = other.clone();
            fields.insert(key.to_string(), json!([previous, value]));
        }
        None => {
            fields.insert(key.to_string(), value);
        }
    }
}

/// Maps a user-facing primary selector flag to a stable selector kind.
fn selector_kind(flag: &str) -> Option<&'static str> {
    match flag {
        "--bot" => Some("bot"),
        "--conversation" => Some("conversation"),
        "--thread" => Some("thread"),
        "--task" => Some("task"),
        "--project" => Some("project"),
        "--channel" => Some("channel"),
        "--scope" | "--owner" => Some("scope"),
        "--memory" => Some("memory"),
        "--proposal" => Some("proposal"),
        "--provider" => Some("provider"),
        "--process" => Some("process"),
        "--operation" => Some("operation"),
        "--approval" => Some("approval"),
        "--side-effect" => Some("side-effect"),
        _ => None,
    }
}

fn is_cas_flag(flag: &str) -> bool {
    matches!(
        flag,
        "--if-revision"
            | "--if-generation"
            | "--if-host-generation"
            | "--if-execution-generation"
            | "--if-source-revision"
            | "--if-scope-revision"
            | "--if-project-revision"
            | "--if-channel-revision"
            | "--if-membership-generation"
            | "--if-proposal-revision"
            | "--if-target-scope-revision"
            | "--if-receipt-revision"
    )
}

fn apply_cas(cas: &mut CasConditions, flag: &str, value: i64) -> Result<(), DxbotError> {
    match flag {
        "--if-revision" => cas.if_revision = Some(value),
        "--if-generation" => cas.if_generation = Some(value),
        "--if-host-generation" => cas.if_host_generation = Some(value),
        "--if-execution-generation" => cas.if_execution_generation = Some(value),
        "--if-source-revision" => cas.if_source_revision = Some(value),
        "--if-scope-revision" => cas.if_scope_revision = Some(value),
        "--if-project-revision" => cas.if_project_revision = Some(value),
        "--if-channel-revision" => cas.if_channel_revision = Some(value),
        "--if-membership-generation" => cas.if_membership_generation = Some(value),
        "--if-proposal-revision" => cas.if_proposal_revision = Some(value),
        "--if-target-scope-revision" => cas.if_target_scope_revision = Some(value),
        "--if-receipt-revision" => cas.if_receipt_revision = Some(value),
        _ => return Err(util::input_error(format!("unknown CAS flag '{flag}'"))),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    fn sl(args: &[&str]) -> Vec<String> {
        args.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn valued_options_are_consumed_exactly_once() {
        let input = CliInput::parse(&sl(&[
            "conversation-send",
            "--profile",
            "work",
            "--instance",
            "prod-1",
            "--format",
            "json",
            "--color",
            "never",
            "--wait",
            "committed",
            "--timeout",
            "30s",
            "--conversation",
            "conversation-a",
            "--text",
            "hello",
        ]))
        .unwrap();
        assert_eq!(input.global_options.profile.as_deref(), Some("work"));
        assert_eq!(input.global_options.instance.as_deref(), Some("prod-1"));
        assert_eq!(input.selector_value().as_deref(), Some("conversation-a"));
        assert_eq!(input.fields.get("positional"), None);
        assert_eq!(
            input.content,
            Some(ContentSource::Text {
                value: "hello".to_owned()
            })
        );
    }

    #[test]
    fn task_submit_owner_is_primary_selector() {
        let input = CliInput::parse(&sl(&[
            "task-submit",
            "--owner",
            "project:alpha",
            "--text",
            "work",
        ]))
        .unwrap();
        assert_eq!(input.selector_value().as_deref(), Some("project:alpha"));
        assert_eq!(input.selector.as_ref().unwrap()["kind"], "scope");
        assert!(input.fields.get("owner").is_none());
    }

    #[test]
    fn memory_get_scope_remains_secondary_semantic_field() {
        let input = CliInput::parse(&sl(&[
            "memory-get",
            "--memory",
            "memory-a",
            "--scope",
            "project:alpha",
        ]))
        .unwrap();
        assert_eq!(input.selector_value().as_deref(), Some("memory-a"));
        assert_eq!(input.fields["scope"], "project:alpha");
    }

    #[test]
    fn parses_selector_cas_without_positional_replay() {
        let input = CliInput::parse(&sl(&[
            "task-cancel",
            "--task",
            "scoped/alpha",
            "--if-revision",
            "7",
            "--if-execution-generation",
            "2",
        ]))
        .unwrap();
        assert_eq!(input.selector_value().as_deref(), Some("scoped/alpha"));
        let cas = input.cas.unwrap();
        assert_eq!(cas.if_revision, Some(7));
        assert_eq!(cas.if_execution_generation, Some(2));
        assert_eq!(input.fields.get("positional"), None);
    }

    #[test]
    fn rejects_unknown_command_key() {
        let err = CliInput::parse(&sl(&["nope"])).unwrap_err();
        assert!(err.message.contains("unknown command key"));
    }

    #[test]
    fn rejects_multiple_content_sources() {
        let err =
            CliInput::parse(&sl(&["conversation-send", "--text", "a", "--stdin"])).unwrap_err();
        assert!(err.message.contains("multiple content sources"));
    }

    #[test]
    fn command_count_matches_registry() {
        assert_eq!(COMMAND_COUNT, 63);
    }
}

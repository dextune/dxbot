//! User-facing CLI input types parsed from argv.
//!
//! `CliInput` is the parser-side projection (per `DXB-IFC-042`): it holds the raw user
//! selector, optional `--if-*` CAS guards, the content source selection and command
//! specific fields. It deliberately does **not** own canonical materialization — that is
//! the responsibility of `crate::contract`.

use dxbot_core::types::*;
use dxbot_core::DxbotError;
use serde_json::json;
use serde_json::Value;

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
    /// options, an optional selector, optional `--if-*` CAS guards, a content source and
    /// command-specific fields.
    pub fn parse(args: &[String]) -> Result<CliInput, DxbotError> {
        let _ = command_count; // keep the registry scope explicit for diagnostics
        if args.is_empty() {
            return Err(util::input_error("missing command key"));
        }
        let command_key = args[0].clone();
        if !is_known_command(&command_key) {
            return Err(util::input_error(format!("unknown command key: '{command_key}'")));
        }

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
                    let v = take_value(args, &mut i, "--profile")?;
                    global.profile = Some(v);
                }
                "--instance" => {
                    let v = take_value(args, &mut i, "--instance")?;
                    global.instance = Some(v);
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
                    let v = take_value(args, &mut i, "--wait")?;
                    global.wait = Some(v);
                }
                "--timeout" => {
                    let v = take_value(args, &mut i, "--timeout")?;
                    global.timeout = Some(v);
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
                    for p in &args[i + 1..] {
                        append_positional(&mut fields, p);
                    }
                    break;
                }
                _ if selector_kind(arg).is_some() => {
                    let kind = selector_kind(arg).unwrap_or("selector");
                    if selector.is_some() {
                        return Err(util::input_error("multiple selectors are not allowed"));
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
                    // Generic command-specific flag.
                    let key = arg.trim_start_matches("--").to_string();
                    let has_value = args
                        .get(i + 1)
                        .map(|next| !next.starts_with("--"))
                        .unwrap_or(false);
                    i += 1;
                    let val = if has_value {
                        args.get(i)
                            .cloned()
                            .map(Value::String)
                            .ok_or_else(|| util::input_error(format!("{arg} requires a value")))?
                    } else {
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

        let cas = if cas_present { Some(cas_builder) } else { None };

        Ok(CliInput {
            command_key,
            global_options: global,
            selector,
            cas,
            content,
            fields: Value::Object(fields),
        })
    }

    /// Resolve the local instance id for this input.
    pub fn instance_id(&self) -> InstanceId {
        if let Some(v) = &self.global_options.instance {
            InstanceId(v.clone())
        } else if let Some(v) = &self.global_options.profile {
            InstanceId(format!("profile:{v}"))
        } else {
            InstanceId("local".to_string())
        }
    }

    /// Resolve the local principal ref for this input.
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

    /// The selector value (if any), as a plain string.
    pub fn selector_value(&self) -> Option<String> {
        self.selector
            .as_ref()
            .and_then(|s| s.get("value"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }
}

fn take_value(args: &[String], i: &mut usize, flag: &str) -> Result<String, DxbotError> {
    *i += 1;
    args.get(*i)
        .cloned()
        .ok_or_else(|| util::input_error(format!("{flag} requires a value")))
}

fn parse_int(v: &str, flag: &str) -> Result<i64, DxbotError> {
    v.parse::<i64>()
        .map_err(|_| util::input_error(format!("{flag} expects an integer, got '{v}'")))
}

fn set_content(slot: &mut Option<ContentSource>, c: ContentSource) -> Result<(), DxbotError> {
    if slot.is_some() {
        return Err(util::input_error("multiple content sources are not allowed"));
    }
    *slot = Some(c);
    Ok(())
}

fn append_positional(fields: &mut serde_json::Map<String, Value>, arg: &str) {
    let pos = fields
        .entry("positional".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(arr) = pos {
        arr.push(Value::String(arg.to_string()));
    }
}

fn put_field(fields: &mut serde_json::Map<String, Value>, key: &str, val: Value) {
    match fields.get_mut(key) {
        Some(Value::Array(arr)) => arr.push(val),
        Some(other) => {
            let prev = other.clone();
            let list = json!([prev, val]);
            fields.insert(key.to_string(), list);
        }
        None => {
            fields.insert(key.to_string(), val);
        }
    }
}

/// Maps a user-facing selector flag to a stable selector kind.
fn selector_kind(flag: &str) -> Option<&'static str> {
    match flag {
        "--bot" => Some("bot"),
        "--conversation" => Some("conversation"),
        "--thread" => Some("thread"),
        "--task" => Some("task"),
        "--project" => Some("project"),
        "--channel" => Some("channel"),
        "--scope" => Some("scope"),
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

fn apply_cas(cas: &mut CasConditions, flag: &str, n: i64) -> Result<(), DxbotError> {
    match flag {
        "--if-revision" => cas.if_revision = Some(n),
        "--if-generation" => cas.if_generation = Some(n),
        "--if-host-generation" => cas.if_host_generation = Some(n),
        "--if-execution-generation" => cas.if_execution_generation = Some(n),
        "--if-source-revision" => cas.if_source_revision = Some(n),
        "--if-scope-revision" => cas.if_scope_revision = Some(n),
        "--if-project-revision" => cas.if_project_revision = Some(n),
        "--if-channel-revision" => cas.if_channel_revision = Some(n),
        "--if-membership-generation" => cas.if_membership_generation = Some(n),
        "--if-proposal-revision" => cas.if_proposal_revision = Some(n),
        "--if-target-scope-revision" => cas.if_target_scope_revision = Some(n),
        "--if-receipt-revision" => cas.if_receipt_revision = Some(n),
        _ => return Err(util::input_error(format!("unknown CAS flag '{flag}'"))),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn sl(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parses_command_key_and_globals() {
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
            "--yes",
        ]))
        .unwrap();
        assert_eq!(input.command_key, "conversation-send");
        assert_eq!(input.global_options.profile.as_deref(), Some("work"));
        assert_eq!(input.global_options.instance.as_deref(), Some("prod-1"));
        assert_eq!(input.global_options.format, OutputFormat::Json);
        assert_eq!(input.global_options.color, ColorMode::Never);
        assert_eq!(input.global_options.wait.as_deref(), Some("committed"));
        assert_eq!(input.global_options.timeout.as_deref(), Some("30s"));
        assert!(input.global_options.yes);
    }

    #[test]
    fn parses_selector_cas_and_content() {
        let input = CliInput::parse(&sl(&[
            "task-cancel",
            "--task",
            "scoped/alpha",
            "--if-revision",
            "7",
            "--if-execution-generation",
            "2",
            "--text",
            "out of budget",
        ]))
        .unwrap();
        assert_eq!(input.selector_value().as_deref(), Some("scoped/alpha"));
        let sel = input.selector.clone().unwrap();
        assert_eq!(sel["kind"], "task");
        let cas = input.cas.unwrap();
        assert_eq!(cas.if_revision, Some(7));
        assert_eq!(cas.if_execution_generation, Some(2));
        assert_eq!(
            input.content,
            Some(ContentSource::Text {
                value: "out of budget".to_string()
            })
        );
    }

    #[test]
    fn rejects_unknown_command_key() {
        let err = CliInput::parse(&sl(&["nope"])).unwrap_err();
        assert!(err.message.contains("unknown command key"));
    }

    #[test]
    fn rejects_multiple_content_sources() {
        let err = CliInput::parse(&sl(&["conversation-send", "--text", "a", "--stdin"])).unwrap_err();
        assert!(err.message.contains("multiple content sources"));
    }

    #[test]
    fn command_count_matches_registry() {
        assert_eq!(COMMAND_COUNT, 63);
    }
}
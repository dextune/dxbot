//! Registry-driven user invocation binding.
//!
//! `CliInput::parse` intentionally parses one canonical `command_key`. This
//! module is the only bridge from user-facing path positionals to the frozen
//! typed-field registry. It prevents the binary from growing a second 63-command
//! parser table while keeping parser-only local options out of wire ownership.

use dxbot_core::DxbotError;
use dxbot_core::types::ContentSource;
use serde_json::{Map, Value, json};

use crate::util;
use crate::{CliInput, CommandMetadata, metadata_for_key};

pub const MAX_CLI_TOKENS: usize = 4096;
pub const MAX_CLI_TOKEN_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FieldSpec {
    name: String,
    type_name: String,
    source: String,
    required: bool,
}

/// Parse canonical command-key argv and then bind remaining user positionals to
/// the registry's typed field order.
pub fn parse_bound_input(args: &[String]) -> Result<CliInput, DxbotError> {
    validate_argv_bounds(args)?;
    let mut input = CliInput::parse(args)?;
    let metadata = metadata_for_key(&input.command_key).ok_or_else(|| {
        util::invariant(format!(
            "known command '{}' has no metadata row",
            input.command_key
        ))
    })?;
    bind_positionals(&mut input, metadata)?;
    validate_required_fields(&input, metadata)?;
    Ok(input)
}

fn validate_argv_bounds(args: &[String]) -> Result<(), DxbotError> {
    if args.len() > MAX_CLI_TOKENS {
        return Err(util::input_error(format!(
            "too many CLI tokens: {} > {}",
            args.len(),
            MAX_CLI_TOKENS
        )));
    }
    if let Some((index, token)) = args
        .iter()
        .enumerate()
        .find(|(_, token)| token.len() > MAX_CLI_TOKEN_BYTES)
    {
        return Err(util::input_error(format!(
            "CLI token {index} exceeds {} bytes",
            MAX_CLI_TOKEN_BYTES
        )));
    }
    Ok(())
}

fn bind_positionals(input: &mut CliInput, metadata: CommandMetadata) -> Result<(), DxbotError> {
    normalize_field_names(&mut input.fields)?;
    let specs = parse_field_specs(metadata.typed_fields);
    let positionals = take_positionals(&mut input.fields)?;
    if positionals.is_empty() {
        return Ok(());
    }

    let mut next = 0usize;
    for value in positionals {
        while next < specs.len() && !can_bind_positional(input, &specs[next]) {
            next += 1;
        }
        let Some(spec) = specs.get(next) else {
            return Err(util::input_error(format!(
                "unexpected positional argument '{value}' for {}",
                input.command_key
            )));
        };
        bind_one(input, spec, value)?;
        next += 1;
    }
    Ok(())
}

fn can_bind_positional(input: &CliInput, spec: &FieldSpec) -> bool {
    if !is_user_source(&spec.source) || is_cas_field(&spec.name) {
        return false;
    }
    if is_selector_type(&spec.type_name) {
        return input.selector.is_none();
    }
    if is_content_type(&spec.type_name) {
        return input.content.is_none();
    }
    !field_present(&input.fields, &spec.name)
}

fn bind_one(input: &mut CliInput, spec: &FieldSpec, value: String) -> Result<(), DxbotError> {
    if is_selector_type(&spec.type_name) {
        input.selector = Some(json!({
            "kind": selector_kind(spec),
            "value": value,
        }));
        return Ok(());
    }
    if is_content_type(&spec.type_name) {
        input.content = Some(ContentSource::Text { value });
        return Ok(());
    }
    let fields = input.fields.as_object_mut().ok_or_else(|| {
        util::invariant("CliInput fields must be an object before positional binding")
    })?;
    fields.insert(spec.name.clone(), Value::String(value));
    Ok(())
}

fn validate_required_fields(
    input: &CliInput,
    metadata: CommandMetadata,
) -> Result<(), DxbotError> {
    for spec in parse_field_specs(metadata.typed_fields) {
        if !spec.required || !is_user_source(&spec.source) {
            continue;
        }
        let present = if is_selector_type(&spec.type_name) {
            input.selector.is_some()
        } else if is_content_type(&spec.type_name) {
            input.content.is_some()
        } else if is_cas_field(&spec.name) {
            cas_field_present(input, &spec.name)
        } else {
            field_present(&input.fields, &spec.name)
        };
        if !present {
            return Err(util::input_error(format!(
                "{} requires field '{}'",
                input.command_key, spec.name
            )));
        }
    }
    Ok(())
}

fn normalize_field_names(fields: &mut Value) -> Result<(), DxbotError> {
    let object = fields
        .as_object_mut()
        .ok_or_else(|| util::invariant("CliInput fields must be a JSON object"))?;
    let mut normalized = Map::new();
    for (key, value) in std::mem::take(object) {
        let normalized_key = key.replace('-', "_");
        if normalized.contains_key(&normalized_key) {
            return Err(util::input_error(format!(
                "field '{normalized_key}' was provided more than once with conflicting spellings"
            )));
        }
        normalized.insert(normalized_key, value);
    }
    *object = normalized;
    Ok(())
}

fn take_positionals(fields: &mut Value) -> Result<Vec<String>, DxbotError> {
    let object = fields
        .as_object_mut()
        .ok_or_else(|| util::invariant("CliInput fields must be a JSON object"))?;
    let Some(value) = object.remove("positional") else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| util::invariant("positional field must be an array"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| util::invariant("positional argv value must be a string"))
        })
        .collect()
}

fn field_present(fields: &Value, name: &str) -> bool {
    fields
        .as_object()
        .is_some_and(|object| object.contains_key(name))
}

fn cas_field_present(input: &CliInput, name: &str) -> bool {
    let Some(cas) = input.cas else {
        return false;
    };
    match name {
        "if_revision" => cas.if_revision.is_some(),
        "if_generation" => cas.if_generation.is_some(),
        "if_host_generation" => cas.if_host_generation.is_some(),
        "if_execution_generation" => cas.if_execution_generation.is_some(),
        "if_source_revision" => cas.if_source_revision.is_some(),
        "if_scope_revision" => cas.if_scope_revision.is_some(),
        "if_project_revision" => cas.if_project_revision.is_some(),
        "if_channel_revision" => cas.if_channel_revision.is_some(),
        "if_membership_generation" => cas.if_membership_generation.is_some(),
        "if_proposal_revision" => cas.if_proposal_revision.is_some(),
        "if_target_scope_revision" => cas.if_target_scope_revision.is_some(),
        "if_receipt_revision" => cas.if_receipt_revision.is_some(),
        _ => false,
    }
}

fn is_cas_field(name: &str) -> bool {
    name.starts_with("if_")
}

fn is_selector_type(type_name: &str) -> bool {
    type_name.contains("Selector")
}

fn is_content_type(type_name: &str) -> bool {
    type_name.starts_with("ContentSource")
}

fn is_user_source(source: &str) -> bool {
    matches!(source, "argv" | "oneof")
}

fn selector_kind(spec: &FieldSpec) -> String {
    match spec.type_name.as_str() {
        value if value.contains("BotSelector") => "bot".to_owned(),
        value if value.contains("ConversationSelector") => "conversation".to_owned(),
        value if value.contains("ThreadSelector") => "thread".to_owned(),
        value if value.contains("TaskSelector") => "task".to_owned(),
        value if value.contains("ProjectSelector") => "project".to_owned(),
        value if value.contains("ChannelSelector") => "channel".to_owned(),
        value if value.contains("OperationSelector") => "operation".to_owned(),
        value if value.contains("ApprovalSelector") => "approval".to_owned(),
        value if value.contains("ProviderSelector") => "provider".to_owned(),
        value if value.contains("ProcessSelector") => "process".to_owned(),
        value if value.contains("MemorySelector") && spec.name == "proposal" => {
            "proposal".to_owned()
        }
        value if value.contains("MemorySelector") => "memory".to_owned(),
        value if value.contains("SideEffectSelector") => "side-effect".to_owned(),
        value if value.contains("ScopeSelector") => "scope".to_owned(),
        _ => spec.name.replace('_', "-"),
    }
}

fn parse_field_specs(source: &str) -> Vec<FieldSpec> {
    split_top_level(source)
        .into_iter()
        .filter_map(parse_field_spec)
        .collect()
}

fn parse_field_spec(raw: &str) -> Option<FieldSpec> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "none" {
        return None;
    }
    let (name, rest) = raw.split_once(':')?;
    let (typed, source) = rest.rsplit_once('@').unwrap_or((rest, "argv"));
    let required = typed.trim_end().ends_with('!') || typed.trim_end().ends_with('+');
    let type_name = typed
        .trim()
        .trim_end_matches(['!', '?', '*', '+'])
        .to_owned();
    let source = source
        .trim()
        .split(['{', '='])
        .next()
        .unwrap_or_default()
        .to_owned();
    Some(FieldSpec {
        name: name.trim().to_owned(),
        type_name,
        source,
        required,
    })
}

fn split_top_level(source: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in source.char_indices() {
        match ch {
            '<' | '[' | '(' | '{' => depth = depth.saturating_add(1),
            '>' | ']' | ')' | '}' => depth = depth.saturating_sub(1),
            ';' if depth == 0 => {
                parts.push(&source[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&source[start..]);
    parts
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn bot_create_positional_binds_name_without_binary_table() {
        let input = parse_bound_input(&args(&["bot-create", "alpha"]))
            .expect("bot-create must bind positional name");
        assert_eq!(input.fields["name"], "alpha");
    }

    #[test]
    fn task_show_positional_binds_selector_from_type() {
        let input = parse_bound_input(&args(&["task-show", "task-a"]))
            .expect("task-show must bind positional selector");
        assert_eq!(input.selector_value().as_deref(), Some("task-a"));
        assert_eq!(
            input.selector.as_ref().and_then(|value| value.get("kind")),
            Some(&json!("task"))
        );
    }

    #[test]
    fn conversation_send_binds_selector_and_text_content() {
        let input = parse_bound_input(&args(&[
            "conversation-send",
            "conversation-a",
            "hello",
        ]))
        .expect("conversation-send must bind selector and content");
        assert_eq!(input.selector_value().as_deref(), Some("conversation-a"));
        assert_eq!(
            input.content,
            Some(ContentSource::Text {
                value: "hello".to_owned()
            })
        );
    }

    #[test]
    fn local_fields_do_not_consume_positionals() {
        let input = parse_bound_input(&args(&[
            "bot-list",
            "50",
            "cursor-a",
        ]))
        .expect("bot-list page arguments must skip local all flag");
        assert_eq!(input.fields["page_size"], "50");
        assert_eq!(input.fields["cursor"], "cursor-a");
    }

    #[test]
    fn hyphenated_named_fields_are_normalized_to_registry_names() {
        let input = parse_bound_input(&args(&[
            "runtime-start",
            "--ready-at",
            "control",
        ]))
        .expect("runtime-start flag must normalize");
        assert_eq!(input.fields["ready_at"], "control");
    }
}

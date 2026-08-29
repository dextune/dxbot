//! Shared parser for the frozen command typed-field DSL.
//!
//! This is intentionally internal. Invocation binding, parser selector routing,
//! and wire-local field stripping consume the same parsed metadata so command
//! semantics are not reimplemented in three different string parsers.

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FieldSpec {
    pub name: String,
    pub type_name: String,
    pub source: String,
    pub required: bool,
    pub multiple: bool,
}

pub(crate) fn parse_field_specs(source: &str) -> Vec<FieldSpec> {
    split_top_level(source)
        .into_iter()
        .filter_map(parse_field_spec)
        .collect()
}

pub(crate) fn primary_selector_name(specs: &[FieldSpec]) -> Option<&str> {
    specs
        .iter()
        .find(|spec| {
            is_user_source(&spec.source)
                && is_selector_type(&spec.type_name)
                && spec.name != "owner_bot"
        })
        .map(|spec| spec.name.as_str())
}

pub(crate) fn is_primary_selector_flag(typed_fields: &str, flag: &str) -> bool {
    let normalized = flag.trim_start_matches("--").replace('-', "_");
    let specs = parse_field_specs(typed_fields);
    primary_selector_name(&specs).is_some_and(|name| name == normalized)
}

pub(crate) fn local_field_names(typed_fields: &str) -> Vec<&str> {
    split_top_level(typed_fields)
        .into_iter()
        .filter_map(|raw| {
            let raw = raw.trim();
            if raw.is_empty() {
                return None;
            }
            let (name, rest) = raw.split_once(':')?;
            let (_, source) = rest.rsplit_once('@').unwrap_or((rest, "argv"));
            source.trim().starts_with("local").then_some(name.trim())
        })
        .collect()
}

pub(crate) fn is_selector_type(type_name: &str) -> bool {
    type_name.contains("Selector")
}

pub(crate) fn is_content_type(type_name: &str) -> bool {
    type_name.starts_with("ContentSource")
}

pub(crate) fn is_user_source(source: &str) -> bool {
    matches!(source, "argv" | "oneof")
}

fn parse_field_spec(raw: &str) -> Option<FieldSpec> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "none" {
        return None;
    }
    let (name, rest) = raw.split_once(':')?;
    let (typed, source) = rest.rsplit_once('@').unwrap_or((rest, "argv"));
    let cardinality = typed.trim().chars().last();
    let required = matches!(cardinality, Some('!' | '+'));
    let multiple = matches!(cardinality, Some('*' | '+'));
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
        multiple,
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
    use super::*;

    #[test]
    fn nested_source_metadata_does_not_split_field() {
        let specs = parse_field_specs(
            "ready_at:ReadyAt?@local{process,storage,runtime,control}=control;timeout:Duration?@local",
        );
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].name, "ready_at");
        assert_eq!(specs[0].source, "local");
    }

    #[test]
    fn primary_selector_comes_from_typed_field_order() {
        let fields = "memory:MemorySelector!@argv{canonical-id};scope:MemoryScopeSelector?@argv";
        assert!(is_primary_selector_flag(fields, "--memory"));
        assert!(!is_primary_selector_flag(fields, "--scope"));
    }
}

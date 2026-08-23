//! Golden-file contract test support.
//!
//! `GoldenContract::generate_snapshot` produces a deterministic JSON transcription of all
//! 63 P0 command input schemas from the static registry. `verify_snapshot` checks it
//! against a stored golden file (exact diff). The generator output is the source of truth;
//! stored golden files must not be hand-edited (see the fixture layout guide).

use dxbot_core::DxbotError;
use serde_json::json;
use serde_json::Value;

use crate::registry::{command_count, REGISTRY};
use crate::util;

/// Snapshot generator name, embedded so a snapshot can always be traced to its source.
const GENERATOR: &str = "application-contract::golden::generate_snapshot";
/// Bump when the snapshot schema/meaning changes.
const SNAPSHOT_VERSION: u32 = 1;

/// Golden snapshot support.
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenContract {
    pub snapshot: String,
}

/// A single parsed typed input field (from the field DSL in `DXB-IFC-042`).
#[derive(Debug, Clone, PartialEq)]
pub struct TypedField {
    pub name: String,
    pub field_type: String,
    pub cardinality: String,
    pub source: String,
    pub items: Vec<String>,
    pub default: Option<String>,
}

impl GoldenContract {
    /// Generate the deterministic JSON snapshot of all 63 command input schemas.
    pub fn generate_snapshot() -> String {
        let commands: Vec<Value> = REGISTRY
            .iter()
            .map(|r| {
                let fields: Vec<Value> = parse_typed_fields(r.typed_fields)
                    .iter()
                    .map(|f| {
                        json!({
                            "name": f.name,
                            "type": f.field_type,
                            "cardinality": f.cardinality,
                            "source": f.source,
                            "items": f.items,
                            "default": f.default,
                        })
                    })
                    .collect();
                json!({
                    "command_key": r.command_key,
                    "input_schema": r.input_schema,
                    "target_schema": r.target_schema,
                    "kind": r.kind,
                    "output_schema": r.output_schema,
                    "security_class": r.security_class,
                    "wait": {
                        "default": r.wait_default,
                        "allowed": split_csv(r.wait_allowed),
                    },
                    "typed_fields": fields,
                })
            })
            .collect();

        let snapshot = json!({
            "generator": GENERATOR,
            "snapshot_version": SNAPSHOT_VERSION,
            "command_total": command_count(),
            "commands": commands,
        });

        serde_json::to_string_pretty(&snapshot).unwrap_or_else(|e| {
            panic!("golden snapshot must be serializable: {e}")
        })
    }

    /// Verify `expected` (the stored golden file) matches the freshly-generated snapshot.
    pub fn verify_snapshot(expected: &str) -> Result<(), DxbotError> {
        let generated = Self::generate_snapshot();
        if generated == expected {
            Ok(())
        } else {
            Err(util::invariant(
                "golden contract snapshot drifted from generator output; \
                 regenerate the golden file from generate_snapshot() without hand-editing",
            ))
        }
    }
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// Parse the semicolon-separated typed field DSL into structured [`TypedField`]s.
fn parse_typed_fields(dsl: &str) -> Vec<TypedField> {
    dsl.split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            parse_field(part).ok()
        })
        .collect()
}

/// Parse one field DSL entry: `name:Type<cardinality><source>{items}[=default]`.
fn parse_field(raw: &str) -> Result<TypedField, String> {
    let (name, rest) = raw
        .split_once(':')
        .ok_or_else(|| format!("field missing ':': {raw}"))?;
    let at = rest
        .find('@')
        .ok_or_else(|| format!("field missing source: {raw}"))?;

    let (field_type, cardinality) = split_type_cardinality(&rest[..at]);
    let (source, items, default) = parse_source_suffix(&rest[at + 1..], raw)?;

    Ok(TypedField {
        name: name.to_string(),
        field_type,
        cardinality,
        source: format!("@{source}"),
        items,
        default,
    })
}

fn split_type_cardinality(tp: &str) -> (String, String) {
    let markers = ['!', '?', '*', '+'];
    match tp.chars().last() {
        Some(c) if markers.contains(&c) => {
            let card = match c {
                '!' => "required-one",
                '?' => "optional-one",
                '*' => "zero-or-more",
                '+' => "one-or-more",
                _ => "single",
            };
            (tp[..tp.len() - 1].to_string(), card.to_string())
        }
        _ => (tp.to_string(), "single".to_string()),
    }
}

/// Parse the part after `@`: source token, optional `{items}` and optional `=default`.
fn parse_source_suffix(after: &str, raw: &str) -> Result<(String, Vec<String>, Option<String>), String> {
    let (token, rest) = match after.find('{') {
        Some(idx) => (&after[..idx], Some(&after[idx..])),
        None => (after, None),
    };

    let mut items = Vec::new();
    let mut default = None;

    if let Some(rest) = rest {
        let close = rest
            .find('}')
            .ok_or_else(|| format!("unterminated '{{' in field: {raw}"))?;
        let inner = &rest[1..close];
        for it in inner.split(',') {
            let it = it.trim();
            if !it.is_empty() {
                items.push(it.to_string());
            }
        }
        let tail = &rest[close + 1..];
        if let Some(eq) = tail.strip_prefix('=') {
            default = Some(eq.to_string());
        }
    }

    Ok((token.to_string(), items, default))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    #[test]
    fn snapshot_is_deterministic_and_covers_63_commands() {
        let a = GoldenContract::generate_snapshot();
        let b = GoldenContract::generate_snapshot();
        assert_eq!(a, b);
        assert!(!a.is_empty());

        let parsed: Value = serde_json::from_str(&a).expect("snapshot is valid json");
        assert_eq!(parsed["snapshot_version"], 1);
        assert_eq!(parsed["command_total"], 63);
        assert_eq!(parsed["commands"].as_array().map(Vec::len), Some(63));
        assert!(verify_stable(&a));
    }

    #[test]
    fn verify_snapshot_passes_on_equal_and_fails_on_drift() {
        let good = GoldenContract::generate_snapshot();
        assert!(GoldenContract::verify_snapshot(&good).is_ok());
        let drifted = good.replacen('{', "{\n  \"tampered\": true,", 1);
        assert!(GoldenContract::verify_snapshot(&drifted).is_err());
    }

    #[test]
    fn parses_complex_typed_field_dsl() {
        let fields = parse_typed_fields(
            "content:ContentSource!@oneof{text,input-file,stdin,artifact-ref};ready_at:ReadyAt?@local{process,storage}=control",
        );
        assert_eq!(fields.len(), 2);
        let content = &fields[0];
        assert_eq!(content.name, "content");
        assert_eq!(content.field_type, "ContentSource");
        assert_eq!(content.cardinality, "required-one");
        assert_eq!(content.source, "@oneof");
        assert_eq!(content.items, ["text", "input-file", "stdin", "artifact-ref"]);
        assert_eq!(content.default, None);

        let ready = &fields[1];
        assert_eq!(ready.cardinality, "optional-one");
        assert_eq!(ready.source, "@local");
        assert_eq!(ready.default.as_deref(), Some("control"));
    }

    fn verify_stable(s: &str) -> bool {
        GoldenContract::verify_snapshot(s).is_ok()
    }

    #[test]
    fn snapshot_json_uses_pretty_but_deterministic_layout() {
        let s = GoldenContract::generate_snapshot();
        assert!(s.starts_with('{'));
        assert!(s.contains('}'));
        // Indented (pretty) JSON, not single-line compact.
        assert!(s.contains("\n  \"generator\""));
        assert!(s.contains("\"command_key\""));
    }
}
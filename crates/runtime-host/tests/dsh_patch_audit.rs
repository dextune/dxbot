//! Repository owner test for the audited DSH ACP profile patch
//! (`DXB-DEL-068` H6/H7).
//!
//! The Runtime embeds `deploy/dsh/dsh-acp.patch.yaml` at compile time and
//! validates a deployment's on-disk patch against its digest. This test is the
//! canonical owner of the patch's *shape*: it asserts the composition is the
//! official list-form overlay (a YAML sequence of route rows) and never the
//! invalid `plugins:` mapping, never a direct DeepSeek default route, and never
//! a silent fallback.
//!
//! It is a strict, bounded, textual owner test: it does not pull in a YAML
//! dependency. Instead it asserts the exact structural rows the audited overlay
//! must contain and forbids the specific tokens that would reintroduce the
//! rejected mapping form, a DeepSeek default, or a fallback route.

#![allow(clippy::expect_used, clippy::panic)]
#![cfg(feature = "dsh-acp")]

const PATCH: &str = include_str!("../../../deploy/dsh/dsh-acp.patch.yaml");

/// The non-comment (structural) lines of the patch, trimmed of trailing
/// whitespace. Comments (`#`) and blank lines are ignored so the shape
/// assertions are about the actual overlay content.
fn structural_lines() -> Vec<&'static str> {
    PATCH
        .lines()
        .map(str::trim_end)
        .filter(|line| {
            let t = line.trim_start();
            !t.is_empty() && !t.starts_with('#')
        })
        .collect()
}

#[test]
fn patch_is_bounded() {
    // The audited patch is intentionally minimal; bound it so an unbounded or
    // unexpectedly large composition cannot slip through review.
    assert!(
        PATCH.len() < 8 * 1024,
        "audited patch must stay small and reviewable (was {} bytes)",
        PATCH.len()
    );
    assert!(
        PATCH.len() > 256,
        "audited patch must carry the full audited overlay"
    );
}

#[test]
fn patch_is_list_form_not_plugins_mapping() {
    let lines = structural_lines();
    // The first structural line must begin a YAML sequence (`- ...`); the
    // official overlay is a list, not a `plugins:` mapping.
    let first = lines.first().expect("patch must have structural content");
    assert!(
        first.trim_start().starts_with("- "),
        "audited patch must be list-form (first structural line was {first:?})"
    );

    // The rejected mapping form must never reappear anywhere in the file, in
    // any indentation. A `plugins:` key is the exact invalid shape a prior
    // audit removed.
    for line in PATCH.lines() {
        let t = line.trim_start();
        assert!(
            !t.starts_with("plugins:"),
            "audited patch must never use the invalid `plugins:` mapping (line {line:?})"
        );
    }
}

#[test]
fn patch_contains_exact_required_rows() {
    let lines = structural_lines();
    // The overlay must contain each of these exact structural rows. These are
    // the audited route identities and the pinned MiniMax binding.
    for required in [
        "- id: llm-deepseek",
        "name: '@deepseek-ai/dsh-llm-deepseek'",
        "disabled: true",
        "- id: llm-pi-ai",
        "name: '@deepseek-ai/dsh-llm-pi-ai'",
        "apiKeyEnv: MINIMAX_API_KEY",
        "api: anthropic-messages",
        "baseURL: https://api.minimax.io/anthropic",
        "- id: MiniMax-M3",
        "- id: agent-default-model",
        "name: '@deepseek-ai/dsh-agent-default-model'",
        "provider: minimax",
        "model: MiniMax-M3",
        "- id: acp",
        "name: '@deepseek-ai/dsh-acp'",
        "inject: [acpAppStartup]",
    ] {
        assert!(
            lines.iter().any(|line| line.trim_start() == required),
            "audited patch must contain the exact row {required:?}"
        );
    }
}

#[test]
fn patch_disables_direct_deepseek_and_has_no_fallback() {
    // The direct DeepSeek adapter row must be present AND disabled: no alternate
    // provider, no silent model/provider fallback.
    let lines = structural_lines();
    let deepseek_row = lines
        .iter()
        .position(|line| line.trim_start() == "- id: llm-deepseek")
        .expect("direct deepseek row must exist to be disabled");
    // Within the two following structural lines the row must be disabled.
    let window = &lines[deepseek_row..(deepseek_row + 3).min(lines.len())];
    assert!(
        window
            .iter()
            .any(|line| line.trim_start() == "disabled: true"),
        "the direct DeepSeek route must be explicitly disabled"
    );

    // There must be no fallback token and no default DeepSeek provider/model in
    // the structural overlay content. Comments explaining the *absence* of a
    // fallback are legitimate and are ignored.
    for line in &lines {
        let t = line.trim_start();
        assert!(
            !t.to_ascii_lowercase().contains("fallback"),
            "audited patch must not declare any fallback route (line {line:?})"
        );
        assert!(
            !t.starts_with("provider: deepseek"),
            "audited patch must not default to a direct DeepSeek provider (line {line:?})"
        );
        assert!(
            !t.starts_with("model: deepseek"),
            "audited patch must not default to a DeepSeek model (line {line:?})"
        );
    }
}

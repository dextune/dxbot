//! Destructive / interactive confirmation (`AT-CLI-CONFIRM-001`).
//!
//! Confirmation never opens a prompt in non-interactive (non-TTY) context.
//! With `--yes` it auto-confirms regardless of TTY. Without `--yes` a prompt
//! is only offered on a controlling TTY. Destructive actions receive an extra
//! irreversible-action warning line.
//!
//! This module only depends on `dxbot-core` and standard types; it is at the
//! CLI boundary and does not import application or control-client crates.

#![forbid(unsafe_code)]
#![allow(clippy::module_name_repetitions)]

use dxbot_core::error::{DxbotError, ErrorCategory, ErrorCode};

/// The marker prefixed to destructive-action prompts.
const DESTRUCTIVE_WARNING: &str = "cannot be undone";

/// Errors produced by the confirmation module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationError {
    /// Confirmation was requested in a non-TTY context without `--yes`, so no
    /// prompt may be opened.
    NonInteractive,
    /// The TTY prompt could not be read.
    PromptIo,
}

impl ConfirmationError {
    /// Projects this error onto the canonical [`DxbotError`] surface.
    pub fn to_dxbot_error(&self) -> DxbotError {
        match self {
            Self::NonInteractive => DxbotError {
                code: ErrorCode::Usage,
                category: ErrorCategory::Input,
                message: "confirmation required: use --yes in non-interactive mode".to_string(),
                retryable: false,
                operation_ref: None,
                target_refs: Vec::new(),
                field_violations: Vec::new(),
                current_revision: None,
                current_generation: None,
                resume_cursor: None,
                next_actions: Vec::new(),
            },
            Self::PromptIo => DxbotError {
                code: ErrorCode::Interrupted,
                category: ErrorCategory::Local,
                message: "failed to read confirmation prompt".to_string(),
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

/// Confirmation gate for interactive and destructive actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Confirmation;

impl Default for Confirmation {
    fn default() -> Self {
        Self
    }
}

impl Confirmation {
    /// Requires confirmation for `prompt`.
    ///
    /// - Non-TTY: prompt is never opened. `--yes` auto-confirms; otherwise an
    ///   error is returned.
    /// - TTY: `--yes` auto-confirms; otherwise the user is prompted and the
    ///   result reflects their answer.
    pub fn require(prompt: &str, is_tty: bool, yes_flag: bool) -> Result<bool, ConfirmationError> {
        if yes_flag {
            // `--yes` auto-confirms in both interactive and non-interactive mode.
            return Ok(true);
        }
        if !is_tty {
            // No prompt in non-TTY without `--yes`.
            let _ = prompt;
            return Err(ConfirmationError::NonInteractive);
        }
        prompt_user(prompt)
    }

    /// Requires confirmation for the destructive `action` on `target`.
    ///
    /// Adds an extra irreversible-action warning line to the prompt. Confirms
    /// (`--yes`) or fails closed exactly like [`Confirmation::require`].
    pub fn require_destructive(
        action: &str,
        target: &str,
        is_tty: bool,
        yes_flag: bool,
    ) -> Result<bool, ConfirmationError> {
        Self::require(&Self::destructive_prompt(action, target), is_tty, yes_flag)
    }

    /// Builds the destructive confirmation prompt, including the extra
    /// irreversible-action warning. Exposed for testability.
    pub fn destructive_prompt(action: &str, target: &str) -> String {
        format!(
            "WARNING: {action} on {target} is destructive and {DESTRUCTIVE_WARNING}. Continue? [y/N] "
        )
    }
}

/// Prompts the user on the controlling TTY and reads a y/N answer.
fn prompt_user(prompt: &str) -> Result<bool, ConfirmationError> {
    use std::io::Write;
    print!("{prompt}");
    std::io::stdout()
        .flush()
        .map_err(|_| ConfirmationError::PromptIo)?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|_| ConfirmationError::PromptIo)?;
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => Ok(true),
        _ => Ok(false),
    }
}

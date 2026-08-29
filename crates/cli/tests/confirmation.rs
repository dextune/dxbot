//! Acceptance coverage for `AT-CLI-CONFIRM-001`: destructive confirmation.
//!
//! Verifies that prompts are never opened in non-TTY mode, `--yes` auto-confirms
//! in both modes, and destructive actions show an extra irreversible warning.
#![allow(clippy::unwrap_used)]

use cli::confirmation::{Confirmation, ConfirmationError};

#[test]
fn confirmation_non_tty_without_yes_returns_error() {
    // No prompt may be opened in non-TTY mode without `--yes`.
    assert_eq!(
        Confirmation::require("proceed?", false, false),
        Err(ConfirmationError::NonInteractive)
    );
    // The error projects to a normal usage error with exit code 2.
    assert_eq!(
        ConfirmationError::NonInteractive
            .to_dxbot_error()
            .code
            .exit_code(),
        2
    );
}

#[test]
fn confirmation_non_tty_with_yes_auto_confirms() {
    // `--yes` auto-confirms in non-interactive mode.
    assert_eq!(Confirmation::require("proceed?", false, true), Ok(true));
}

#[test]
fn confirmation_tty_with_yes_auto_confirms() {
    // `--yes` auto-confirms in interactive (TTY) mode too, without prompting.
    assert_eq!(Confirmation::require("proceed?", true, true), Ok(true));
}

#[test]
fn confirmation_destructive_shows_extra_warning() {
    // Destructive confirmation carries an extra irreversible-action warning
    // line that a plain prompt does not. The action and target are named.
    let destructive_prompt = Confirmation::destructive_prompt("archive", "bot alpha");
    assert!(
        destructive_prompt
            .to_lowercase()
            .contains("cannot be undone"),
        "destructive prompt must carry an irreversible warning: {destructive_prompt}"
    );
    assert!(
        destructive_prompt.contains("archive"),
        "destructive prompt must name the action"
    );
    assert!(
        destructive_prompt.contains("bot alpha"),
        "destructive prompt must name the target"
    );

    // The destructive confirmation gate routes through the same non-TTY/-yes
    // rules, but with the extra warning text in its prompt.
    assert_eq!(
        Confirmation::require_destructive("archive", "bot alpha", true, true),
        Ok(true)
    );
}

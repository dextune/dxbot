//! Always-present cancellation and usage primitives for the canonical Execute
//! contract.
//!
//! `CancellationToken` and `UsageInfo` are part of the Common contract and must
//! exist regardless of which production adapter slices are compiled in (the ACP
//! slice and the Common host use them even without the HTTP transport stack).
//! The HTTP `protocol` module re-uses these exact types so there is a single
//! canonical owner and no duplicated cancellation semantics.

#![forbid(unsafe_code)]

use tokio::sync::watch;

/// Level-triggered caller cancellation owned by provider Common.
///
/// The state is retained after cancellation, so a subscriber created after the
/// cancel request still observes it immediately. This avoids the lost-wakeup
/// semantics of using a bare notification as a cancellation token.
#[derive(Debug, Clone)]
pub struct CancellationToken {
    state: watch::Sender<bool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        let (state, _) = watch::channel(false);
        Self { state }
    }

    pub fn cancel(&self) {
        self.state.send_replace(true);
    }

    pub fn is_cancelled(&self) -> bool {
        *self.state.borrow()
    }

    /// Resolve once the token is cancelled (level-triggered).
    pub async fn cancelled(&self) {
        let mut receiver = self.state.subscribe();
        if *receiver.borrow_and_update() {
            return;
        }
        while receiver.changed().await.is_ok() {
            if *receiver.borrow_and_update() {
                return;
            }
        }
    }
}

/// Token usage reported by the upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub reasoning_tokens: Option<u32>,
}

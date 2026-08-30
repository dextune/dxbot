//! Common HTTP `ExecuteProvider` (`DXB-DEL-068` H2/H4).
//!
//! Wraps a model [`RealProvider`] adapter with the registration's immutable
//! protocol/transport binding and drives the canonical async
//! [`ExecuteProvider`] contract. This is where Common owns protocol execution,
//! deadline/cancellation, and output bounding for all HTTP-protocol providers.

#![forbid(unsafe_code)]

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use dxbot_core::types::ProviderId;

use crate::cancellation::{CancellationToken, UsageInfo};
use crate::execute::{
    ExecuteError, ExecuteFuture, ExecuteOutcome, ExecuteProvider, ExecuteRequest, ExecuteResult,
};
use crate::protocol::{
    ChatCompletionProtocol, ProviderError, ProviderEvent, ProviderExecuteConfig,
};
use crate::real_provider::RealProvider;
use crate::registration::TransportBinding;

/// Common HTTP execution wrapper around a model adapter.
pub struct HttpExecuteProvider {
    adapter: Box<dyn RealProvider>,
    protocol: ChatCompletionProtocol,
    max_output_bytes: usize,
    max_output_items: usize,
}

impl fmt::Debug for HttpExecuteProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpExecuteProvider")
            .field("id", self.adapter.id())
            .field("capability", &self.adapter.capability())
            .field("generation", &self.adapter.generation())
            .finish_non_exhaustive()
    }
}

impl HttpExecuteProvider {
    /// Bind a model adapter to a registration's immutable transport binding.
    pub fn new(
        adapter: Box<dyn RealProvider>,
        transport: &TransportBinding,
        max_output_bytes: usize,
        max_output_items: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            adapter,
            protocol: transport.protocol().clone(),
            max_output_bytes,
            max_output_items,
        })
    }
}

impl ExecuteProvider for HttpExecuteProvider {
    fn id(&self) -> &ProviderId {
        self.adapter.id()
    }

    fn capability(&self) -> &str {
        self.adapter.capability()
    }

    fn generation(&self) -> i64 {
        self.adapter.generation()
    }

    fn execute<'a>(
        &'a self,
        request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a> {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Ok(ExecuteResult::new(
                    self.adapter.id().clone(),
                    self.adapter.generation(),
                    ExecuteOutcome::Cancelled,
                ));
            }
            let deadline = deadline_instant(request.deadline)?;
            let provider_request = self.adapter.build_request(request);
            let config = ProviderExecuteConfig {
                deadline,
                cancel_token: Some(cancellation.clone()),
                max_output_bytes: self.max_output_bytes,
                max_output_items: self.max_output_items,
            };
            match self.protocol.execute(&provider_request, &config).await {
                Ok(events) => self.assemble_result(events),
                Err(ProviderError::Cancelled) => Ok(ExecuteResult::new(
                    self.adapter.id().clone(),
                    self.adapter.generation(),
                    ExecuteOutcome::Cancelled,
                )),
                Err(error) => Err(map_protocol_error(error)),
            }
        })
    }
}

impl HttpExecuteProvider {
    fn assemble_result(&self, events: Vec<ProviderEvent>) -> Result<ExecuteResult, ExecuteError> {
        let mut output = String::new();
        let mut usage = UsageInfo {
            prompt_tokens: 0,
            completion_tokens: 0,
            reasoning_tokens: None,
        };
        let mut completed = false;
        for event in events {
            match self.adapter.map_event(event) {
                ProviderEvent::ContentDelta { text, .. } => output.push_str(&text),
                ProviderEvent::ReasoningDelta { .. } => {}
                ProviderEvent::Completed { usage: reported } => {
                    usage = reported;
                    completed = true;
                }
                ProviderEvent::Failed { .. } => return Err(ExecuteError::ExecutionFailed),
                ProviderEvent::Cancelled => {
                    return Ok(ExecuteResult::new(
                        self.adapter.id().clone(),
                        self.adapter.generation(),
                        ExecuteOutcome::Cancelled,
                    ));
                }
            }
        }
        if !completed {
            return Err(ExecuteError::IncompleteStream);
        }
        Ok(ExecuteResult::new(
            self.adapter.id().clone(),
            self.adapter.generation(),
            ExecuteOutcome::Succeeded { output, usage },
        ))
    }
}

fn map_protocol_error(error: ProviderError) -> ExecuteError {
    match error {
        ProviderError::TransportUnavailable { .. } => ExecuteError::TransportUnavailable,
        ProviderError::UpstreamUnavailable { status, .. } => {
            ExecuteError::UpstreamUnavailable { status }
        }
        ProviderError::RateLimited { retry_after } => ExecuteError::RateLimited {
            retry_after_seconds: retry_after.map(|duration| duration.as_secs()),
        },
        ProviderError::InvalidRequest { status, .. } => ExecuteError::InvalidRequest { status },
        ProviderError::ProtocolViolation { .. } => ExecuteError::ProtocolViolation,
        ProviderError::OutputExceeded => ExecuteError::OutputExceeded,
        ProviderError::DeadlineExceeded => ExecuteError::DeadlineExceeded,
        // Cancellation is handled by the caller as a Cancelled outcome before
        // reaching this mapping; treat any residual as a generic failure.
        ProviderError::Cancelled => ExecuteError::ExecutionFailed,
    }
}

fn deadline_instant(deadline: Option<i64>) -> Result<Option<tokio::time::Instant>, ExecuteError> {
    let Some(deadline) = deadline else {
        return Ok(None);
    };
    let now_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let now_seconds = i64::try_from(now_seconds).unwrap_or(i64::MAX);
    let remaining = deadline
        .checked_sub(now_seconds)
        .filter(|seconds| *seconds > 0)
        .ok_or(ExecuteError::DeadlineExceeded)?;
    let remaining = u64::try_from(remaining).map_err(|_| ExecuteError::DeadlineExceeded)?;
    Ok(Some(
        tokio::time::Instant::now() + Duration::from_secs(remaining),
    ))
}

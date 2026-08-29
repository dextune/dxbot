#![forbid(unsafe_code)]

use std::future::Future;
use std::time::Duration;

use futures::StreamExt;
use serde::Deserialize;
use tokio::sync::watch;

use crate::transport::HttpTransport;

const PROTOCOL_OVERHEAD_BYTES: usize = 64 * 1024;
const ERROR_DETAIL_BYTES: usize = 16 * 1024;

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

    async fn cancelled(&self) {
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

/// A single chat message with role and content.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Model-specific fields that an extension provider supplies.
/// The common handler owns request building, transport, and parsing.
#[derive(Debug, Clone, PartialEq)]
pub struct ProviderRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

/// Token usage reported by the upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub reasoning_tokens: Option<u32>,
}

/// Unified output event produced by the common handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderEvent {
    ContentDelta { sequence: u64, text: String },
    ReasoningDelta { sequence: u64, text: String },
    Completed { usage: UsageInfo },
    Failed { reason: String },
    Cancelled,
}

/// Stable error from the common handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    TransportUnavailable { detail: String },
    InvalidRequest { status: u16, detail: String },
    RateLimited { retry_after: Option<Duration> },
    UpstreamUnavailable { status: u16, detail: String },
    DeadlineExceeded,
    Cancelled,
    ProtocolViolation { detail: String },
    OutputExceeded,
}

/// Execution configuration for a provider call.
#[derive(Debug, Clone)]
pub struct ProviderExecuteConfig {
    /// Optional caller-owned monotonic deadline.
    pub deadline: Option<tokio::time::Instant>,
    /// Optional level-triggered cancellation token.
    pub cancel_token: Option<CancellationToken>,
    /// Maximum cumulative output bytes before `OutputExceeded`.
    pub max_output_bytes: usize,
    /// Maximum number of output events, including the terminal event.
    pub max_output_items: usize,
}

/// OpenAI-compatible chat completion protocol handler.
/// Owns the wire format; extension providers only supply model-specific fields.
#[derive(Debug, Clone)]
pub struct ChatCompletionProtocol {
    pub transport: HttpTransport,
    pub max_output_bytes: usize,
    pub max_output_items: usize,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChoiceResponse>,
    usage: Option<UsageResponse>,
}

#[derive(Deserialize)]
struct ChoiceResponse {
    message: Option<MessageResponse>,
}

#[derive(Deserialize)]
struct MessageResponse {
    content: Option<String>,
    #[serde(rename = "reasoning_content")]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct UsageResponse {
    prompt_tokens: u32,
    completion_tokens: u32,
    #[serde(rename = "reasoning_tokens")]
    reasoning_tokens: Option<u32>,
}

#[derive(Deserialize)]
struct SseChunk {
    choices: Option<Vec<SseChoice>>,
    usage: Option<UsageResponse>,
}

#[derive(Deserialize)]
struct SseChoice {
    delta: Option<DeltaResponse>,
}

#[derive(Deserialize)]
struct DeltaResponse {
    content: Option<String>,
    #[serde(rename = "reasoning_content")]
    reasoning_content: Option<String>,
}

impl ChatCompletionProtocol {
    pub fn new(transport: HttpTransport, max_output_bytes: usize, max_output_items: usize) -> Self {
        Self {
            transport,
            max_output_bytes,
            max_output_items,
        }
    }

    /// Executes one bounded provider request under the same deadline and
    /// cancellation authority for connect, body reads, and streaming reads.
    pub async fn execute(
        &self,
        request: &ProviderRequest,
        config: &ProviderExecuteConfig,
    ) -> Result<Vec<ProviderEvent>, ProviderError> {
        self.check_control(config)?;

        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|message| {
                serde_json::json!({"role": message.role, "content": message.content})
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": request.stream,
        });
        let url = format!(
            "{}/v1/chat/completions",
            self.transport.base_url.trim_end_matches('/')
        );
        let response = self
            .controlled(self.transport.client.post(&url).json(&body).send(), config)
            .await?
            .map_err(map_reqwest_error)?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(ProviderError::RateLimited { retry_after });
        }
        if status.is_client_error() {
            let detail = self.read_error_detail(response, config).await?;
            return Err(ProviderError::InvalidRequest {
                status: status.as_u16(),
                detail,
            });
        }
        if status.is_server_error() {
            let detail = self.read_error_detail(response, config).await?;
            return Err(ProviderError::UpstreamUnavailable {
                status: status.as_u16(),
                detail,
            });
        }

        if request.stream {
            self.parse_stream(response, config).await
        } else {
            self.parse_non_stream(response, config).await
        }
    }

    async fn parse_non_stream(
        &self,
        response: reqwest::Response,
        config: &ProviderExecuteConfig,
    ) -> Result<Vec<ProviderEvent>, ProviderError> {
        let (max_bytes, max_items) = self.effective_limits(config);
        let wire_limit = max_bytes.saturating_add(PROTOCOL_OVERHEAD_BYTES);
        let bytes = self.read_body_limited(response, config, wire_limit).await?;
        let body: ChatCompletionResponse =
            serde_json::from_slice(&bytes).map_err(|error| ProviderError::ProtocolViolation {
                detail: error.to_string(),
            })?;

        let mut events = Vec::new();
        let mut total_bytes = 0usize;
        let mut sequence = 0u64;

        for choice in body.choices {
            let Some(message) = choice.message else {
                continue;
            };
            if let Some(content) = message.content {
                push_delta(
                    &mut events,
                    &mut total_bytes,
                    &mut sequence,
                    content,
                    false,
                    max_bytes,
                    max_items,
                )?;
            }
            if let Some(reasoning) = message.reasoning_content {
                push_delta(
                    &mut events,
                    &mut total_bytes,
                    &mut sequence,
                    reasoning,
                    true,
                    max_bytes,
                    max_items,
                )?;
            }
        }

        let usage = body.usage.map_or(
            UsageInfo {
                prompt_tokens: 0,
                completion_tokens: 0,
                reasoning_tokens: None,
            },
            usage_info,
        );
        push_completed(&mut events, usage, max_items)?;
        Ok(events)
    }

    async fn parse_stream(
        &self,
        response: reqwest::Response,
        config: &ProviderExecuteConfig,
    ) -> Result<Vec<ProviderEvent>, ProviderError> {
        let (max_bytes, max_items) = self.effective_limits(config);
        let max_frame_bytes = max_bytes.saturating_add(PROTOCOL_OVERHEAD_BYTES);
        let mut events = Vec::new();
        let mut total_bytes = 0usize;
        let mut sequence = 0u64;
        let mut buffer = Vec::<u8>::new();
        let mut stream = response.bytes_stream();

        loop {
            let next = self.controlled(stream.next(), config).await?;
            match next {
                Some(Ok(chunk)) => {
                    if buffer.len().saturating_add(chunk.len()) > max_frame_bytes {
                        return Err(ProviderError::OutputExceeded);
                    }
                    buffer.extend_from_slice(&chunk);
                }
                Some(Err(error)) => return Err(map_reqwest_error(error)),
                None => break,
            }

            while let Some((position, separator_len)) = find_sse_separator(&buffer) {
                let mut frame = buffer.drain(..position + separator_len).collect::<Vec<_>>();
                frame.truncate(position);
                if process_sse_frame(
                    &frame,
                    &mut events,
                    &mut total_bytes,
                    &mut sequence,
                    max_bytes,
                    max_items,
                )? {
                    return Ok(events);
                }
            }
        }

        if buffer.iter().any(|byte| !byte.is_ascii_whitespace()) {
            return Err(ProviderError::ProtocolViolation {
                detail: "stream ended with an incomplete SSE frame".to_string(),
            });
        }
        Err(ProviderError::ProtocolViolation {
            detail: "stream ended without a terminal SSE event".to_string(),
        })
    }

    fn effective_limits(&self, config: &ProviderExecuteConfig) -> (usize, usize) {
        (
            self.max_output_bytes.min(config.max_output_bytes),
            self.max_output_items.min(config.max_output_items),
        )
    }

    fn check_control(&self, config: &ProviderExecuteConfig) -> Result<(), ProviderError> {
        if config
            .cancel_token
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return Err(ProviderError::Cancelled);
        }
        if config
            .deadline
            .is_some_and(|deadline| deadline <= tokio::time::Instant::now())
        {
            return Err(ProviderError::DeadlineExceeded);
        }
        Ok(())
    }

    async fn controlled<T, F>(
        &self,
        future: F,
        config: &ProviderExecuteConfig,
    ) -> Result<T, ProviderError>
    where
        F: Future<Output = T>,
    {
        self.check_control(config)?;
        match (&config.cancel_token, config.deadline) {
            (Some(token), Some(deadline)) => {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => Err(ProviderError::Cancelled),
                    _ = tokio::time::sleep_until(deadline) => Err(ProviderError::DeadlineExceeded),
                    output = future => Ok(output),
                }
            }
            (Some(token), None) => {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => Err(ProviderError::Cancelled),
                    output = future => Ok(output),
                }
            }
            (None, Some(deadline)) => match tokio::time::timeout_at(deadline, future).await {
                Ok(output) => Ok(output),
                Err(_) => Err(ProviderError::DeadlineExceeded),
            },
            (None, None) => Ok(future.await),
        }
    }

    async fn read_body_limited(
        &self,
        response: reqwest::Response,
        config: &ProviderExecuteConfig,
        max_bytes: usize,
    ) -> Result<Vec<u8>, ProviderError> {
        let mut stream = response.bytes_stream();
        let mut body = Vec::with_capacity(max_bytes.min(8 * 1024));
        loop {
            let next = self.controlled(stream.next(), config).await?;
            match next {
                Some(Ok(chunk)) => {
                    if body.len().saturating_add(chunk.len()) > max_bytes {
                        return Err(ProviderError::OutputExceeded);
                    }
                    body.extend_from_slice(&chunk);
                }
                Some(Err(error)) => return Err(map_reqwest_error(error)),
                None => return Ok(body),
            }
        }
    }

    async fn read_error_detail(
        &self,
        response: reqwest::Response,
        config: &ProviderExecuteConfig,
    ) -> Result<String, ProviderError> {
        match self
            .read_body_limited(response, config, ERROR_DETAIL_BYTES)
            .await
        {
            Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
            Err(ProviderError::OutputExceeded) => {
                Ok("upstream error body exceeded diagnostic limit".to_string())
            }
            Err(error) => Err(error),
        }
    }
}

fn process_sse_frame(
    frame: &[u8],
    events: &mut Vec<ProviderEvent>,
    total_bytes: &mut usize,
    sequence: &mut u64,
    max_bytes: usize,
    max_items: usize,
) -> Result<bool, ProviderError> {
    let text = std::str::from_utf8(frame).map_err(|error| ProviderError::ProtocolViolation {
        detail: error.to_string(),
    })?;

    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let data = data.trim_start();
        if data == "[DONE]" {
            push_completed(
                events,
                UsageInfo {
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    reasoning_tokens: None,
                },
                max_items,
            )?;
            return Ok(true);
        }
        if data.is_empty() {
            continue;
        }

        let chunk: SseChunk =
            serde_json::from_str(data).map_err(|error| ProviderError::ProtocolViolation {
                detail: error.to_string(),
            })?;
        if let Some(usage) = chunk.usage {
            push_completed(events, usage_info(usage), max_items)?;
            return Ok(true);
        }
        if let Some(choices) = chunk.choices {
            for choice in choices {
                let Some(delta) = choice.delta else {
                    continue;
                };
                if let Some(content) = delta.content {
                    if !content.is_empty() {
                        push_delta(
                            events,
                            total_bytes,
                            sequence,
                            content,
                            false,
                            max_bytes,
                            max_items,
                        )?;
                    }
                }
                if let Some(reasoning) = delta.reasoning_content {
                    if !reasoning.is_empty() {
                        push_delta(
                            events,
                            total_bytes,
                            sequence,
                            reasoning,
                            true,
                            max_bytes,
                            max_items,
                        )?;
                    }
                }
            }
        }
    }
    Ok(false)
}

fn push_delta(
    events: &mut Vec<ProviderEvent>,
    total_bytes: &mut usize,
    sequence: &mut u64,
    text: String,
    reasoning: bool,
    max_bytes: usize,
    max_items: usize,
) -> Result<(), ProviderError> {
    let next_bytes = total_bytes.saturating_add(text.len());
    if next_bytes > max_bytes || events.len() >= max_items {
        return Err(ProviderError::OutputExceeded);
    }
    *total_bytes = next_bytes;
    let event = if reasoning {
        ProviderEvent::ReasoningDelta {
            sequence: *sequence,
            text,
        }
    } else {
        ProviderEvent::ContentDelta {
            sequence: *sequence,
            text,
        }
    };
    events.push(event);
    *sequence = (*sequence).saturating_add(1);
    Ok(())
}

fn push_completed(
    events: &mut Vec<ProviderEvent>,
    usage: UsageInfo,
    max_items: usize,
) -> Result<(), ProviderError> {
    if events.len() >= max_items {
        return Err(ProviderError::OutputExceeded);
    }
    events.push(ProviderEvent::Completed { usage });
    Ok(())
}

fn usage_info(usage: UsageResponse) -> UsageInfo {
    UsageInfo {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        reasoning_tokens: usage.reasoning_tokens,
    }
}

fn find_sse_separator(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(left), Some(right)) if left <= right => Some((left, 2)),
        (Some(_), Some(right)) => Some((right, 4)),
        (Some(position), None) => Some((position, 2)),
        (None, Some(position)) => Some((position, 4)),
        (None, None) => None,
    }
}

fn map_reqwest_error(error: reqwest::Error) -> ProviderError {
    ProviderError::TransportUnavailable {
        detail: error.to_string(),
    }
}

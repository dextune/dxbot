#![forbid(unsafe_code)]

use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::transport::HttpTransport;

// ── public data types ──────────────────────────────────────────────────────

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
    /// A content chunk (non-reasoning).
    ContentDelta { sequence: u64, text: String },
    /// A reasoning chunk (e.g. deepseek thinking).
    ReasoningDelta { sequence: u64, text: String },
    /// Terminal: completed with usage.
    Completed { usage: UsageInfo },
    /// Terminal: failed.
    Failed { reason: String },
    /// Terminal: cancelled.
    Cancelled,
}

/// Stable error from the common handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// HTTP transport failure (DNS, connection refused, TLS).
    TransportUnavailable { detail: String },
    /// HTTP 4xx — invalid request (model not found, bad API key).
    InvalidRequest { status: u16, detail: String },
    /// HTTP 429 — rate limited.
    RateLimited { retry_after: Option<Duration> },
    /// HTTP 5xx — upstream unavailable.
    UpstreamUnavailable { status: u16, detail: String },
    /// Deadline exceeded.
    DeadlineExceeded,
    /// Cancelled by caller.
    Cancelled,
    /// Response parsing failed (malformed SSE, JSON).
    ProtocolViolation { detail: String },
    /// Output exceeded bounded buffer.
    OutputExceeded,
}

/// Execution configuration for a provider call.
#[derive(Debug, Clone)]
pub struct ProviderExecuteConfig {
    /// Optional wall-clock deadline.
    pub deadline: Option<tokio::time::Instant>,
    /// Optional cancellation signal.
    pub cancel_notify: Option<Arc<tokio::sync::Notify>>,
    /// Maximum cumulative output bytes before `OutputExceeded`.
    pub max_output_bytes: usize,
    /// Maximum number of events before `OutputExceeded`.
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

// ── private deserialization helpers ────────────────────────────────────────

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

/// An SSE data chunk in the streaming path.
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

// ── ChatCompletionProtocol implementation ──────────────────────────────────

impl ChatCompletionProtocol {
    pub fn new(transport: HttpTransport, max_output_bytes: usize, max_output_items: usize) -> Self {
        Self {
            transport,
            max_output_bytes,
            max_output_items,
        }
    }

    /// Execute a provider request.
    ///
    /// Builds the OpenAI-compatible JSON body, POSTs to
    /// `{base_url}/v1/chat/completions`, and returns a sequence of
    /// [`ProviderEvent`]s.
    pub async fn execute(
        &self,
        request: &ProviderRequest,
        config: &ProviderExecuteConfig,
    ) -> Result<Vec<ProviderEvent>, ProviderError> {
        let body = serde_json::json!({
            "model": request.model,
            "messages": request.messages.iter().map(|m| {
                serde_json::json!({"role": m.role, "content": m.content})
            }).collect::<Vec<_>>(),
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stream": request.stream,
        });

        let url = format!(
            "{}/v1/chat/completions",
            self.transport.base_url.trim_end_matches('/')
        );

        let http_req = self.transport.client.post(&url).json(&body);

        let response = self.send_request(http_req, config).await?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(ProviderError::RateLimited { retry_after });
        }

        if status.is_client_error() {
            let detail = response
                .text()
                .await
                .unwrap_or_default();
            return Err(ProviderError::InvalidRequest {
                status: status.as_u16(),
                detail,
            });
        }

        if status.is_server_error() {
            let detail = response
                .text()
                .await
                .unwrap_or_default();
            return Err(ProviderError::UpstreamUnavailable {
                status: status.as_u16(),
                detail,
            });
        }

        if request.stream {
            self.parse_stream(response, config).await
        } else {
            self.parse_non_stream(response).await
        }
    }

    /// Send the HTTP request, respecting deadline and cancellation.
    async fn send_request(
        &self,
        req: reqwest::RequestBuilder,
        config: &ProviderExecuteConfig,
    ) -> Result<reqwest::Response, ProviderError> {
        let response_future = req.send();

        // Check cancel signal
        if let Some(notify) = &config.cancel_notify {
            tokio::select! {
                _ = notify.notified() => {
                    Err(ProviderError::Cancelled)
                }
                result = response_future => {
                    result.map_err(map_reqwest_error)
                }
            }
        } else if let Some(deadline) = config.deadline {
            match tokio::time::timeout_at(deadline, response_future).await {
                Ok(Ok(resp)) => Ok(resp),
                Ok(Err(e)) => Err(map_reqwest_error(e)),
                Err(_elapsed) => Err(ProviderError::DeadlineExceeded),
            }
        } else {
            response_future.await.map_err(map_reqwest_error)
        }
    }

    /// Parse a non-streaming JSON response.
    async fn parse_non_stream(
        &self,
        response: reqwest::Response,
    ) -> Result<Vec<ProviderEvent>, ProviderError> {
        let body: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| ProviderError::ProtocolViolation {
                detail: e.to_string(),
            })?;

        let mut events: Vec<ProviderEvent> = Vec::new();
        let mut total_bytes: usize = 0;
        let mut seq: u64 = 0;

        for choice in &body.choices {
            if let Some(msg) = &choice.message {
                if let Some(content) = &msg.content {
                    total_bytes = total_bytes.saturating_add(content.len());
                    if total_bytes > self.max_output_bytes
                        || events.len() >= self.max_output_items
                    {
                        return Err(ProviderError::OutputExceeded);
                    }
                    events.push(ProviderEvent::ContentDelta {
                        sequence: seq,
                        text: content.clone(),
                    });
                    seq = seq.saturating_add(1);
                }
                if let Some(reasoning) = &msg.reasoning_content {
                    total_bytes = total_bytes.saturating_add(reasoning.len());
                    if total_bytes > self.max_output_bytes
                        || events.len() >= self.max_output_items
                    {
                        return Err(ProviderError::OutputExceeded);
                    }
                    events.push(ProviderEvent::ReasoningDelta {
                        sequence: seq,
                        text: reasoning.clone(),
                    });
                    seq = seq.saturating_add(1);
                }
            }
        }

        let usage = body.usage.map_or(
            UsageInfo {
                prompt_tokens: 0,
                completion_tokens: 0,
                reasoning_tokens: None,
            },
            |u| UsageInfo {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                reasoning_tokens: u.reasoning_tokens,
            },
        );

        events.push(ProviderEvent::Completed { usage });
        Ok(events)
    }

    /// Parse an SSE streaming response.
    async fn parse_stream(
        &self,
        response: reqwest::Response,
        config: &ProviderExecuteConfig,
    ) -> Result<Vec<ProviderEvent>, ProviderError> {
        use futures::StreamExt;

        let mut events: Vec<ProviderEvent> = Vec::new();
        let mut total_bytes: usize = 0;
        let mut seq: u64 = 0;
        let mut buffer = String::new();

        let mut stream = response.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| ProviderError::TransportUnavailable {
                detail: e.to_string(),
            })?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // Process complete SSE events (delimited by "\n\n")
            while let Some(pos) = buffer.find("\n\n") {
                let event_str = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_str.lines() {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }

                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            let usage = UsageInfo {
                                prompt_tokens: 0,
                                completion_tokens: 0,
                                reasoning_tokens: None,
                            };
                            events.push(ProviderEvent::Completed { usage });
                            return Ok(events);
                        }

                        let chunk: SseChunk =
                            serde_json::from_str(data).map_err(|e| {
                                ProviderError::ProtocolViolation {
                                    detail: e.to_string(),
                                }
                            })?;

                        if let Some(usage_resp) = chunk.usage {
                            let usage = UsageInfo {
                                prompt_tokens: usage_resp.prompt_tokens,
                                completion_tokens: usage_resp.completion_tokens,
                                reasoning_tokens: usage_resp.reasoning_tokens,
                            };
                            events.push(ProviderEvent::Completed { usage });
                            return Ok(events);
                        }

                        if let Some(choices) = &chunk.choices {
                            for choice in choices {
                                if let Some(delta) = &choice.delta {
                                    if let Some(content) = &delta.content {
                                        if !content.is_empty() {
                                            total_bytes =
                                                total_bytes.saturating_add(content.len());
                                            if total_bytes > self.max_output_bytes
                                                || events.len() >= self.max_output_items
                                            {
                                                return Err(
                                                    ProviderError::OutputExceeded,
                                                );
                                            }
                                            events.push(ProviderEvent::ContentDelta {
                                                sequence: seq,
                                                text: content.clone(),
                                            });
                                            seq = seq.saturating_add(1);
                                        }
                                    }
                                    if let Some(reasoning) = &delta.reasoning_content {
                                        if !reasoning.is_empty() {
                                            total_bytes = total_bytes
                                                .saturating_add(reasoning.len());
                                            if total_bytes > self.max_output_bytes
                                                || events.len() >= self.max_output_items
                                            {
                                                return Err(
                                                    ProviderError::OutputExceeded,
                                                );
                                            }
                                            events.push(
                                                ProviderEvent::ReasoningDelta {
                                                    sequence: seq,
                                                    text: reasoning.clone(),
                                                },
                                            );
                                            seq = seq.saturating_add(1);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Check cancellation between chunks
            if let Some(notify) = &config.cancel_notify {
                // Notify::notified() is a future; use a quick poll
                if std::future::Future::poll(
                    std::pin::pin!(notify.notified()),
                    &mut std::task::Context::from_waker(std::task::Waker::noop()),
                )
                .is_ready()
                {
                    return Err(ProviderError::Cancelled);
                }
            }
        }

        // Stream ended without an explicit [DONE]
        let usage = UsageInfo {
            prompt_tokens: 0,
            completion_tokens: 0,
            reasoning_tokens: None,
        };
        events.push(ProviderEvent::Completed { usage });
        Ok(events)
    }
}

// ── helpers ────────────────────────────────────────────────────────────────

fn map_reqwest_error(e: reqwest::Error) -> ProviderError {
    if e.is_timeout() {
        ProviderError::DeadlineExceeded
    } else {
        ProviderError::TransportUnavailable {
            detail: e.to_string(),
        }
    }
}
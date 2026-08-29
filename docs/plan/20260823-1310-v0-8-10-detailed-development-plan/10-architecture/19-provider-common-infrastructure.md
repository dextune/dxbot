---
title: "Provider Common Infrastructure — HTTP·Async·Protocol Handler"
document_id: "DXB-ARC-019"
version: "0.8.11"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-017", "DXB-ARC-018"]
---
# Provider Common Infrastructure — HTTP·Async·Protocol Handler

## 1. 목적과 Scope

본 문서는 `provider-host`가 M5 canary 단계를 넘어 실제 외부 Provider와 통신하기 위해 필요한 **Common Infrastructure**를 정의한다. `DXB-ARC-012`와 `DXB-ARC-017`이 선언한 "Common이 lifecycle/transport를 소유하고 Extension은 adapter logic만 담당한다"는 원칙을 구현 수준으로 구체화한다.

### Scope

- HTTP transport layer (`reqwest` 기반)
- Async runtime (`tokio`)
- OpenAI-compatible chat completion protocol handler
- Streaming (SSE) response parser
- Bounded output buffer (item+byte cap)
- Deadline/cancellation propagation
- Stable error mapping

### 비범위

- 여러 transport protocol (gRPC, WebSocket) — P0는 HTTP/SSE만
- multi-provider registry, persistent credential vault, quota management — 후속 작업. P0 단일 production Provider의 owner-only config/credential-reference composition은 `ADR-0138`과 `DXB-RUN-035`가 소유한다.
- Plugin package/isolation boundary — `DXB-ARC-016` 영역
- Model-specific prompting 전략 — Extension 책임

---

## 2. Classification / Quality Tier

- **분류**: Common (Tier A)
- **Canonical Owner**: `provider-host` crate
- **Lifecycle Owner**: `ProviderHost` (기존)
- **Persistent State Owner**: 없음 (stateless transport layer)

---

## 3. Dependency 추가

`provider-host/Cargo.toml`에 추가:

```toml
[dependencies]
dxbot-core = { path = "../dxbot-core" }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }
tokio = { version = "1", features = ["rt", "sync", "time", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
bytes = "1"
```

- `rustls-tls` → `openssl` 대신 순수 Rust TLS (cross-platform)
- `stream` → SSE 응답 파싱
- `time` → deadline/timer

### 금지 dependency

- `openssl`, `native-tls` — 빌드 복잡성 증가
- Provider-specific SDK (openai, anthropic 등) — `provider-host`가 특정 Provider에 결합되지 않음
- `reqwest`의 `blocking` feature — 전체 async 유지

---

## 4. 상태와 상호작용

### 4.1 Transport Layer

```rust
/// Common HTTP transport owned by provider-host.
/// Extension providers do NOT create their own HTTP clients.
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    timeout: Duration,
}
```

- `ProviderHost`가 `HttpTransport`를 소유하고 Provider에게 `&self` 참조만 전달
- Provider는 `reqwest::Client`를 직접 생성하거나 소유하지 않음
- `base_url`은 Provider 등록 시 설정 (e.g., `http://localhost:10000`)

### 4.2 Protocol Handler

```rust
/// OpenAI-compatible chat completion protocol.
/// Common handler owns the wire format; Provider only supplies model-specific fields.
pub struct ChatCompletionProtocol {
    transport: HttpTransport,
    // Bounded output buffer
    max_output_bytes: usize,
    max_output_items: usize,
}
```

Provider가 제출하는 것은 **model-specific config**만:

```rust
/// Extension provider supplies only these fields.
/// Common handler owns request building, transport, parsing.
pub struct ProviderRequest {
    pub model: String,              // e.g., "alibaba/deepseek-v4-flash-0731"
    pub messages: Vec<ChatMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}
```

### 4.3 Response / Stream Event

```rust
/// Unified output from any provider, produced by Common handler.
pub enum ProviderEvent {
    /// A content chunk (non-reasoning).
    ContentDelta { sequence: u64, text: String },
    /// A reasoning chunk (deepseek-style thinking).
    ReasoningDelta { sequence: u64, text: String },
    /// Tool call requested (future).
    ToolCall { id: String, name: String, args: String },
    /// Terminal: completed with usage.
    Completed { usage: UsageInfo },
    /// Terminal: failed.
    Failed { reason: String },
    /// Terminal: cancelled.
    Cancelled,
}

pub struct UsageInfo {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub reasoning_tokens: Option<u32>,
}
```

### 4.4 Deadline / Cancellation

```rust
pub struct ProviderExecuteConfig {
    pub deadline: Option<tokio::time::Instant>,
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
    pub max_output_bytes: usize,
    pub max_output_items: usize,
}
```

- `deadline` 초과 → `ProviderEvent::Failed { reason: "deadline exceeded" }`
- `cancel_token` trigger → `ProviderEvent::Cancelled`
- Common handler가 `tokio::select!`로 deadline/cancel을 감시

---

## 5. Stable Error

```rust
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
```

- `ExternalUnknown`은 blind retry하지 않음 (`DXB-ARC-013` 규칙)
- `RateLimited` → `retry_after`가 있으면 caller가 backoff 결정
- `TransportUnavailable` / `UpstreamUnavailable` → transient, retryable

---

## 6. Resource / Bounded Execution

- `max_output_bytes`: 누적 컨텐츠 바이트 상한, 초과 시 `OutputExceeded`
- `max_output_items`: 이벤트 개수 상한, 초과 시 `OutputExceeded`
- Provider-local retry는 `RateLimited`에 한정 (observable semantic 불변)
- Provider output sink는 `ProviderHost`가 item+byte cap으로 감시

---

## 7. 기존 구조와의 통합

### 7.1 ProviderHost 확장

```rust
pub struct ProviderHost {
    reference: Option<ReferenceProvider>,
    adapter: Option<HarnessAdapter>,
    // v0.8.11 추가
    transport: Option<HttpTransport>,
    protocol: Option<ChatCompletionProtocol>,
    // 등록된 real provider들
    real_providers: Vec<Box<dyn RealProvider>>,
}
```

### 7.2 RealProvider trait (Extension SPI)

```rust
/// Extension providers implement this trait.
/// Common handler owns transport, lifecycle, output bounding.
pub trait RealProvider: Debug + Send + Sync {
    fn id(&self) -> &ProviderId;
    fn capability(&self) -> &str;
    fn generation(&self) -> i64;

    /// Build model-specific fields for the Common protocol handler.
    fn build_request(&self, task: &TaskDescription) -> ProviderRequest;

    /// Optional: post-process raw events into provider-specific
    /// evidence (default: passthrough).
    fn map_event(&self, event: ProviderEvent) -> ProviderEvent {
        event
    }
}
```

### 7.3 Provider Chain 확장

```
execute_task(task)
  → real_providers 먼저 시도 (등록 순)
    → Succeed: adapter 결과 반환
    → Unavailable: 다음 provider로
    → Fail: 에러 반환
  → HarnessAdapter (canary)
  → ReferenceProvider (폴백)
```

---

## 8. 검증

### Acceptance

| ID | 설명 | Fixture |
|---|---|---|
| AT-PROVIDER-INFRA-001 | HTTP transport + async runtime 정상 동작 | localhost:10000 proxy health check |
| AT-PROVIDER-INFRA-002 | OpenAI-compatible non-streaming completion | known prompt → known output assertion |
| AT-PROVIDER-INFRA-003 | SSE streaming parse (content + reasoning) | deepseek model chunk parse |
| AT-PROVIDER-INFRA-004 | Deadline exceeded → DeadlineExceeded error | short deadline + slow model |
| AT-PROVIDER-INFRA-005 | Output bounded → OutputExceeded | low max_output_bytes |
| AT-PROVIDER-INFRA-006 | Transport unavailable → TransportUnavailable | wrong port |
| AT-PROVIDER-INFRA-007 | Real provider chain fallback to Reference | unavailable real provider |
| AT-PROVIDER-INFRA-008 | 기존 canary test (AT-HARNESS-001) 회귀 없음 | existing 4 tests |

### Risk

| ID | 위험 | 영향 |
|---|---|---|
| R-101 | `reqwest` + `tokio` 추가로 빌드 시간 증가 | Low (workspace 수준) |
| R-102 | SSE 파싱이 proxy별 포맷 차이에 취약 | Medium (deepseek reasoning token이 특수) |
| R-103 | 외부 의존성으로 CI 재현성 저하 | Medium (proxy 필요) |

### Open Question

| ID | 질문 | Gate |
|---|---|---|
| OQ-090 | `reqwest` blocking feature를 완전히 배제할 수 있는가 | 구현 전 |
| OQ-091 | `RealProvider` trait 객체를 `Box<dyn>` vs enum 중 어느 것으로 할 것인가 | 구현 전 |

---

## 9. 변경 영향

| 대상 | 변경 |
|---|---|
| `provider-host/Cargo.toml` | `reqwest`, `tokio`, `futures`, `bytes` 추가 |
| `provider-host/src/lib.rs` | `HttpTransport`, `ChatCompletionProtocol`, `RealProvider` trait, `ProviderEvent`, `ProviderError` export |
| `provider-host/src/harness.rs` | `ProviderHost`에 `transport`, `protocol`, `real_providers` 필드 추가, `execute_task` chain 확장 |
| `provider-host/tests/harness.rs` | 신규 acceptance test 8건 추가 |
| `DXB-ARC-019` (본 문서) | 신규 |
| `DXB-DEL-060` | M5→M5a milestone 분리 또는 M5 범위 확장 |
| `DXB-DEL-061` | 신규 Acceptance 등록 |

---

## 10. 비목표 확인

- 범용 RPC/IDL framework ❌
- 여러 transport protocol (gRPC, WebSocket) ❌
- multi-provider registry / persistent credential vault / quota ❌ (P0 owner-only 단일 Provider config와 ephemeral credential reference는 `ADR-0138`/`DXB-RUN-035`)
- Plugin package/isolation ❌
- `ReferenceProvider`의 production fallback ❌
- Provider-specific SDK 의존성 ❌
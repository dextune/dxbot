---
title: "Provider Common Infrastructure — HTTP·Async·Protocol Handler"
document_id: "DXB-ARC-019"
version: "0.8.12"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-30"
depends_on: ["DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-017", "DXB-ARC-018"]
---
# Provider Common Infrastructure — HTTP·Async·Protocol Handler

## 1. 목적과 Scope

본 문서는 `provider-host`가 M5 canary 단계를 넘어 실제 외부 Provider와 통신하기 위해 필요한 **Common Infrastructure**를 정의한다. `DXB-ARC-012`와 `DXB-ARC-017`이 선언한 "Common이 lifecycle/transport를 소유하고 Extension은 adapter logic만 담당한다"는 원칙을 구현 수준으로 구체화한다.

### Scope

- HTTP transport layer (`reqwest` 기반)
- Async execution on a **Runtime-owned** `tokio` handle (no host-owned runtime)
- Canonical async Execute contract (`ExecuteProvider`/`ExecuteRequest`/`ExecuteEvent`/`ExecuteResult`)
- OpenAI-compatible Chat Completions protocol handler
- Anthropic-compatible Messages non-streaming protocol handler
- Streaming (SSE) response parser for OpenAI-compatible providers
- Per-registration immutable protocol/transport binding, generation, and limits
- Static `ProviderFactory` inventory (adapter key → schema/capability/constructor)
- Bounded output buffer (item+byte cap)
- Deadline/cancellation propagation
- Stable error mapping

### 비범위

- 여러 transport protocol (gRPC, WebSocket) — P0는 HTTP/SSE만
- persistent credential vault, quota management — 후속 작업. P0 단일 production Provider의 owner-only config/credential-reference composition은 `ADR-0138`과 `DXB-RUN-035`가 소유한다.
- Plugin package/isolation boundary — `DXB-ARC-016` 영역
- Model-specific prompting 전략 — Extension 책임
- 공식 DeepSeek Harness ACP subprocess adapter (`DXB-DEL-068` H6 이후)

---

## 2. Classification / Quality Tier

- **분류**: Common protocol/transport + canonical async Execute contract (Tier A), concrete MiniMax/DeepSeek adapters (Tier B Extension)
- **Canonical Owner**: `provider-host` crate
- **Lifecycle Owner**: `ProviderHost` (immutable registration + atomic replace/drain/unregister)
- **Async runtime Owner**: Runtime Host (`LocalRuntimeHost`) owns the `tokio` runtime and hands `ProviderHost` a `Handle` at the scheduler boundary. `ProviderHost` no longer owns a runtime or calls a nested `block_on` on its own runtime (`DXB-DEL-068` H2/H8).
- **Factory inventory Owner**: `provider-host::factory` static `ProviderFactory` inventory; Runtime config resolves adapter keys through it rather than a concrete `match` (`DXB-DEL-068` H5).
- **Persistent State Owner**: 없음 (stateless transport layer)
- **Wire selection**: 각 immutable registration이 자신의 protocol/transport binding을 소유하며 host-wide mutable protocol singleton은 없다 (`DXB-DEL-068` H4). Consumer는 분기하지 않는다.
- **Selection**: exact `ProviderId` + generation. Ready 전 selection, replaced/drained generation의 신규 admission, production reference/canary fallback을 금지한다.

---

## 3. Dependency 추가

`provider-host/Cargo.toml`에 추가:

```toml
[dependencies]
dxbot-core = { path = "../dxbot-core" }
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }
tokio = { version = "1", features = ["rt", "rt-multi-thread", "sync", "time", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
futures = "0.3"
bytes = "1"
```

`tokio`의 `rt-multi-thread`는 Runtime Host가 소유하는 multi-thread runtime handle을 위해 필요하다. `provider-host`는 이 handle을 주입받을 뿐 runtime을 직접 build하지 않는다(테스트 제외). Runtime Host crate(`runtime-host`)도 동일 정책의 pinned `tokio` (`rt`, `rt-multi-thread`, `time`)를 추가한다.

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
enum WireProtocol {
    OpenAiChatCompletions,
    AnthropicMessages,
}

/// Common text-generation protocol handler.
/// Wire flavor is selected by Runtime composition, never by the Consumer.
pub struct ChatCompletionProtocol {
    transport: HttpTransport,
    wire_protocol: WireProtocol,
    max_output_bytes: usize,
    max_output_items: usize,
}
```

OpenAI flavor는 base endpoint에 `/v1/chat/completions`를 붙이고 Bearer credential을 사용한다. Anthropic flavor는 configured API base(예: `https://api.minimax.io/anthropic/v1`)에 `/messages`를 붙이며 scoped `x-api-key`와 고정 `anthropic-version`을 Common transport가 설치한다. 두 flavor 모두 같은 deadline/cancellation, item+byte bound, stable error mapping을 사용한다. P0 Anthropic flavor는 non-streaming만 허용하며 stream 요청을 silent downgrade하지 않고 protocol violation으로 거부한다.

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
    pub cancel_token: Option<CancellationToken>,
    pub max_output_bytes: usize,
    pub max_output_items: usize,
}
```

- `deadline` 초과 → `ProviderError::DeadlineExceeded`
- `cancel_token` trigger → `ProviderError::Cancelled` (host maps to a `Cancelled` `ExecuteOutcome`)
- Common handler가 `tokio::select!`로 deadline/cancel을 감시
- `CancellationToken`은 level-triggered watch state이며 취소 후 구독자도 즉시 관측한다. `tokio_util`을 신규 도입하지 않는다.

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

### 7.0 Canonical async Execute contract (`DXB-DEL-068` H2)

reference/canary/direct adapter가 모두 하나의 async SPI를 구현한다.

```rust
pub trait ExecuteProvider: Debug + Send + Sync {
    fn id(&self) -> &ProviderId;
    fn capability(&self) -> &str;
    fn generation(&self) -> i64;
    fn execute<'a>(
        &'a self,
        request: &'a ExecuteRequest,
        cancellation: &'a CancellationToken,
    ) -> ExecuteFuture<'a>;
}
```

`ExecuteRequest`는 exact selected `provider_id`/`provider_generation`, bounded intent/context, budget, deadline만 담는다(raw credential·mutable Domain handle·external session identity 없음). `ExecuteResult`는 typed `ExecuteOutcome`(`Succeeded`/`Cancelled`)과 provider generation을 담고, `ExecuteError`는 safe한 category만 노출한다.

### 7.1 ProviderHost (immutable registration owner, shared-safe)

```rust
pub struct ProviderHost {
    handle: RwLock<Option<tokio::runtime::Handle>>, // Runtime-owned
    registry: RwLock<Registry>,                     // private synchronized slot registry
    next_activity_id: AtomicU64,                    // process-unique in-flight activity ids
}
```

- Host는 하나의 `Arc<ProviderHost>`로 Runtime scheduler·control server·lifecycle caller가 공유한다(`DXB-DEL-068` H9). 모든 `register`/`replace`/`begin_drain`/`unregister`/`reclaim_drained`/`drain_*` API는 `&self`이며 private `RwLock<Registry>`를 동기적으로 변경한다. 더 이상 `&mut self`·`Arc::get_mut`·no-op drain은 없다.
- Runtime Host가 `with_runtime_handle`/`set_runtime_handle`로 handle을 주입한다(`set_runtime_handle`도 `&self`). host는 runtime을 소유하지 않는다.
- `register`는 immutable `ProviderRegistration`을 발행하고 중복 id를 fail-close한다.
- `replace`는 하나의 write lock 안에서 기존 `Ready` generation을 `Draining`(no-new-admission)으로 전환하고 strictly-greater `Ready` generation을 발행한다. zero-lease draining generation은 즉시 완전히 reclaim되어 **선택 가능한 old generation을 남기지 않는다**. drain/replace/unregister 이후 *새로 시작하는* 호출은 fencing되고, 이미 provider clone과 activity lease를 잡은 in-flight 호출만 old fence 아래에서 완료할 수 있다.
- Execute admission은 registry write lock 안에서 exact `Ready`+generation slot을 관측하고 per-slot RAII activity lease(고유 activity id로 in-flight `CancellationToken` 추적)를 원자적으로 획득한 뒤 provider를 clone하고, lock을 해제한 다음에 `Handle::block_on`을 호출한다. host/slot lock을 provider 호출 동안 절대 잡지 않는다. `ProviderActivityRef` drop은 lease를 감소시키고 자신의 token을 제거한다.
- `begin_drain`/`unregister`는 captured old generation을 반환해 late result를 fencing한다. leased draining slot만 lease가 0으로 돌아올 때까지 retain된다.
- `drain_to_quiescence`(id별)와 `drain_all_to_quiescence`(all-provider shutdown drain)는 bounded deadline을 사용하고, deadline 초과 시 추적 중인 token을 cancel하며 절대 unbounded-wait하지 않는다.
- `HarnessAdapter`/`ReferenceProvider` special slot은 제거됐다.

### 7.2 Per-registration binding + ModelAdapter SPI

각 registration은 `ProtocolKind + TransportBinding + Generation + RegistrationLimits`를 소유한다(`DXB-DEL-068` H4). HTTP-protocol provider는 Common `HttpExecuteProvider`가 model adapter(`RealProvider`)를 감싸 canonical contract를 구현한다.

```rust
pub trait RealProvider: Debug + Send + Sync {
    fn id(&self) -> &ProviderId;
    fn capability(&self) -> &str;
    fn generation(&self) -> i64;
    fn build_request(&self, request: &ExecuteRequest) -> ProviderRequest;
    fn map_event(&self, event: ProviderEvent) -> ProviderEvent { event }
}
```

### 7.3 Static ProviderFactory inventory (`DXB-DEL-068` H5)

```rust
pub struct ProviderFactory {
    pub key: &'static str,
    pub schema_version: u32,
    pub capability_class: &'static str,
    pub protocol_kind: ProtocolKind,
    pub credential_mode: CredentialMode,
    pub build: fn(&FactoryConfig<'_>) -> Result<ProviderRegistration, FactoryError>,
}
```

Runtime config는 `lookup_factory(key)`로 factory를 조회하고 완전한 candidate registration을 만든 뒤 `register`한다. unknown key와 duplicate key는 typed fail-close이며 concrete adapter `match`, dynamic ABI, reflection은 금지한다.

### 7.4 Selection / execution path

```
scheduler (OS thread)
  → ProviderHost::execute_task_with_cancel(exact id, exact generation, task, cancel)
    → selectable_slot(id, generation) — exact match, no fallback
      → not found: ProviderNotFound (id absent) | GenerationFenced (id present, gen replaced)
    → Runtime-owned Handle::block_on(registration.provider().execute(request, cancel))
    → typed TaskResult / HarnessError
```

reference/canary는 production selection·fallback이 아니라 `testkit` feature 하에서만 존재한다(`DXB-DEL-068` H1). production `dxb` binary는 canary/reference symbol이 0이다.

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
| AT-PROVIDER-INFRA-007 | Unknown/replaced provider fails closed (no fallback) | exact id/generation selection |
| AT-PROVIDER-INFRA-008 | Test canary executes through canonical async contract | `testkit` canary |
| AT-HARNESS-H1 | production `dxb` symbol/config에 canary/reference 0 | built binary symbol scan |
| AT-HARNESS-H2 | reference/canary/direct가 하나의 async SPI, host-owned runtime 제거 | single `ExecuteProvider` + Runtime handle |
| AT-HARNESS-H3 | shared Conformance suite가 canary/reference를 동일 public path로 검증 | `ConformanceSuite::run_deterministic` |
| AT-HARNESS-H4 | 동시 Anthropic/OpenAI registration이 cross-route 0으로 실행 | per-registration binding isolation |
| AT-HARNESS-H5 | registry-derived config parse/unknown/duplicate fail-close | static `ProviderFactory` inventory |

### MiniMax M3 Extension evidence

- deterministic adapter request test: `MiniMax-M3`, bounded `max_tokens`, temperature 0, non-streaming
- local Host-path fixture: `POST /anthropic/v1/messages`, scoped `x-api-key`, `anthropic-version`, response/usage normalization, secret-safe `Debug`
- owner-only Runtime config: `adapter=minimax-m3`, `credential_ref=env:MINIMAX_API_KEY`, no secret projection
- opt-in live canary: OpenCode metadata endpoint/model/credential source를 사용한 non-empty external result
- built `dxb` journey: isolated Runtime start → provider doctor → Bot activate → Task submit/result → provider evidence → host stop

External canary의 비결정 output은 deterministic Host-path invariant를 대체하지 않는다.

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
| `provider-host/Cargo.toml` | `reqwest`, `tokio` (`rt-multi-thread` 포함), `futures`, `bytes`; `testkit` feature |
| `provider-host/src/lib.rs` | `HttpTransport`, dual-flavor `ChatCompletionProtocol`, `RealProvider`, `ExecuteProvider`/`ExecuteRequest`/`ExecuteResult`, `ProviderRegistration`/`TransportBinding`, `ProviderFactory`, DeepSeek/MiniMax adapters export |
| `provider-host/src/execute.rs` | canonical async Execute contract (신규) |
| `provider-host/src/registration.rs` | per-registration protocol/transport binding + limits (신규) |
| `provider-host/src/http_provider.rs` | Common `HttpExecuteProvider` bridge (신규) |
| `provider-host/src/factory.rs` | static `ProviderFactory` inventory (신규) |
| `provider-host/src/testkit.rs` | test-only `TestCanaryProvider`/`ReferenceProvider` + shared Conformance suite (신규, `cfg(feature="testkit")`) |
| `provider-host/src/harness.rs` | `ProviderHost`가 immutable registration·atomic replace/drain/unregister·exact selection 소유, host-owned runtime 제거 |
| `provider-host/src/minimax.rs`, `deepseek.rs` | `build_request(&ExecuteRequest)` model adapter |
| `runtime-host/Cargo.toml` | pinned `tokio` (`rt`, `rt-multi-thread`, `time`) |
| `runtime-host/src/local_runtime.rs` | Runtime-owned `tokio::runtime::Runtime`, provider handle 주입, shutdown 시 `provider-runtime-stopped` step |
| `runtime-host/src/provider_config.rs` | static factory registry lookup (concrete `match` 제거), typed unknown/duplicate fail-close |
| `runtime-host/src/scheduler.rs` | exact id+generation 실행 호출 |

---

## 10. 비목표 확인

- 범용 RPC/IDL framework ❌
- 여러 transport protocol (gRPC, WebSocket) ❌
- multi-provider registry / persistent credential vault / quota ❌ (P0 owner-only 단일 Provider config와 ephemeral credential reference는 `ADR-0138`/`DXB-RUN-035`)
- Plugin package/isolation ❌
- `ReferenceProvider`의 production fallback ❌
- Provider-specific SDK 의존성 ❌
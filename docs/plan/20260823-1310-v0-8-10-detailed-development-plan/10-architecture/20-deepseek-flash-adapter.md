---
title: "DeepSeek v4 Flash 0731 Adapter — 첫 Real Provider 구현"
document_id: "DXB-PRV-001"
version: "0.8.11"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-019", "DXB-ARC-013", "DXB-ARC-017"]
---
# DeepSeek v4 Flash 0731 Adapter — 첫 Real Provider 구현

## 1. 목적과 Scope

본 문서는 `DXB-ARC-019`(Provider Common Infrastructure) 위에서 동작하는 첫 번째 실제 Provider Adapter를 정의한다. 대상 모델은 `alibaba/deepseek-v4-flash-0731`이며, `localhost:10000`의 OpenAI 호환 프록시를 통해 호출한다.

### Scope

- `RealProvider` trait 구현
- deepseek reasoning token(`reasoning_content`) 처리
- `ProviderRequest` 구성 (model, messages, max_tokens, temperature)
- `ProviderEvent` mapping (reasoning/content 분리)

### 비범위

- 다른 deepseek variant (v4-pro, v3 등) — 별도 Provider
- 다른 모델 family (GPT, Claude, Gemini) — 별도 Provider
- 프록시 인증/credential 관리 — `DXB-ARC-019`에서 다루지 않음

---

## 2. Classification / Quality Tier

- **분류**: Extension (Tier B)
- **Canonical Owner**: `provider-host` crate 내 `deepseek` module
- **Lifecycle Owner**: `ProviderHost`
- **Persistent State Owner**: 없음

---

## 3. 구현 계약

### 3.1 DeepSeekFlashAdapter

```rust
/// DeepSeek v4 Flash 0731 adapter.
/// Implements `RealProvider` trait from `DXB-ARC-019`.
pub struct DeepSeekFlashAdapter {
    id: ProviderId,
    capability: String,
    generation: i64,
    model: String,       // "alibaba/deepseek-v4-flash-0731"
}
```

### 3.2 RealProvider 구현

```rust
impl RealProvider for DeepSeekFlashAdapter {
    fn id(&self) -> &ProviderId { &self.id }
    fn capability(&self) -> &str { &self.capability }
    fn generation(&self) -> i64 { self.generation }

    fn build_request(&self, task: &TaskDescription) -> ProviderRequest {
        ProviderRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "user".to_string(),
                    content: format!("{}\n\nContext: {}", task.intent, task.context),
                },
            ],
            max_tokens: task.budget.map(|b| b as u32),
            temperature: Some(0.0),  // deterministic for test reproducibility
            stream: false,           // non-streaming for simplicity
        }
    }

    fn map_event(&self, event: ProviderEvent) -> ProviderEvent {
        // deepseek reasoning tokens are already separated by Common handler.
        // Pass through without modification.
        event
    }
}
```

### 3.3 ProviderHost 등록

```rust
let mut host = ProviderHost::new();

// Common infrastructure
host.set_transport(HttpTransport::new("http://localhost:10000", Duration::from_secs(30)));

// Real provider
host.register_real_provider(Box::new(DeepSeekFlashAdapter::new(
    ProviderId("deepseek-flash".to_string()),
    "text",
    1,
)));

// ReferenceProvider (fallback)
host.register_reference_provider(ReferenceProvider::new(
    ProviderId("ref-1".to_string()),
    "text",
    1,
));
```

---

## 4. SSE Stream Parsing (deepseek 특수)

deepseek 모델은 SSE chunk에 `reasoning_content` 필드를 포함한다:

```json
{
  "choices": [{
    "delta": {
      "content": "",
      "reasoning_content": "We need to respond with..."
    }
  }]
}
```

Common handler (`DXB-ARC-019`)가 이를 파싱하여:

- `content` → `ProviderEvent::ContentDelta`
- `reasoning_content` → `ProviderEvent::ReasoningDelta`
- `usage` → `ProviderEvent::Completed { usage }`

로 분리한다. `DeepSeekFlashAdapter`는 `map_event`에서 passthrough한다.

---

## 5. Error Mapping

| Proxy 응답 | ProviderError |
|---|---|
| HTTP 200 + 유효 JSON | 정상 |
| HTTP 400 | `InvalidRequest` |
| HTTP 429 | `RateLimited` |
| HTTP 500/502/503 | `UpstreamUnavailable` |
| Connection refused | `TransportUnavailable` |
| Timeout | `DeadlineExceeded` |
| Malformed SSE | `ProtocolViolation` |

---

## 6. 검증

### Acceptance

| ID | 설명 | Fixture |
|---|---|---|
| AT-DEEPSEEK-001 | `DeepSeekFlashAdapter`가 `RealProvider` trait 구현 | `build_request` returns valid `ProviderRequest` |
| AT-DEEPSEEK-002 | 실제 프록시 호출 → non-empty response | `cargo test -p provider-host deepseek` |
| AT-DEEPSEEK-003 | reasoning token 포함된 SSE parse | deepseek-specific chunk fixture |
| AT-DEEPSEEK-004 | Provider chain: real adapter 우선, Reference 폴백 | real adapter unavailable scenario |
| AT-DEEPSEEK-005 | `temperature=0.0` → deterministic output (soft) | repeated calls produce consistent output |

### Test Fixture

```rust
#[test]
fn deepseek_flash_produces_non_empty_output() {
    let mut host = ProviderHost::new();
    host.set_transport(HttpTransport::new("http://localhost:10000", Duration::from_secs(30)));
    host.register_real_provider(Box::new(DeepSeekFlashAdapter::new(
        ProviderId("deepseek-flash".to_string()),
        "text",
        1,
    )));
    host.register_reference_provider(ReferenceProvider::new(
        ProviderId("ref-1".to_string()),
        "text",
        1,
    ));

    let task = TaskDescription {
        intent: "Say hello".to_string(),
        context: "".to_string(),
        budget: Some(50),
        deadline: None,
    };

    let result = host.execute_task(&task).unwrap();
    assert!(!result.output.is_empty());
    assert_eq!(result.status, TaskStatus::Completed);
}
```

---

## 7. Risk

| ID | 위험 | 영향 |
|---|---|---|
| R-110 | 프록시 비가용 시 테스트 실패 | Medium (CI에 proxy 필요) |
| R-111 | deepseek reasoning token이 향후 변경될 수 있음 | Low (Common handler가 버퍼) |
| R-112 | `temperature=0.0`이 항상 동일 출력을 보장하지 않음 | Low (soft assertion) |

---

## 8. 비목표 확인

- 다른 deepseek variant ❌
- 다른 모델 family ❌
- streaming mode (`stream: true`) ❌ — P0는 non-streaming만
- multi-turn conversation ❌ — 단일 메시지만
- system prompt / tool calling ❌
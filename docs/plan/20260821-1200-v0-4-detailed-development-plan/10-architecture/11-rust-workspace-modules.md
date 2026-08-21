---
title: "Rust Workspace와 모듈 경계"
document_id: "DXB-ARC-011"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-GOV-003"]
---

# Rust Workspace와 모듈 경계

## 1. 목적

의미 소유권과 실제 변형 축에 맞는 crate/module 경계를 유지한다. Conversation/Thread/Live Control을 새로운 Core Domain/Runtime 의미로 추가하되 책임마다 무조건 새 crate를 만드는 과추상을 금지한다.

## 2. 권장 Layout

v0.3의 `dxb-kernel`, `dxb-domain`, `dxb-application`, `dxb-storage`, Capability contract crates, `dxb-provider-sdk`, `dxb-provider-host`, `dxb-runtime`, `dxb-control`, interfaces, providers/plugins/testkit 구조를 유지한다.

초기 구현은 다음 책임을 명확한 module boundary로 먼저 둔다.

```text
dxb-domain
├─ bot
├─ conversation-thread
├─ memory
├─ goal-task
└─ execution

dxb-runtime
├─ bot-coordinator
├─ conversation-runtime
├─ task-runtime
├─ execution-supervisor
├─ dynamic-core-scheduler
├─ live-control
├─ recovery
└─ provider-host composition

dxb-control
├─ conversation-thread api
├─ command-query
├─ live-control protocol
└─ event stream
```

실제 Rust source file은 language-native `snake_case.rs`를 사용한다. 위 표기는 logical module 책임을 설명하기 위한 kebab-style 문서 표기다.

## 3. Domain 책임

`dxb-domain`은 Bot/Conversation/Thread/Memory/Goal/Task/Task Revision/Execution/Core semantic type과 pure invariant를 소유한다. Provider Session, WebSocket, runtime worker handle, external DTO를 포함하지 않는다.

Conversation/Thread가 커져 independent reuse/test/compile pressure가 확인되기 전 별도 crate를 미리 만들지 않는다.

## 4. Runtime 책임

### Conversation Runtime
- durable message/thread command orchestration
- thread routing/lineage service composition
- Memory/Task Port 호출
- Session-independent client attachment

### Execution Supervisor / Live Control
- committed Directive 관찰
- bounded Control Channel
- report/freshness
- cooperative safe-point/yield orchestration
- suspend/resume handoff

Supervisor가 Task state machine/Side Effect Ledger/Scheduler Lease를 직접 소유하지 않는다.

### Scheduler
Core Lease/fairness/admission을 소유한다. control hint를 입력으로 읽되 fixed worker/core count mutator API를 공개하지 않는다.

## 5. Dependency 방향

```text
dxb-kernel
   ↑
dxb-domain  ← Capability Contracts
   ↑
dxb-application
   ↑
dxb-runtime composition ← dxb-provider-host
   ↑
dxb-control
   ↑
interfaces
```

Provider dependency firewall과 Application Port→Host implementation 분리는 v0.3 규칙을 그대로 유지한다.

Live Control이 Provider implementation crate를 직접 import하지 않는다. Provider native steering/cancellation은 Capability/Host의 stable feature surface를 통해 사용한다.

## 6. State/DTO 분리

다음을 동일 struct로 무분별하게 공유하지 않는다.
- Conversation Domain type vs Control Wire DTO vs DB row
- Thread lineage Domain type vs Projection
- Control Directive Domain record vs Runtime signal
- Provider Session/resume token vs ThreadId
- Task Specification revision vs Provider prompt DTO

## 7. Shared Reference / Allocation

Bot-global immutable prefix, Thread immutable revision metadata, large Conversation/Artifact payload는 `Arc`/shared buffer/content reference 등 소유권이 명확한 zero/low-copy 경로를 사용할 수 있다. 범용 RuntimeContext 또는 shared mutable map으로 최적화하지 않는다.

## 8. Provider Scaffold / 기존 Common Framework

v0.3의 Standard Provider Package, SDK/Host/Conformance/Reference/Dependency Firewall 규칙은 그대로 유효하다. 새 Conversation/Thread 기능을 구현하기 위해 Provider Contract를 확장하지 않는다.

## 9. 검증 기준

- Domain→Provider/Plugin/UI forbidden edge 0
- Conversation/Thread Domain이 Interface Session/Provider Session type에 의존하지 않음
- Runtime live-control module이 Scheduler/Task/Side Effect owner를 중복 구현하지 않음
- `cargo metadata`에서 dependency cycle 0
- new module 추가가 불필요한 crate 분할/clone/service locator를 만들지 않음

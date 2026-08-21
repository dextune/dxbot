---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-RUN-038", "DXB-ENG-050", "DXB-IFC-040", "DXB-IFC-041"]
---

# 테스트·검증 전략

## 1. 목적

v0.6 deterministic Domain/Process/Epistemic/Security/Resource suites를 유지하고, v0.7 Application Contract/Runtime Host/CLI를 실제 Common path로 검증한다.

## 2. Test Layer

Unit → Property/Model → Concurrency → Component → Contract/Conformance → Host Integration → CLI E2E → Fault/Recovery → Performance/Soak → Security → Architecture/Docs 순으로 필요한 계층을 사용한다.

Core correctness는 deterministic fake/reference Provider, fake clock/ID/RNG/budget/barrier를 사용하며 실제 모델 응답에 의존하지 않는다.

## 3. v0.7 Deterministic Fixture

추가:
- Application Contract schema fixture
- fake/local Control Endpoint/Client fixture
- fake Host Lifecycle Client/service-manager fixture
- Runtime absent → start → readiness → Control connect bootstrap fixture
- protocol/schema compatible/incompatible matrix
- CommandId/Idempotency receipt store fixture
- command commit 후 response drop failpoint
- stale revision/generation fixture
- authorization deny/approval-required fixture
- event cursor retention/gap/resync fixture
- CLI process/SIGINT local cancellation fixture
- slow stdout/broken pipe writer fixture
- large paged query/Artifact stream fixture
- Runtime Host restart/endpoint recreation fixture
- memory pressure/admission error fixture

production shortcut이 되지 않는 deterministic in-process adapter는 ADR 범위에서만 사용한다.

## 4. Application Contract Suite

### AT-APP-001
Representative Backend use-case를 public Contract만으로 수행한다.
- direct Domain Store 0
- direct Scheduler/Core mutation 0
- direct Provider call 0

### AT-APP-002
old/new compatible pair와 incompatible pair를 검증한다.
- compatible semantic 동일
- incompatible explicit error
- silent field reinterpretation 0
- data schema/protocol version 혼합 0

### AT-APP-003
mutation commit 직후 response를 drop하고 동일 key retry.
- duplicate Canonical effect 0
- 기존 receipt/outcome 조회 가능

### AT-APP-004
stream disconnect/gap/expired cursor.
- resume 또는 explicit resync
- silent event loss/current 오판 0

## 5. Runtime Host Bootstrap Suite

Runtime이 실행 중이지 않은 상태에서:

```text
dxb runtime start
→ Host Lifecycle Client
→ service/launcher start
→ endpoint readiness discovery
→ protocol negotiation
→ Runtime status Query
→ normal Domain Command
```

검증:
- Host Lifecycle Client의 Domain Store/API direct mutation 0
- Runtime Ready 이전 Domain command를 local fallback으로 처리 0
- start timeout/failure가 Bot/Task state를 생성하지 않음
- stop/kill path가 Task cancel/reconcile semantic을 대체하지 않음
- Runtime restart 후 Domain identity unchanged

## 6. CLI Suite

### AT-CLI-001
CLI만으로 Runtime/Bot/Conversation/Thread/Project/Channel/Task/Process/Memory/Control/Recovery P0 flow 완결.

### AT-CLI-002
Task/Process 실행 중 CLI exit/crash/SIGINT/disconnect 주입.
- explicit command 없으면 Runtime lifecycle 변화 0

### AT-CLI-003
`--json`/`--jsonl`:
- stdout machine data 외 혼입 0
- diagnostics stderr
- terminal width/color semantic 영향 0
- stable exit class

### AT-CLI-004
large history/memory/artifact/watch:
- full heap materialization 0
- bounded page/chunk
- slow consumer unbounded queue 0
- CLI RSS total dataset linear growth 0

### AT-CLI-005
stale Role/Authority, Memory conflict, pressure를 주입해 client-side policy inference 0.

### AT-CLI-006
Runtime restart/endpoint recreation 뒤 same Domain identity 조회.

### AT-CLI-007
v0.7 active Interface inventory에서 IFC-040/041만 존재하고 superseded Interface active requirement 0.

## 7. Fault Matrix

반드시 결합한다.
- Runtime absent/startup failure/readiness timeout
- stale revision
- permission deny/approval required
- command commit 후 response loss
- CLI crash
- SIGINT during watch
- Runtime restart
- Control Endpoint recreation
- Provider loss
- event cursor gap/expiry
- slow pipe
- broken pipe
- large history/export
- Runtime memory pressure
- protocol mismatch

각 fault에서 duplicate effect, identity drift, secret leakage, resource leak을 검사한다.

## 8. Architecture / Dependency Tests

- CLI→Domain/Runtime/Storage/Provider internal dependency 0
- Host Lifecycle Client→Domain/Task/Memory/Scheduler/Provider mutation/internal dependency 0
- public DTO = Domain/Persistence struct 공유 0
- direct Store/Scheduler/Provider control path 0
- Runtime Ready 이후 Domain use-case의 Host Lifecycle path 사용 0
- TUI/Web active crate/module/doc dependency 0
- `DXB-IFC-042/043` active `depends_on` 0

## 9. Document Relationship Review — 2 Passes

### Review 1 — Structural / Consistency
- file/path/ID/inventory/naming
- `DXB-IFC-042/043` supersession 및 active package 부재
- active dependency/release milestone/Acceptance/Risk/OQ 잔존 여부
- Canonical Owner duplication
- Contract/Traceability/Removal 영향
- dependency DAG
- v0.6 Backend inheritance

### Review 2 — Cross-Layer Executability / Compatibility

```text
Shell
→ dxb CLI
→ Host Lifecycle Client (bootstrap only) or Control Client
→ Application Contract
→ Application Command/Query
→ Domain/Runtime
→ Persistence/Scheduler/Provider Host
→ Event/Result
→ Control Protocol
→ CLI Output
```

위 fault matrix를 삽입해 duplicate semantic effect 0, lifecycle independence, bounded output, auth/resource/version semantics, Backend v0.6 regression 0을 확인한다.

발견 사항 수정 후 해당 Review를 재실행한다.

## 10. Release Gate

v0.6 suites에 추가:
- Application Contract schema/compatibility
- Command idempotency/lost-response
- Subscription gap/resume
- Runtime Host bootstrap/process independence
- CLI machine output/exit semantics
- CLI large-output/slow-consumer soak
- CLI security/redaction
- forbidden dependency architecture test
- active Interface scope inventory test

## 11. 검증 기준

- AT-APP-001~004, AT-CLI-001~007이 Risk/OQ/Metric/roadmap에 연결된다.
- Runtime bootstrap fixture가 narrow Host Lifecycle boundary를 증명한다.
- 2회 독립 Review evidence가 `manifest.md`에 존재한다.
- v0.6 Acceptance semantic regression 0.
- TUI/Web active implementation/release test requirement 0.

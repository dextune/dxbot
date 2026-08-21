---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-037", "DXB-RUN-038", "DXB-IFC-040", "DXB-IFC-041", "DXB-ENG-052"]
---

# 구현 로드맵과 단계별 Gate

## 1. 목적

v0.6 Backend Foundation과 모든 correctness/resource/security Gate를 유지한 채, v0.7은 **Application Contract → Runtime Host/Control Endpoint → CLI** 순서로 구현한다. CLI command를 먼저 만들고 Backend shortcut을 뒤에서 맞추는 방식을 금지한다.

## 2. v0.6 Foundation 유지

재설계하지 않는다.
- Common Capability Contract/Provider Host/Lifecycle/SDK/Conformance
- Persistent Bot/Main Conversation/Thread
- Project/Channel collaboration
- Scope-Aware Epistemic Memory
- Durable Process/ActionGrant/Information Flow
- immutable Execution + Live Control/Side Effect recovery
- Dynamic Core Scheduler ownership
- Runtime Memory envelope/reservation/pressure/staged recovery
- Bot-only path

v0.6 Acceptance/Risk/OQ는 v0.7이 명시적으로 supersede하지 않는 한 계속 유효하다.

## 3. v0.7 Active Interface Scope

v0.7 구현/검증/Release Gate의 product Interface는 다음뿐이다.

```text
Headless Application Contract (`DXB-IFC-040`)
→ CLI Reference Interface (`DXB-IFC-041`)
```

TUI/Web 구현 milestone, acceptance, release blocker는 v0.7 roadmap에서 제거한다. 미래 Interface를 위한 BFF/view/route/frontend state도 선행 구현하지 않는다.

## 4. M0 — Interface Scope / Baseline Cleanup

작업:
- v0.6 문서/manifest/interface inheritance inventory
- `DXB-IFC-042/043` active baseline explicit supersession
- active Interface docs를 IFC-040/041로 freeze
- stale TUI/Web milestone/Acceptance/Risk/OQ/dependency 제거
- Backend Canonical Owner regression 검토

Exit:
- active package에서 IFC-042/043 파일 0
- active `depends_on` IFC-042/043 0
- TUI/Web release gate/milestone 0
- Backend v0.6 owner/Acceptance regression 0

## 5. M1 — Headless Application Contract

구현/고정:
- Command/Query/Subscription 분리
- public DTO/error/resource/version semantics
- CommandId/Idempotency/Correlation/Causation
- pagination/windowing/Artifact reference
- subscription cursor/watermark/gap/resync/backpressure
- authorization/principal context
- capability discovery
- compatibility negotiation

Exit:
- AT-APP-001~004
- direct Store/Scheduler/Provider access 0
- incompatible pair explicit fail
- lost-response duplicate effect 0

## 6. M2 — Runtime Host / Control Endpoint / Bootstrap

구현/고정:
- CLI-independent long-lived Runtime Host
- local authenticated Control Endpoint
- Control Client
- narrow Host Lifecycle Adapter/Client for Runtime-absent bootstrap only
- OS service manager/launcher start/status/stop/readiness boundary
- Host Lifecycle Client의 Domain mutation 금지
- request/stream byte bounds
- deadline/local cancel semantics
- reconnect/version negotiation
- Runtime start/status/stop/doctor의 host-state vs Domain-state 구분
- platform daemon/service/IPC ADR

Exit:
- Runtime absent→start→readiness→Control connect fixture
- Host Lifecycle Client direct Domain/Store/Scheduler/Provider access 0
- Runtime Ready 이후 Domain use-case의 Host Lifecycle shortcut 0
- CLI disconnect/SIGINT independence
- endpoint recreation/reconnect fixture
- Runtime restart identity preservation
- server/client subscriber cleanup

## 7. M3 — CLI Core Operations

P0 use-case:
- runtime
- bot/conversation/thread
- project/channel
- task/process/control
- memory
- provider/capability status
- approval/reconcile/trace 최소 경로

추가:
- human output
- JSON machine mode
- stable error/exit class
- TTY/non-interactive mutation safety

Exit:
- AT-CLI-001/002/005/006
- Bot-only + Project/Channel representative E2E
- client-side policy duplication 0

## 8. M4 — CLI Automation / Streaming / Recovery

구현/고정:
- JSONL/stream mode
- watch/follow
- cursor resume/gap resync
- idempotent retry/reconcile
- SIGINT/broken pipe
- large output paging/streaming
- slow consumer policy
- doctor/reconcile diagnostics

Exit:
- AT-CLI-003/004
- AT-APP-003/004 재검증
- unbounded server/client buffer 0
- machine stdout contamination 0

## 9. M5 — CLI Hardening / Packaging

구현/측정:
- compatibility matrix
- security/secret redaction
- performance/RSS/soak
- Runtime Host + CLI package/install smoke
- docs/help/completion 중 채택 범위
- forbidden dependency architecture gate
- full v0.6 Backend regression
- final Review 1/2

Exit:
- AT-CLI-007
- R-078~089 critical mitigation evidence
- no TUI/Web active artifact
- v0.7 release gate PASS

## 10. Implementation Dependency

```text
M0 Scope Cleanup
→ M1 Application Contract
→ M2 Runtime Host / Control Endpoint / Bootstrap
→ M3 CLI Core
→ M4 Automation / Streaming / Recovery
→ M5 Hardening / Packaging
```

중요:
- CLI command tree를 먼저 구현하고 Backend API를 뒤에서 맞추지 않는다.
- Contract를 먼저 prototype/freeze한다.
- Runtime bootstrap 외 mutation/read는 Common Application/Control path 하나를 사용한다.
- Host Lifecycle Client를 Domain fallback API로 사용하지 않는다.
- headless fixture 전 CLI convenience API 추가 금지.
- CLI가 Runtime internal crate를 참조해야만 가능한 기능은 architecture smell로 판정한다.

## 11. Non-Goals / Future Re-entry

v0.7 active scope 아님:
- TUI
- Web Control Center
- browser/frontend state model
- BFF/view-specific endpoint
- distributed/HA를 Interface 이유로 선행 구현
- 범용 Workflow DSL
- speculative multi-transport/service-locator abstraction

새 product Interface 계획은 **M0~M5와 v0.7 DoD, Runtime Host 안정성, Application Contract compatibility, CLI E2E/resource/security evidence가 충분히 안정화된 뒤 별도 버전에서 재개**한다.

## 12. 검증 기준

- M0~M5가 AT-APP/CLI, R-078~089, OQ-068~079와 연결된다.
- Runtime bootstrap이 ADR-0083의 narrow Host Lifecycle boundary를 따른다.
- Backend v0.6 correctness/resource/security gate를 우회하지 않는다.
- active TUI/Web milestone 0.
- CLI가 `DXB-IFC-040` 없이 Backend Domain을 직접 호출하는 단계 0.

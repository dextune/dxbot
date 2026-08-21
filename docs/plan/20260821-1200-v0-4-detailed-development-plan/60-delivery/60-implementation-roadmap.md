---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-036", "DXB-ENG-052"]
---

# 구현 로드맵과 단계별 Gate

## 1. 목적

v0.3의 **Common correctness boundary → 정형화된 Extension 개발 표면 → 자동화 → 생태계** 순서를 유지하면서, v0.4에서는 제품 사용자 모델을 **Persistent Main Conversation + Thread + Live Control**로 완성한다. 기간보다 dependency/exit gate를 기준으로 한다.

## 2. 비협상 선행 순서

두 Foundation을 동시에 지킨다.

### A. Common Provider Framework Foundation
v0.3의 Complete Capability Contract, Provider Host, Standard Lifecycle, Thin Provider, Dependency Firewall, Executable Conformance, Reference Provider, Tier A/B 규칙을 유지한다.

### B. Persistent Conversation / Live Control Foundation
다음이 닫히기 전 UI 편의 기능이나 Provider-native steering에 맞춰 임시 semantic을 만들지 않는다.

1. Session 3분리: Interface Session / Provider Session / Thread
2. Bot당 Main Conversation 1:1
3. Thread Canonical Owner/lifecycle/lineage
4. Conversation History ≠ Memory
5. Thread-local Memory + Promotion
6. bounded Thread-aware Context
7. immutable Execution + Task Specification revision
8. bounded Runtime Control Channel
9. cooperative preemption/safe point
10. suspension/checkpoint/recovery

## 3. Common Framework Phase 유지

### Phase 1 — Common Contract Freeze / P0
v0.3의 Capability Contract Package, Host/Lifecycle/Registry/Selector, Provider Call Context, Error/Retry/Side Effect owner, Dependency Firewall, Tier A/B를 완료한다.

### Phase 2 — Provider Development Framework / P0-P1
Provider SDK, Capability Conformance, Host-path harness, Reference/negative Provider, Standard Package Skeleton, removal/minimal build를 완료한다.

### Phase 3 — Extension Automation / P1
`dxb-dev new-provider` Scaffold, template drift, config/error/mapping/conformance skeleton을 자동화한다.

### Phase 4 — Extension Ecosystem / P2
remote Provider, public Extension SDK, Plugin Provider certification을 검토한다.

v0.4 신규 Domain 기능 때문에 위 단계의 Host/Provider 책임을 다시 Provider 안으로 되돌리지 않는다.

## 4. Milestone 통합

| Milestone | 핵심 결과 | v0.4 추가 Gate |
|---|---|---|
| M0 Foundation & Harness | workspace/testkit/storage/Host skeleton | Session 용어/owner freeze, Reference Harness safe-point/session-loss fixture |
| M1 Persistent Single Bot | restart Bot + CLI | Main Conversation persistence, Interface Session independence, Thread create/restore, migration fixture |
| M2 Memory/Task/Harness | Memory/Task/single Core | Thread Memory, Thread-aware Context, redirect/suspend/resume basic semantic, Provider Session independence |
| M3 Dynamic Core | bounded parallel/fairness | separate Control Channel, reprioritize/rebalance, cooperative preemption, A/B/C Thread isolation |
| M4 Multi-Bot | durable delegation | delegated Task의 Thread/provenance와 control authority 분리 |
| M5 Control API & TUI | management plane | Main Conversation 기반 Thread control, live report/freshness, Directive/ack UI |
| M6 Web | control UI | one Bot Chat + Thread Graph + realtime intervention UX |
| M7 Distributed | remote/HA candidates | remote control/Provider가 local Directive/Host semantics 유지 |

## 5. M0 Exit

기존 Domain no-I/O, atomic Event, Capability Contract/Host/Lifecycle/Reference/dependency gate에 추가:
- `Main Conversation`, `Thread`, `Interface Session`, `Provider Session` glossary/ID freeze
- Conversation/Thread Domain types가 Provider/UI에 의존하지 않음
- Execution Supervisor/Control Channel test seams
- scripted Provider Session loss/cancel/yield supported/unsupported fixture
- migration strategy 초안

## 6. M1 Exit — Persistent Single Bot / Conversation

- BotId/Identity/Bot-global Memory restart restore
- Bot당 Main Conversation exactly one
- 모든 Interface Session disconnect/reconnect 후 동일 Conversation/Thread list
- Thread create/archive/restore/branch 기본 lineage
- Conversation history durable + paginated
- Session connect/disconnect가 Bot/Thread lifecycle을 변경하지 않음
- Routine persistence/restart 기존 gate 유지
- AT-CONV-001, AT-SESSION-002 기반 skeleton

## 7. M2 Exit — Thread Memory / Context / Task Control

- Thread-local Memory scope
- Working→Thread→Bot Promotion policy
- Conversation History≠Memory enforcement
- bounded Thread-aware Context Plan
- Task primary Thread linkage + Task Specification revision
- redirect: old Execution mutation 금지 + new revision
- suspend/resume durable checkpoint 의미
- Harness cancellation/yield/steering feature negotiation
- Provider Session loss 후 Canonical Context로 new call
- Waiting/Side Effect/Provider replacement/removal 기존 gate 유지

## 8. M3 Exit — Dynamic Core + Live Control

- fixed core count 없음
- deterministic scheduling/fairness
- Work Queue / Control Channel 분리, 모두 bounded
- control safety reserve/fairness
- inspect/report starvation 방지
- redirect/suspend safe-point cooperative preemption
- reprioritize/parallelism hint → Scheduler rebalance, direct Lease mutation 0
- Thread A/B/C simultaneous execution/control isolation
- redirect/complete, suspend/side-effect, duplicate resume deterministic race suite

## 9. M5 Exit — Control API / TUI

- Main Conversation Query/Message
- Thread create/list/get/archive/restore/branch
- Thread history/local Memory/promotion view
- report/redirect/suspend/resume/reprioritize/fork
- Directive accepted/ack/superseded/awaiting-safe-point
- report freshness/stale
- recovery doctor/reconcile
- CLI/TUI shared schema

## 10. M6 Exit — Web

- one Bot = one persistent chat surface
- Thread navigator/lineage/workspace
- Task/Execution/Core live status
- Supervisor intervention
- Session/tab와 Thread identity 분리
- virtualized history/graph and bounded client state

## 11. Quality Tier

### Tier A — Frontier-quality Common
Conversation/Thread Domain, Memory scope/promotion, Context budget, Task revision/control semantics, Runtime Control Channel, preemption/recovery, Scheduler ownership, Security, Persistence/Migration, API compatibility, Acceptance/Testkit은 Tier A다.

필수 검증: ADR 영향, property/model/concurrency/fault, storage migration, security/resource, API stability, recovery/rollback, benchmark.

### Tier B — Extension
기존 Stable Capability Provider/Adapter는 v0.3 Tier B 규칙을 유지한다. Provider가 Session/steering 기능을 구현하면서 Thread/Control semantic 변경을 요구하면 Tier A로 escalation한다.

## 12. Cross-Cutting Workstream

- docs/ADR/traceability/risk
- v0.3→v0.4 migration fixtures
- Thread/history growth + RSS/allocation/cache
- control latency/starvation/fairness
- fault injection for Directive/suspension
- Provider Session loss
- security threat model for Supervisor control
- observability correlation
- naming/layout/dependency gate
- Provider Framework regression

## 13. Scope Control

- Thread merge exact semantics는 ADR 전 P0 구현에 억지로 포함하지 않는다.
- native Provider steering을 위해 immutable Execution을 포기하지 않는다.
- hard-real-time preemption을 약속하지 않는다.
- Web/TUI 편의를 위해 Session을 Thread로 재도입하지 않는다.
- “Core 추가” UX를 fixed Core allocation API로 구현하지 않는다.
- raw Conversation History를 전체 model context에 넣지 않는다.

## 14. 검증 기준

- 각 Milestone에 새 AT-CONV/THREAD/CTX/MEM/CTRL/SESSION Gate가 연결됨.
- v0.3 AT-SPI/AT-MOD/Waiting/Side Effect/Plugin/Repository Gate가 회귀하지 않음.
- Tier B가 Conversation/Thread/Live Control semantic을 임의 결정하지 못함.
- M3 이전에 production UI workaround로 direct worker control을 만들지 않음.

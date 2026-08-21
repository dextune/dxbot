---
title: "구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-ARC-016", "DXB-ENG-052"]
---

# 구현 로드맵과 단계별 Gate

## 1. 목적

기능을 화면 중심이 아니라 제품 의미·복구·모듈 제거성의 위험 순서로 구현한다. 기간보다 exit gate와 의존성을 기준으로 한다.

## 2. v0.2 Architecture Integrity Gate — 구현 Freeze 전 P0

다음이 문서·Acceptance 수준에서 닫히기 전 Core architecture를 freeze하지 않는다.

1. 5개 Architecture Classification
2. Repository Naming/Layout Normative Rule
3. Feature Removal Contract
4. Harness Provider 독립/복수 선택
5. Routine first-class contract
6. Waiting Continuation persistence/recovery
7. Side Effect write-ahead/reconciliation
8. Cross-Layer Scenario Review

Exit:
- AT-MOD-001/002/003 설계 가능
- AT-ROUTINE-001
- AT-TASK-003
- AT-SFX-001
- AT-REPO-001
- Structural/Cross-Layer 문서 재검수 통과

## 3. Extensibility Gate — P1

- Plugin 독립 설계/manifest/lifecycle/data ownership
- Long-term Memory growth/archival metrics
- Policy/Limit SSOT
- Provider selection/config/version lifecycle
- Plugin disable/uninstall semantics

Exit:
- reference Plugin lifecycle test
- Provider/Plugin stale config migration
- canonical vs cache memory metrics 분리

## 4. Operational Extensibility — P2

- Scheduler starvation/oldest waiting advanced telemetry
- dynamic Plugin registry/isolated host
- remote Provider/CoreExecutor
- public Plugin SDK
- signed Plugin/package verification
- third-party ecosystem 후보

단일 노드 의미와 P0/P1 Acceptance를 유지한다.

## 5. 기존 Milestone 연계

| Milestone | 핵심 결과 | v0.2 추가 Gate |
|---|---|---|
| M0 Foundation & Harness | workspace/testkit/storage/Harness spike | classification, naming, provider-free build |
| M1 Persistent Single Bot | restart Bot + CLI | Routine definition persistence, config SSOT base |
| M2 Memory/Task/Harness | Memory/Task/single Core | Waiting Continuation, Side Effect Ledger, provider replacement |
| M3 Dynamic Core | bounded parallel/fairness | deterministic tie-break, multi-provider, starvation metrics |
| M4 Multi-Bot | durable delegation | Waiting delegated child recovery |
| M5 Control API & TUI | management plane | Routine/Provider/Plugin lifecycle APIs |
| M6 Web | control UI | same shared API; no plugin/domain logic duplication |
| M7 Distributed | remote/HA candidates | remote provider/plugin contracts without semantic drift |

## 6. M0 Exit 핵심

- Domain no-I/O test
- Event atomic append/replay
- Fake Harness conformance
- DeepSeek sidecar는 optional candidate로 feasibility 확인
- DeepSeek 없이 minimal build
- naming/dependency/docs CI
- Feature Classification/Removal ADR baseline

## 7. M1 Exit 핵심

- BotId/Identity/Memory namespace restart restore
- Session independence
- Routine create/enable/disable persistence
- deactivate가 Routine 정의와 Waiting state를 삭제하지 않음
- clean quiescence

## 8. M2 Exit 핵심

- Working Memory 자동 승격 금지
- Task retry가 새 Execution
- Waiting + Continuation atomic commit
- crash 후 pending child만 restore
- Side Effect Unknown/reconcile
- Fake/DeepSeek 또는 대체 Provider 동일 conformance
- Provider A 제거 후 core acceptance

## 9. M3 Exit 핵심

- core count가 Bot schema에 없음
- deterministic priority/tie-break
- global/per-Bot/provider limits
- Provider A/B/C multi-selection
- cancellation/expiry/late-result race
- queue latency/oldest waiting/starvation metric

## 10. M4~M7

기존 durable Bot Network, Control API/TUI, Web, distributed hardening의 의미를 유지한다. 각 단계에서 Plugin/Provider가 Core Domain을 우회하지 않으며 Cross-Layer Scenario Suite를 반복한다.

## 11. Cross-Cutting Workstream

- docs/ADR/traceability/risk
- security threat model
- RSS/allocation/cache
- migration fixture
- fault injection
- provider/plugin removal cleanliness
- observability
- temporary shim cleanup
- naming/layout

## 12. Scope Control

- DeepSeek가 불안정해도 Fake/Native 또는 다른 Provider로 Core 개발을 지속한다.
- Web 요구가 앞당겨져도 Domain write logic을 UI에 넣지 않는다.
- Plugin SDK는 internal trait가 안정되었다는 이유만으로 조기 공개하지 않는다.
- 분산화는 단일 노드 lease/event/idempotency/recovery가 검증된 후 진행한다.

## 13. 검증 기준

- 각 milestone이 정상 vertical slice와 failure/removal path를 가진다.
- 다음 milestone 진입 전에 선행 gate가 CI에서 green이다.
- Provider/Plugin 제거성이 구현 후반의 별도 리팩터링 항목으로 미뤄지지 않는다.

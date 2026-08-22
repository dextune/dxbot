---
title: "DXBOT v0.8.5 최상위 구현 기준선"
document_id: "DXB-BASE-000"
version: "0.8.5"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: []
---
# DXBOT v0.8.5 최상위 구현 기준선

## 1. 효력과 버전

본 문서는 active package `v0.8.5`의 최상위 구현 기준선이다. 과거 v0.1~v0.8.0 package와 원문 보존본은 변경 근거이며 현재 구현 의미를 직접 결정하지 않는다. 현재 의미는 이 package 안의 Canonical Owner만으로 판정한다.

버전 축은 분리한다.

```text
plan_version      = active package 계약 버전
review_revision   = 동일 계획선의 적대적 검수·교정 차수
document_version  = 개별 문서 의미의 버전
runtime_version   = 실행 바이너리 버전
```

`plan_version=0.8.5`, `review_revision=5`가 현재 기준이다. 개별 문서가 변경되지 않아 `0.8.0`을 유지하더라도 active package 판정은 `DXB-INDEX`의 plan version으로 수행한다.

> **Persistent Bot Runtime → typed Headless Application Contract → CLI Reference Interface**

## 2. 채택된 제품 불변조건

- `INV-001`: Bot은 Interface Session과 독립된 Persistent Identity·Memory·State를 가진다.
- `INV-002`: 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
- `INV-003`: Core는 Scheduler-owned Dynamic Lease이며 Bot 복제본이나 독립 Agent가 아니다.
- `INV-004`: Bot당 하나의 Persistent Main Conversation을 유지한다.
- `INV-005`: Conversation, Thread, Task, Execution, Core Lease, Provider Session, Interface Session은 서로 다른 identity다.
- `INV-006`: Project와 Channel은 협업·권한·지식·자원 Scope이며 Brain을 소유하지 않는다.
- `INV-007`: Conversation History와 Memory를 구분하고 모든 Memory assertion은 scope·revision·provenance를 가진다.
- `INV-008`: running Execution의 Context Plan, Task revision, Provider binding은 immutable snapshot이다.
- `INV-009`: Durable Process는 cross-aggregate 진행만 소유하며 child canonical state를 복제하지 않는다.
- `INV-010`: Authorization Decision Owner와 Runtime Resource Owner는 각각 하나다.
- `INV-011`: Provider, Plugin, Interface는 Domain Identity와 Canonical State를 소유하지 않는다.
- `INV-012`: Bot-only path는 Project·Channel·Durable Process 없이 완전하게 동작한다.
- `INV-013`: 최소 하나의 실제 Harness Adapter 경로가 Provider Host를 통해 Task를 수행해야 한다.
- `INV-014`: Multi-Bot은 typed delegation과 membership boundary로 동작하며 anonymous sub-agent로 축약하지 않는다.

## 3. Active P0 Scope

```text
Linux user-scoped Runtime Instance
→ Domain/Application/Persistence
→ Provider Host + deterministic Reference Provider + one real Harness Adapter
→ typed Command / Query / Subscription
→ authenticated Unix-domain Control Endpoint / Client
→ dxb CLI
```

TUI, Web, BFF, remote multi-tenant transport, distributed HA, generic Workflow/RPC/IDL framework, Plugin ecosystem 전체 CLI는 비범위다.

## 4. 비협상 구현 계약

1. public mutation은 typed operation만 사용하고 generic JSON dispatch와 내부 shortcut을 금지한다.
2. Canonical ID와 expected revision/generation으로 target을 고정하며 ambiguity를 거부한다.
3. mutation은 durable Operation Receipt와 crash-safe client submission protocol을 사용한다.
4. Runtime은 data root당 active HostGeneration 하나만 허용한다.
5. Query는 bounded snapshot/cursor, Subscription은 explicit terminal/gap/resync를 사용한다.
6. stdout=result, stderr=diagnostic, exit class는 stable machine contract다.
7. OS peer에서 principal을 server-side로 파생하며 profile·client payload는 Authority가 아니다.
8. 최초 Instance는 안전한 default policy generations를 원자적으로 생성한다.
9. Storage profile은 atomic commit, versioned snapshot, receipt recovery, bounded disk pressure를 executable spike로 증명한다.
10. Linux export publication은 descriptor-relative no-follow와 atomic no-replace를 사용한다.
11. `runtime stop`은 Control graceful path와 explicit host-level escalation을 구분한다.
12. active plan validator와 negative fixtures가 PR/main에서 실제 실행되어야 한다.

## 5. Canonical Owner

| 의미 | Owner |
|---|---|
| active package·version·원문 지위 | `DXB-GOV-001` |
| 제품 불변조건 | `DXB-BASE-000` |
| Runtime/Bootstrap | `DXB-RUN-035` |
| Storage implementation profile | `DXB-ARC-018` |
| Harness boundary/canary | `DXB-ARC-013` |
| public receipt/submission/selector/cursor | `DXB-IFC-040` |
| CLI machine/command surface | `DXB-IFC-041` |
| command별 typed input grammar | `DXB-IFC-042` |
| endpoint/principal/export security | `DXB-RUN-032` |
| tests/actual CI | `DXB-ENG-052`, `DXB-ENG-054` |
| roadmap/acceptance/risk/decisions | `DXB-DEL-060`~`063` |

## 6. 구현 진입 판정

Workspace bootstrap은 가능하다. public Command DTO와 CLI parser freeze는 `M1A Storage Proof`, crash-safe submission, bootstrap principal/default policy, complete input registry가 executable evidence를 얻은 뒤에만 허용한다.

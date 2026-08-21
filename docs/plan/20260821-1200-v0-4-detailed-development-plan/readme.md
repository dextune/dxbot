---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.4.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# DXBOT 상세 개발 기획 문서 집합 v0.4

이 디렉터리는 `20260821-1120-v0-3-detailed-development-plan`의 **Common Framework + Stable SPI + Provider Host + Executable Conformance**를 그대로 유지하면서, DXBOT의 사용자·Domain·Runtime 모델을 **Persistent Main Conversation + Thread + Live Control**로 고도화한 신규 버전이다.

> DXBOT은 Session에 묶인 Agent가 아니라, 하나의 Persistent Bot 아래 하나의 Main Conversation과 여러 영속 Thread가 병렬로 존재하고, 실행을 관찰·재지시·중지·재개할 수 있는 Memory-Centric Concurrent Bot Runtime이다.

## 1. 기준 우선순위

1. `00-governance/00-source-concept-original.md` 제품 철학
2. `00-governance/00-normative-baseline.md` 비협상 기준
3. 승인 ADR 및 `03-architecture-decision-baseline.md`
4. Domain 문서 Canonical Owner / invariants
5. Architecture/Runtime/Engineering 계약
6. 구현 코드와 자동화 테스트

## 2. v0.4 핵심 강화점

### Persistent Main Conversation
Bot당 하나의 논리적 Main Conversation을 두며 Web/CLI/TUI Session이 바뀌어도 같은 Conversation에 접속한다. Main Conversation은 `Thread 0`가 아니라 Bot Communication + Supervisor Control Surface다.

### Thread as Canonical Context Boundary
작업 문맥의 영속 독립 단위를 Session이 아니라 Thread로 정의한다. `Thread ≠ Task ≠ Execution ≠ Core Lease ≠ Provider Session`을 고정한다.

### Session 3분리
- Interface Session: client connection, Ephemeral
- Provider Session: provider-private optimization, Derived/Ephemeral
- Thread: DXBOT-owned persistent context boundary, Canonical

### Conversation History ≠ Memory
대화 기록과 기억을 분리한다. transcript 저장은 Memory commit이 아니며 Thread-local 정보는 Promotion 정책 없이 Bot-global Memory로 들어가지 않는다.

### Thread-aware Bounded Context
전체 transcript 크기와 model Context 크기를 분리한다. Context Plan은 current Thread, active Task, Thread-local Memory, relevant Bot-global Memory, recent/retrieved history, Artifact를 deterministic budget으로 조립한다.

### Live Control / Cooperative Preemption
Normal Work Queue와 bounded Runtime Control Channel을 분리한다. `inspect/report/redirect/suspend/resume/cancel/reprioritize/fork`를 별도 semantic으로 정의한다.

### Immutable Redirect
running Execution을 mutation하지 않는다. `Directive + Task Specification revision → safe yield → new Execution`로 전환하며 old Execution history/Provider binding/Side Effect를 보존한다.

### Suspend / Resume Recovery
Suspension은 dependency Waiting과 다르며 durable checkpoint/Continuation을 갖는다. crash/restart 후 idempotent resume로 새 Execution을 한 번만 만든다.

### Scheduler Ownership 유지
“Core 추가” intent는 priority/parallelism/resource/rebalance hint로 변환하며 실제 Core Lease authority는 Scheduler에 남긴다.

### Provider Session Independence
Provider Session이 사라져도 Bot/Main Conversation/Thread/Memory/Task가 손상되지 않으며 Canonical Context Plan으로 새 Provider call/session을 만들 수 있다.

## 3. 신규 Canonical 문서

### `20-domains/27-conversation-thread-model.md`
소유:
- Main Conversation
- Thread identity/lifecycle/lineage
- Session 3분리
- Conversation History와 Memory 경계
- Thread↔Task 관계
- Thread-local Memory scope 연결
- branch/archive/recovery/isolation

### `30-runtime/36-live-control-preemption.md`
소유:
- Runtime Control Channel
- Control Directive
- report/redirect/suspend/resume/reprioritize/fork semantics
- cooperative preemption / safe point
- control priority/starvation
- control race/recovery/audit

## 4. 핵심 Canonical Owner

| 의미 | Owner |
|---|---|
| Bot Identity/Lifecycle | `DXB-DOM-020` |
| Brain/Thread-aware Context | `DXB-DOM-021` |
| Bot/Thread/Working Memory scope | `DXB-DOM-022` |
| Task/Task Spec/Execution/Continuation | `DXB-DOM-023` |
| Core Lease/Scheduler | `DXB-DOM-024` |
| Control Plane user surface | `DXB-DOM-026` |
| Main Conversation/Thread | `DXB-DOM-027` |
| Provider Framework/Host/SPI | `DXB-ARC-017` |
| Live Control/Preemption | `DXB-RUN-036` |

## 5. 권장 읽기 순서

1. `00-governance`
2. `10-architecture/10~17`
3. `20-domains/20~27`
4. `30-runtime/30~36`
5. `40-interfaces`
6. `50-engineering`
7. `60-delivery`
8. `manifest.md`

## 6. 핵심 실행 흐름

```text
Main Conversation / Thread Message
→ Thread-aware Context
→ Task Specification
→ Scheduler / Core Lease
→ Provider Host / Provider
→ Execution Result / Memory Proposal
```

Live control:

```text
Main Conversation / Control API
→ Authorization + expected revision
→ Durable Control Directive
→ bounded Runtime Control Channel
→ Execution Supervisor
→ safe point / yield
→ Scheduler admission
→ new Execution or Suspended state
```

## 7. 신규 Acceptance

- AT-CONV-001 Persistent Main Conversation
- AT-THREAD-001 Concurrent Thread Isolation
- AT-CTX-002 Bounded Thread Context
- AT-MEM-004 Thread Memory Promotion
- AT-CTRL-002 Out-of-Band Report/Control Starvation
- AT-CTRL-003 Live Redirect
- AT-CTRL-004 Suspend/Resume Recovery
- AT-CTRL-005 Parallel Intervention Isolation
- AT-SESSION-002 Provider Session Loss Independence

v0.3의 AT-SPI-001~010과 기존 Bot/Brain/Core/Memory/Task/Side Effect/Routine/Provider/Plugin/Repository Acceptance는 유지한다.

## 8. 문서 구성

- Governance: 6
- Architecture: 8
- Domains: 8
- Runtime: 7
- Interfaces: 4
- Engineering: 5
- Delivery: 4
- Normative/plan documents: **42**
- Index + Manifest 포함 총 Markdown: **44**

## 9. 구현 순서

M1에서 Main Conversation persistence/Thread restore를 닫고, M2에서 Thread Memory/Context/redirect/suspend/Provider Session independence를 닫는다. M3에서 bounded Control Channel/cooperative preemption/reprioritize/A-B-C isolation을 닫은 후 M5/M6에서 Main Conversation 중심 TUI/Web control UX를 제공한다.

Provider Framework Phase와 Extension Tier A/B 규칙은 v0.3에서 그대로 유지한다.

## 10. 완료 정의

1. Session이 Bot/Thread identity의 owner가 아니다.
2. Thread A/B/C가 병렬 실행되어도 local Memory/Context/control이 격리된다.
3. Thread history 성장과 model Context 크기가 분리된다.
4. Thread-local Memory는 Promotion 없이 global에 들어가지 않는다.
5. Work Queue 포화에서도 control path가 starvation되지 않는다.
6. redirect/suspend가 immutable Execution/Side Effect invariant를 깨지 않는다.
7. Provider Session loss 후 같은 Bot/Conversation/Thread가 복구된다.
8. Core Lease ownership이 Scheduler에 유지된다.
9. 기존 Common Provider Framework/Conformance/Removal gate가 회귀하지 않는다.
10. Structural / Contract-Traceability / Cross-Layer Executability 3회 검수를 통과한다.

## 11. 관계성 검토 상태

- Review 1 — Structural / Canonical Ownership: **PASS after correction**
- Review 2 — Contract / Traceability: **PASS after correction**
- Review 3 — Cross-Layer Executability: **PASS**

발견 사항과 수정·재검증 내용은 `manifest.md`가 검증 이력의 Canonical 문서다.

---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.5.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# DXBOT 상세 개발 기획 문서 집합 v0.5

이 디렉터리는 `docs/plan/20260821-1200-v0-4-detailed-development-plan`을 strict superset으로 고도화한 Tier A 문서 패키지다. v0.4의 **Persistent Bot + Main Conversation + Thread + Task + Execution + Dynamic Core Lease + Live Control + Provider Independence**를 유지하면서, **Project + Channel + Scope-Aware Memory + Persistent Multi-Bot Collaboration**을 추가한다.

> Identity는 Bot에, 판단은 Bot Brain에, 공유 지식은 해당 Scope에, Conversation/Thread는 Conversation Domain에, Task state는 Task에, Execution snapshot은 Execution에, Core Lease는 Scheduler에 속한다. Project와 Channel은 협업 상태를 소유하지만 지능을 소유하지 않는다.

## 0. 버전 상속 규칙

v0.5는 v0.4 상세 개발기획 전체를 normative baseline으로 참조 상속한다. 동일 `document_id`의 v0.5 문서는 명시적으로 바꾼 의미만 supersede하며, v0.4의 나머지 상세 계약은 계속 유효하다. 따라서 v0.5 문서에서 기존 세부를 반복하지 않았다는 사실은 제거·완화·deprecation을 뜻하지 않는다. Canonical 규칙은 `00-governance/00-normative-baseline.md`, 검수 증거는 `manifest.md`가 소유한다.

## 1. 비협상 호환성

v0.5는 v0.4의 Bot-only 사용 경로를 그대로 보존한다.

```text
User → Persistent Bot → Main Conversation → Thread → Task → Execution → Core Lease
```

Project/Channel을 사용하는 추가 경로는 병렬로 존재한다.

```text
Project → Channel → Channel Conversation → Thread
                    ↓
            Persistent Bot participants
                    ↓
               Bot Brain
                    ↓
          Dynamic Scheduler/Core Lease
```

Project/Channel을 사용하지 않는 기존 Bot에 Project를 강제로 생성하지 않는다. `Interface Session ≠ Provider Session ≠ Thread`, `Conversation History ≠ Memory`, immutable Execution, Scheduler-owned Core Lease, Provider Host/Stable SPI, Side Effect write-ahead, bounded queue/cache 원칙은 모두 유지한다.

## 2. v0.5 핵심 축

### Persistent Collaboration
Project는 장기 Collaboration/Knowledge/Resource boundary다. Channel은 Project 안의 영속 Multi-Bot Collaboration Space다. 둘 다 Brain이 아니며, Channel membership은 Bot을 Canonical participant로 사용한다.

### Scope-Aware Knowledge
Canonical Memory Scope를 Bot/Project/Channel/Thread로 일반화한다. Execution Working Context는 Runtime-only다. Scope 간 승격은 in-place move가 아니라 source revision/provenance를 보존하는 proposal/promotion으로 처리한다.

### Dynamic Execution
Channel event도 대상 Bot의 Brain을 거쳐 Scheduler가 Core Lease를 발급한다. Channel별 Core 고정 할당, Manager Core 고정, Coordinator 내부 LLM reasoning을 금지한다.

## 3. 신규 Canonical 문서

- `20-domains/28-project-scope-model.md` — Project identity/lifecycle/membership/resource boundary
- `20-domains/29-channel-collaboration-model.md` — Channel identity/membership/role/authority/conversation relation
- `30-runtime/37-channel-orchestration.md` — participant routing, bounded fan-out, manager-first dispatch, membership fencing, presence projection

## 4. Canonical Owner

| 의미 | Owner |
|---|---|
| Bot Identity/Lifecycle | `DXB-DOM-020` |
| Brain/Scope-aware Context | `DXB-DOM-021` |
| Memory Scope/Recall/Promotion | `DXB-DOM-022` |
| Task/Execution/SupervisorRef | `DXB-DOM-023` |
| Core Lease/Scheduler | `DXB-DOM-024` |
| Durable Bot Network | `DXB-DOM-025` |
| Control Plane semantics | `DXB-DOM-026` |
| Conversation/Thread/ParentRef | `DXB-DOM-027` |
| Project | `DXB-DOM-028` |
| Channel | `DXB-DOM-029` |
| Live Control/Preemption | `DXB-RUN-036` |
| Channel Runtime Orchestration | `DXB-RUN-037` |
| Provider Framework/Host/SPI | `DXB-ARC-017` |

## 5. 문서 구성

- Governance: 6
- Architecture: 8
- Domains: 10
- Runtime: 8
- Interfaces: 4
- Engineering: 5
- Delivery: 5
- Normative/plan documents: **46**
- `readme.md` + `manifest.md`: 2
- Total Markdown: **48**

## 6. 핵심 실행 흐름

```text
User Message
→ Project/Channel authorization
→ Channel Coordinator routing
→ Target Persistent Bot
→ Bot Brain / Scope-aware Context Plan
→ Task / durable delegation
→ Scheduler / Dynamic Core Lease
→ Provider Host / Provider
→ Result / Artifact / MemoryProposal
→ Scope Classification + Canonical Authorization
→ Scoped Memory Commit / explicit Promotion
```

## 7. 신규 Acceptance

- AT-PROJECT-001 Project Persistence / Isolation
- AT-CHANNEL-001 Persistent Channel Membership
- AT-CHANNEL-002 Bounded Participation Routing
- AT-CHANNEL-003 Role / Authority Separation
- AT-MEM-005 Scope Isolation
- AT-MEM-006 Promotion Provenance
- AT-CTX-003 Scope-Aware Bounded Context
- AT-SEC-002 Membership Revocation
- AT-COLLAB-001 Channel Manager Collaboration
- AT-MIG-001 v0.4 Compatibility

기존 v0.4 및 v0.3 Acceptance는 baseline 상속 규칙에 따라 삭제·축소되지 않는다.

## 8. 구현 순서

M0 Contract/Scope Foundation → M1 Persistent Project/Channel → M2 Scope-Aware Memory/Context → M3 Channel Runtime Orchestration → M4 Multi-Bot Collaboration → M5 API/CLI/TUI → M6 Web → M7 Distributed/HA 순서로 Gate를 닫는다.

## 9. 완료 정의

1. Project/Channel Canonical Owner가 각각 하나다.
2. Memory scope semantic은 `DXB-DOM-022`가 단독 소유한다.
3. Bot-only v0.4 경로가 회귀하지 않는다.
4. Shared Memory는 Bot Memory 복제본이 아니다.
5. Role 문자열과 Runtime Authority가 분리된다.
6. 모든 Shared Scope read/write가 current canonical authorization을 통과한다.
7. Channel fan-out/queue/cache/history retrieval은 bounded다.
8. Coordinator가 Brain 역할을 하지 않는다.
9. Provider Session/Core/Interface가 Project/Channel/Thread identity를 소유하지 않는다.
10. v0.4→v0.5 migration이 identity/revision/provenance를 보존한다.
11. Structural / Compatibility-Traceability / Cross-Layer Executability 3회 독립 검수를 통과한다.

검수 이력과 inventory 증거는 `manifest.md`가 소유한다.

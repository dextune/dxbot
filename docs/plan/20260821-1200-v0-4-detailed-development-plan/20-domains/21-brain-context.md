---
title: "Brain과 Thread-aware Context 조립"
document_id: "DXB-DOM-021"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-020", "DXB-DOM-022", "DXB-DOM-027", "DXB-ARC-013", "DXB-ARC-017"]
---

# Brain과 Thread-aware Context 조립

## 1. 목적

Brain을 특정 LLM/Provider가 아닌 Bot의 판단·조정 중심으로 정의하고, 여러 Thread/Core가 병렬 실행되어도 Bot-global 지식은 일관되게 사용하면서 Thread-local/Execution Working Context는 격리한다.

## 2. 책임 범위

- Brain logical responsibility / BrainSnapshot
- current Thread-aware Context Plan
- Capability requirement/selection hint
- Context budget/cache/reproducibility
- 공유/격리 경계
- historical conversation retrieval와 Memory 분리

LLM 내부 추론은 저장·모델링 대상이 아니다. Conversation/Thread는 `DXB-DOM-027`, Provider invocation은 `DXB-ARC-017`이 소유한다.

## 3. Brain 구성

Brain은 Identity revision, Goal/Task view, Memory query policy, Thread/context assembly policy, Model/Tool routing policy, Core spawn/delegation policy, Permission/Resource references, communication/control policy, policy bundle version의 조합이다.

Brain은 mutable singleton process가 아니다. Bot Coordinator가 command/Execution 준비 시 immutable `BrainSnapshot`을 만든다.

## 4. 공유/격리 Matrix

| Bot-global 공유 | Thread scope | Execution/Core 격리 |
|---|---|---|
| BotId/Identity revision | ThreadId/revision/lineage | ExecutionId/CoreLeaseId |
| committed Bot-global Memory | Thread-local Memory | selected Memory snapshot |
| Goals/global Task projection | Thread-linked Task projection | current Task slice |
| Permission/Resource policy ref | Thread sensitivity/retention ref | scoped grant/deadline |
| Capability catalog snapshot | recent/retrieved Thread history | Provider binding/call stream |
| communication policy | Thread artifacts/summary refs | scratch/result fragments |

Core는 Memory/Thread/Registry mutable 객체를 직접 수정하지 않는다.

## 5. Thread-aware Context Plan

Context Plan은 model-visible input 전의 구조화 선택 결과다.

최소 후보:
1. current user input
2. Bot Identity/Persona + safety/policy
3. current Thread identity/revision/intent/lineage
4. active Goal/Task specification revision
5. active Execution/checkpoint state
6. Thread-local Memory IDs/revisions
7. relevant Bot-global Memory IDs/revisions
8. recent Thread Conversation Message refs
9. retrieved historical Thread context refs
10. Artifact/checkpoint/workspace refs
11. required Capability metadata
12. token+byte budget allocation
13. trust/sensitivity labels
14. deterministic ordering/truncation
15. content/source digest

Prompt 문자열만 원본으로 보존하지 않고 source reference와 selection reason을 기록한다.

## 6. 핵심 불변조건

> **Thread의 전체 transcript 길이와 LLM Context 크기는 독립적이다.**

Thread가 장기간 누적되어도 전체 history를 매 Provider call에 전달하지 않는다. `recent + retrieved + Memory + summary/artifact`를 deterministic budget으로 선택한다.

Provider Session이 자체 hidden conversation history를 유지하더라도 DXBOT Context Plan과 상충하는 hidden state를 Canonical input으로 간주하지 않는다. Provider Session 재사용은 optimization이며 필요하면 stateless/new-session call로 재현 가능해야 한다.

## 7. 조립 순서

```text
Current Input
+ Identity / Policy
+ Current Thread
+ Active Task / Execution State
+ Thread-local Memory
+ Relevant Bot-global Memory
+ Recent Thread Events/Messages
+ Retrieved Historical Context
+ Artifacts / Checkpoints
→ Context Plan
→ Budget / Stable Ordering / Trust Filter
→ Provider-neutral Render Plan
→ Digest + Trace
→ Provider Host
```

우선순위:
1. 안전/권한/현재 명시 user directive
2. Bot Identity + current Thread/Task success condition
3. current control/task revision
4. 필요한 local/global Memory
5. 최근/검색된 conversation context
6. Tool/Skill schema
7. 오래된 상세 trace

## 8. Conversation History와 Memory

Conversation Message는 Context 후보이며 Memory와 동일하지 않다. historical transcript는 relevance/freshness/trust 정책을 통해 선택하고, 장기적으로 반복 사용해야 하는 사실은 Memory Proposal/Promotion 경로를 거친다.

Thread-local Memory와 Bot-global Memory가 충돌하면 scope/provenance/confidence/recency/conflict marker를 유지하며 Thread local을 무조건 global보다 우선하거나 반대로 하지 않는다. policy가 결정 근거를 기록한다.

## 9. Capability Routing

Brain은 concrete Provider를 선택하지 않고 Capability requirement + selection hint를 만든다. 실제 provider/version/config/generation 선택은 Provider Host/Selector가 수행한다. Execution의 동일 Capability binding은 immutable하다.

redirect는 existing Execution Context Plan을 mutation하지 않는다. Task Specification/Directive revision이 바뀌면 새 Execution에서 새 Context Plan을 생성한다.

## 10. Cache / 메모리 효율

- Identity/Policy/Tool schema stable prefix는 revision digest cache
- Thread summary/index는 Derived, bounded, rebuildable
- 같은 immutable Bot-global prefix를 Core별 복사하지 않고 shared ref
- dynamic Thread/Task/Memory와 stable prefix 분리
- Context cache key에 필요한 Bot/Thread/Task/Identity/Policy/Capability/Provider generation 포함
- token+byte cap
- 전체 Thread transcript/Memory heap materialization 금지
- hit rate/retained bytes/invalidation cost 측정

## 11. 예외

- Thread history retrieval timeout: recent context + required Memory로 degraded 또는 policy failure
- budget 초과: deterministic priority 축약
- archived Thread: read/context 가능 여부와 신규 Task 연결 정책 분리
- Provider Session lost: Canonical Context Plan으로 새 session/call 생성
- redirect pending: 새 Task revision을 old Execution prompt에 삽입하지 않고 safe yield 후 새 Context Plan
- prompt injection content: untrusted data label + sensitive action reauthorization

## 12. 검증 기준

- AT-BRAIN-001: 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
- AT-CTX-001: Core temporary context가 서로 누출되지 않는다.
- AT-CTX-002: Thread history가 증가해도 Context Plan이 정책 token/byte budget을 초과하지 않는다.
- AT-THREAD-001: Thread A/B/C의 local context가 병렬 실행에서 격리된다.
- 동일 snapshot/source가 동일 Context Plan digest를 만든다.
- Provider Session 제거가 Context 재구성을 막지 않는다.
- redirect가 old Execution Context Plan을 mutation하지 않는다.

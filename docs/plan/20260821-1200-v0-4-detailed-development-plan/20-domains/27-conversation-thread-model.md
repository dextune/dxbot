---
title: "Persistent Conversation과 Thread 모델"
document_id: "DXB-DOM-027"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# Persistent Conversation과 Thread 모델

## 1. 목적

DXBOT의 사용자 대화 모델을 Session 중심이 아니라 **Bot당 하나의 Persistent Main Conversation + 영속 Thread Graph**로 고정한다. Interface 연결이나 Provider conversation/session handle이 사라져도 Bot의 Conversation/Thread/Memory/Task 의미가 유지되어야 한다.

## 2. Canonical 정의

### Main Conversation

Bot당 하나 존재하는 논리적 영속 Communication + Supervisor Control Surface다. 사용자는 여러 Web/CLI/TUI Interface Session에서 같은 Bot의 같은 Main Conversation에 접속할 수 있다.

Main Conversation은 `Thread 0`, Provider Session, UI room object가 아니다. Bot 자체와 장기 상호작용하고 Thread를 탐색·생성·전환·제어하는 상위 대화 표면이다.

### Thread

DXBOT이 소유하는 영속 작업·문맥 경계다. Thread는 대화/작업의 장기 lineage와 local context scope를 보존하며 여러 Task와 Execution을 포함할 수 있다. Task 종료 후에도 Thread는 유지될 수 있다.

불변조건:

- `Thread ≠ Task`
- `Thread ≠ Execution`
- `Thread ≠ Core Lease`
- `Thread ≠ Interface Session`
- `Thread ≠ Provider Session`
- Thread는 특정 LLM/Provider의 conversation handle을 Canonical identity로 사용하지 않는다.

### Interface Session

WebSocket, browser tab, CLI/TUI client connection 등 Interface의 임시 접속 상태다. connect/disconnect가 Bot/Conversation/Thread lifecycle을 생성·삭제·종료하지 않는다.

### Provider Session

Provider가 내부 최적화를 위해 유지할 수 있는 conversation/session/resume handle이다. Derived/Ephemeral이며 Provider-private optimization이다. 손실되어도 Bot/Conversation/Thread/Memory의 Canonical State가 손상되지 않는다.

## 3. 소유권

| 의미 | Canonical Owner | 비소유자 |
|---|---|---|
| Bot당 Main Conversation identity | Bot/Conversation domain | Interface, Provider |
| Thread identity/revision/lineage | Conversation/Thread domain | Task, Core, Provider |
| Thread message history | Conversation/Thread domain + durable store | Memory index, UI cache |
| Thread↔Task linkage | Conversation/Thread + Task references | Scheduler |
| Thread-local Memory scope | Memory subsystem | Thread transcript itself, Provider |
| Thread branch/archive semantics | Conversation/Thread domain | UI |
| Provider Session handle | Provider/Adapter derived state | Bot/Thread domain |

Main Conversation과 Thread의 상세 lifecycle/state는 이 문서가 의미를 소유하며, 정확한 enum/저장 구조는 ADR에서 확정할 수 있다.

## 4. Bot / Conversation / Thread 관계

```text
Persistent Bot
│
├─ Identity / Brain / Bot-global Memory
│
├─ Main Conversation
│    ├─ 일반 Bot Communication
│    ├─ Thread 생성/탐색/전환
│    ├─ Runtime Inspection
│    └─ Supervisor Control Surface
│
└─ Thread Graph
     ├─ Thread A
     │    ├─ Conversation Messages
     │    ├─ Thread-local Memory
     │    ├─ Tasks
     │    └─ Executions/Artifacts references
     ├─ Thread B
     └─ ...
```

Main Conversation에서 특정 Thread를 선택하지 않은 일반 대화도 가능하다. 이 대화가 자동으로 임의 Thread의 local context에 편입되어서는 안 된다. Thread를 생성하거나 기존 Thread에 메시지를 귀속시키는 결정은 명시적 사용자 선택 또는 versioned routing policy를 거친다.

## 5. Conversation Message와 Memory 분리

`Conversation History ≠ Memory`다.

Conversation History는 **무슨 상호작용이 있었는가**를 순서·actor·thread/provenance와 함께 보존한다. Memory는 그 기록이나 Execution 결과에서 추출·검증된 **무엇을 기억해야 하는가**를 저장한다.

따라서:

- Main Conversation/Thread transcript가 길어져도 전체 transcript를 매 LLM 호출에 넣지 않는다.
- Conversation Message 저장이 Bot-global Memory commit을 의미하지 않는다.
- Thread transcript 삭제/보관 정책과 Memory retention/forget 정책은 별도 owner가 가진다.
- Memory에는 source Conversation/Thread/Message reference를 provenance로 가질 수 있다.

Bot Network의 inter-Bot `Message`와 사용자/Bot의 `Conversation Message`는 wire/storage type을 구분한다. 같은 이름의 DTO를 공유해 semantic owner를 섞지 않는다.

## 6. Thread Scope

Thread는 최소 다음 scope를 제공한다.

- Thread identity/revision
- parent/lineage reference
- title/summary/metadata projection
- Conversation Message sequence/reference
- Thread-local Memory namespace/scope
- linked Goal/Task references
- active/suspended/terminal Task projection
- Artifact/checkpoint references
- created/archived/restored provenance
- sensitivity/retention policy reference

Thread가 Task의 state machine이나 Execution snapshot을 소유하지 않는다. 반대로 Task/Execution이 Thread identity를 생성하거나 archive하지 않는다.

## 7. Thread Lifecycle 의미

P0에서 고정하는 의미:

- create: 새 영속 Thread identity와 lineage를 생성한다.
- active/inactive 의미: 메시지/Task를 연결할 수 있는지와 별개로 Thread는 지속한다.
- archive: 신규 일반 작업 연결을 기본 차단하되 history/Memory/Artifact를 삭제하지 않는다.
- restore: 기존 identity/revision lineage를 유지해 다시 활성 사용 가능하게 한다.
- branch/fork: source Thread의 특정 revision/context reference에서 새 Thread lineage를 만든다. source transcript/Memory를 mutable 공유하지 않는다.

정확한 lifecycle enum, merge 지원 여부, archive retention은 `DXB-DEL-063` Open Question에서 ADR로 닫는다.

## 8. Thread와 Task

- 하나의 Thread는 0..N Task를 연결할 수 있다.
- 하나의 Task는 P0에서 하나의 primary Thread context를 가진다. cross-thread read/reference가 필요하면 명시적 source reference를 사용한다.
- Task 완료 후 Thread는 계속 존재한다.
- Thread archive가 running Task를 암묵 cancel하지 않는다. archive preflight가 running/suspended Task를 발견하면 정책에 따라 reject, explicit control, 또는 archive-pending operation으로 처리한다.
- Thread branch가 기존 Task/Execution을 mutation하지 않는다. 새 lineage에서 새 Task/Execution을 생성한다.
- Task revision/Execution snapshot은 Thread revision을 필요한 범위에서 pin하여 재현성을 가진다.

## 9. Thread-local Memory

Thread에서 발생한 Working 결과는 바로 Bot-global Memory로 들어가지 않는다.

기본 승격 흐름:

```text
Execution Working State
        ↓ proposal/policy
Thread-scoped Memory
        ↓ promotion proposal/policy
Bot-global Memory
```

Thread-local 정보에는 임시 가설, 조사 중간 결과, 실패한 접근, 특정 작업에만 유효한 사실이 포함될 수 있다. Bot-global 승격은 provenance/confidence/conflict/retention/sensitivity 정책을 통과한다.

명시적으로 Bot-global scope가 필요한 사용자 지시도 Memory subsystem의 정상 Commit 경로를 사용하며 Thread가 global store를 직접 mutate하지 않는다.

## 10. Thread-aware Context

Brain Context Plan은 current Thread를 명시적으로 포함한다. 전체 transcript 길이와 모델 Context 크기는 독립적이어야 한다.

Context 후보:

1. current user input
2. Bot Identity/Policy
3. current Thread identity/revision/goal intent
4. active Task/Execution state
5. Thread-local Memory
6. relevant Bot-global Memory
7. recent Thread Conversation Messages
8. retrieval된 historical Thread context
9. Artifact/checkpoint refs

선택·정렬·축약은 deterministic budget policy를 사용하며 Provider Session history를 hidden Canonical context로 신뢰하지 않는다.

## 11. Thread Branch / Lineage

branch는 source Thread의 immutable revision/reference를 남긴다. 새 Thread는 이후 독립 revision을 가진다. raw transcript나 local Memory 객체를 mutable 공유하지 않는다.

P0에서 요구하는 것은 lineage 추적과 독립성이다. semantic merge, 자동 conflict resolution, 대규모 DAG compaction은 P1 이후 ADR 대상이다.

## 12. Recovery

startup/restart 시:

1. Bot + Main Conversation identity 복원
2. Thread identities/revisions/lineage 복원
3. Conversation Message durable sequence와 projection watermark 확인
4. Thread-local Memory scope 연결 검증
5. linked Task/Execution/Continuation/Suspension 상태 reconcile
6. Provider Session handle은 있더라도 compatibility optimization으로만 재검증
7. Provider Session 손실 시 Canonical context source로 새 Provider call을 구성

Provider Session resume 실패 때문에 Thread를 새 identity로 만들지 않는다.

## 13. 동시성

- 여러 Interface Session이 같은 Main Conversation에 동시에 접속할 수 있다.
- Thread A/B/C는 동시에 Task를 실행할 수 있다.
- local Memory/Context write는 Thread scope와 revision을 명시한다.
- Thread A의 redirect/suspend/control이 Thread B/C의 Execution에 영향을 주지 않는다.
- 동일 Thread에 상충 Command가 들어오면 expected revision/idempotency/control precedence로 해결한다.
- archive vs running Task, branch vs concurrent message append 등의 race는 `DXB-RUN-030`에서 검증한다.

## 14. Interface 원칙

Interface는 하나의 Bot 화면에서 Main Conversation과 Thread Graph를 보여줄 수 있지만, UI tab/window/session을 Thread identity로 사용하지 않는다.

최소 기능:
- Main Conversation read/send
- Thread create/list/get/archive/restore/branch
- Thread message send/history pagination
- Thread↔Task/Execution status 조회
- current Thread 선택/전환
- Supervisor control 결과/acknowledgement 표시

## 15. 금지 패턴

- browser tab ID를 ThreadId로 저장
- Provider conversation/session ID를 ThreadId로 사용
- Thread를 Task와 1:1 강제
- Main Conversation을 Thread 0로 특별 취급
- 전체 Thread transcript를 항상 Provider prompt에 포함
- Thread-local Memory를 자동 Bot-global commit
- Thread branch 시 mutable Memory/Working Context 공유
- Session 종료 시 Thread/Bot lifecycle 변경

## 16. 검증 기준

- **AT-CONV-001** Bot restart와 모든 Interface Session disconnect/reconnect 후 동일 Main Conversation/Thread 목록이 복원된다.
- **AT-THREAD-001** Thread A/B/C 병렬 실행에서 local context/Memory/control이 서로 누출되지 않는다.
- **AT-CTX-002** Thread history가 증가해도 Context Plan이 정책 token/byte budget을 초과하지 않는다.
- **AT-MEM-004** Thread-local Memory가 promotion policy 없이 Bot-global Memory로 들어가지 않는다.
- **AT-SESSION-002** Provider Session 제거/손실 후에도 Bot/Conversation/Thread/Memory가 복원되고 새 Provider call을 만들 수 있다.
- Thread branch가 source Task/Execution snapshot을 mutation하지 않는다.
- Session connect/disconnect가 Conversation/Thread revision을 불필요하게 변경하지 않는다.

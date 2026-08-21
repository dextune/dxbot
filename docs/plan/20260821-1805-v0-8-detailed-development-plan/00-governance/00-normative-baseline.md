---
title: "DXBOT 최상위 기준선"
document_id: "DXB-BASE-000"
version: "0.8.0"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-SOURCE-000"]
---

# DXBOT 최상위 기준선

## 1. 목적과 효력

본 문서는 v0.8 active package의 최상위 구현 기준선이다. 이 패키지 안의 Canonical Owner만으로 현재 계약을 판정하며, 과거 v0.1~v0.7 패키지는 변경 근거와 역사 기록일 뿐 active semantic source가 아니다. `생략 = 상속` 규칙은 사용하지 않는다.

> **Persistent DXBOT Runtime → Headless Application Contract → CLI Reference Interface**

원문 보존본 `DXB-SOURCE-000`의 제품 철학은 유지한다. 다만 현재 구현 범위·책임 경계·검증 기준은 본 문서와 v0.8 Canonical Owner가 구체화한다.

## 2. 제품 불변조건

1. Bot은 Session과 독립된 Persistent Identity·Memory·State를 가진다.
2. 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
3. Core는 Scheduler-owned Dynamic Lease이며 Bot 복제본이나 독립 Agent가 아니다.
4. Bot당 하나의 Persistent Main Conversation을 유지한다.
5. `Conversation ≠ Thread ≠ Task ≠ Execution ≠ Core Lease ≠ Provider Session ≠ Interface Session`이다.
6. Project와 Channel은 협업·권한·지식·자원 Scope이며 Brain을 소유하지 않는다.
7. Conversation History와 Memory는 구분하고, Bot/Project/Channel/Thread Memory는 독립 Scope와 provenance를 가진다.
8. running Execution의 Context Plan, Task revision, Provider binding은 immutable snapshot이다.
9. Durable Process는 cross-aggregate 진행만 소유하며 child Task·Memory·Directive state를 복제하지 않는다.
10. Common Authorization Decision Owner와 Runtime Resource Owner는 각각 하나다.
11. Provider/Plugin/Interface는 Domain Identity와 Canonical State의 소유자가 아니다.
12. Bot-only path는 Project·Channel·Durable Process 없이 완전하게 동작한다.

## 3. v0.8 active product scope

Active 구현·Release 범위는 다음뿐이다.

```text
Persistent Runtime Host
→ Application Layer
→ Headless Command / Query / Subscription Contract
→ authenticated local Control Endpoint / Client
→ dxb CLI
```

TUI, Web Control Center, BFF, frontend state, 범용 remote transport, 분산 HA, 범용 Workflow DSL은 active scope가 아니다. 미래 가능성을 이유로 현재 사용하지 않는 추상화·crate·schema를 선행 생성하지 않는다.

## 4. v0.8 신규 비협상 계약

1. **Effective Baseline**: active package 밖의 문장을 조립하지 않아도 구현 판정이 가능해야 한다.
2. **Typed Application Surface**: generic `control <json>` mutation과 Domain/Store/Scheduler/Provider shortcut을 금지한다.
3. **Deterministic Resource Addressing**: Canonical ID가 최종 식별자이며 ambiguous mutation은 거부한다.
4. **Operation Receipt**: response loss, CLI crash, Runtime restart 뒤에도 mutation 결과를 durable receipt로 복구한다.
5. **Runtime Instance**: Linux 사용자 단위, data root당 active Runtime 1개와 fencing된 HostGeneration을 P0 reference로 한다.
6. **Snapshot Pagination**: Query cursor는 query/filter/sort/scope/principal/snapshot/schema에 결박된다.
7. **Explicit Stream Completion**: JSONL은 item/event와 terminal record를 구분하고 partial/gap/resync를 숨기지 않는다.
8. **Machine Contract**: stdout=result, stderr=diagnostic이며 wait/timeout/partial/exit class가 안정된 자동화 계약이다.
9. **Local Threat Model**: endpoint hijack, terminal control sequence, path traversal, symlink overwrite, partial sensitive file을 P0 공격면으로 다룬다.
10. **Schema SSOT**: public Contract schema는 Rust Application Contract source에서 결정적으로 생성한다.
11. **Executable Gates**: 문서 assertion만으로 PASS를 선언하지 않고 deterministic fixture와 CI Gate를 정의한다.
12. **Vertical Slice First**: Contract kernel과 Bot-only slice를 먼저 검증한 뒤 Project/Channel, Recovery/Streaming으로 확장한다.

## 5. Canonical Ownership

| 의미 | Canonical Owner |
|---|---|
| 문서 상태·Effective Baseline | `DXB-GOV-001` |
| 용어·식별 구분 | `DXB-GOV-002` |
| Architecture Decision | `DXB-GOV-003` |
| System/Runtime Instance 경계 | `DXB-ARC-010`, `DXB-RUN-035` |
| public Contract·Receipt·Selector·Cursor | `DXB-IFC-040` |
| CLI command/output/exit/file/terminal | `DXB-IFC-041` |
| Authorization·local endpoint security | `DXB-RUN-032` |
| Runtime resource envelope | `DXB-RUN-031` |
| Test와 3회 Review | `DXB-ENG-052` |
| Acceptance/Risk/결정 기록 | `DXB-DEL-061`~`063` |

## 6. Reference deployment 결정

P0 Reference Platform은 **Linux + user-scoped service manager + Unix-domain local IPC**다. 하나의 Runtime Instance는 하나의 persistent data root와 `InstanceId`에 결박되며 같은 instance의 active `HostGeneration`은 하나뿐이다. 다른 플랫폼은 동일 semantic을 만족하는 adapter로 후속 지원한다.

## 7. 문서 승인과 구현 의미

v0.8의 P0 normative 문서는 `Accepted` 상태다. 이는 구현자가 따라야 할 계약이 결정되었다는 뜻이며 Rust 구현·성능·보안 Acceptance가 이미 통과했다는 뜻은 아니다. 실제 실행 증거는 `DXB-ENG-052/054`와 `DXB-DEL-061`의 Gate에서 생성한다.

## 8. 완료 판정

- hidden normative inheritance 0
- active Canonical Owner 중복 0
- P0 Command Matrix 누락 0
- ambiguous selector silent selection 0
- response-loss duplicate effect 0
- duplicate Runtime instance 0
- pagination/stream silent loss 0
- machine stdout contamination 0
- endpoint/terminal/export bypass 0
- public schema drift 0
- TUI/Web active milestone·artifact 0

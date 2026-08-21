---
title: "Live Control과 Cooperative Preemption"
document_id: "DXB-RUN-036"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-023", "DXB-DOM-024", "DXB-DOM-026", "DXB-DOM-027", "DXB-RUN-030", "DXB-RUN-031", "DXB-RUN-033"]
---

# Live Control과 Cooperative Preemption

## 1. 목적

병렬 Task/Execution을 유지하면서 사용자가 Main Conversation 또는 Control API에서 실행 중 작업을 **관찰·보고 요청·방향 전환·중지 보존·재개·취소·우선순위 변경**할 수 있게 한다. 사용자 경험은 즉시 개입 가능해야 하지만 내부적으로 Execution snapshot, Side Effect safety, Scheduler ownership, crash recovery를 훼손하지 않는다.

## 2. 기본 원칙

1. 일반 Work Queue와 Runtime Control Channel을 분리한다.
2. Control Channel도 bounded이며 별도 admission/resource reserve를 가진다.
3. `inspect/report`가 backlog 뒤에서 starvation되지 않아야 한다.
4. 실행 중 `Execution` snapshot을 mutation하지 않는다.
5. 방향 변경은 durable Directive/Task Specification revision과 새 Execution으로 표현한다.
6. interruption은 cooperative preemption과 safe point를 기본으로 한다.
7. 외부 Provider/API 호출에 Hard Real-Time 중단을 약속하지 않는다.
8. Side Effect의 불명확 outcome을 제어 편의 때문에 재실행하지 않는다.
9. Control authority는 Prompt/Provider output이 획득할 수 없다.
10. Core Lease 발급·회수·rebalance 권한은 Scheduler에 유지한다.

## 3. Control Path

```text
User / Operator / Main Conversation
          ↓
Control Gateway / Authorization
          ↓
Runtime Control Channel ───────────────┐
          ↓                             │
Execution Supervisor                   │
  ├─ inspect/report                    │
  ├─ redirect                          │
  ├─ suspend/resume                    │
  ├─ cancel                            │
  ├─ reprioritize                      │
  └─ fork                              │
          ↓                             │
Durable Command/Directive/Revision     │
          ↓                             │
Safe-point / Scheduler / Recovery ─────┘
```

일반 Task 제출은 Work Queue를 사용한다. Control 요청을 일반 WorkItem 뒤에 enqueue하여 기동 중 작업이 모두 끝나야 응답 가능한 구조를 금지한다.

## 4. Command Semantic

| Command | 의미 | Canonical 효과 |
|---|---|---|
| `inspect` | 현재 committed/runtime 상태 조회 | Query, 상태 변경 없음 |
| `report` | 작업자/Runtime에 최신 진행 보고 요구 | control request + bounded report result/metadata |
| `redirect` | 목표/방향/지시 변경 | Directive commit + Task Specification revision + 기존 Execution yield/terminal + 새 Execution |
| `suspend` | 진행 상태를 보존하고 실행 중단 | durable suspend intent + safe checkpoint/Continuation + suspended semantic state |
| `resume` | 보존된 상태에서 재개 | idempotent resume command + 새 Execution/continuation consume |
| `cancel` | 작업을 폐기 방향으로 종료 | 기존 Task cancellation semantic 사용 |
| `reprioritize` | Scheduler 우선순위/자원 힌트 변경 | Task scheduling policy/revision 변경, Scheduler 재평가 |
| `fork` | 현재 상태를 보존하고 독립 분기 | 새 Thread/Task lineage + 새 Execution; source mutation 없음 |

`redirect = cancel + unrelated new Task`로 축약하지 않는다. redirect는 기존 Task/Thread lineage와 directive causation을 보존한다.

## 5. Control Directive

상태를 바꾸는 live control은 durable `Control Directive` 또는 동등한 Canonical Command 결과를 남긴다.

최소 의미:
- DirectiveId
- target Bot/Thread/Task/Execution reference
- command kind
- actor/principal + authority scope
- target expected revision/epoch
- payload or Artifact reference
- created/committed time
- correlation/causation
- idempotency key
- acknowledgement/status
- supersedes/conflicts reference where applicable
- audit reason/sensitivity

Runtime queue에만 존재하고 crash 시 사라지는 redirect/suspend를 금지한다.

`inspect`는 일반적으로 durable Directive를 만들 필요가 없으나 audit 정책상 조회 이벤트/trace를 기록할 수 있다.

## 6. Directive Revision / Epoch

Execution Supervisor는 target Task의 committed specification/control revision과 Runtime control epoch를 관찰한다. stale Directive는 적용하지 않고 conflict/superseded로 명시한다.

진행 중 Execution이 old revision을 계속 실행해도 Canonical Task revision이 이미 바뀐 경우, 그 결과는 fencing/merge 규칙을 통과해야 하며 새 revision의 성공 결과로 암묵 승격되지 않는다.

## 7. Redirect 흐름

```text
Execution N Running (Task Spec Revision R)
        ↓
Redirect Command authorize + expected revision
        ↓
Directive + Task Spec Revision R+1 commit
        ↓
Execution Supervisor signal
        ↓
safe point / cooperative yield request
        ↓
Execution N terminal/yield outcome + checkpoint/result fragment preserve
        ↓
Scheduler admission
        ↓
Execution N+1 starts with Revision R+1
```

금지:
- Execution N prompt/context/provider binding을 중간 mutation
- old Provider Session의 hidden history에만 새 지시 삽입하고 Canonical revision을 만들지 않음
- redirect 중 이미 Confirmed Side Effect를 되돌린 것으로 간주

Provider가 native steering을 제공해도 DXBOT Canonical Task revision/Directive를 먼저 확정하고 그 기능은 safe responsiveness optimization으로만 사용할 수 있다.

## 8. Safe Interruption Point

Safe point는 Capability/Provider마다 다를 수 있으나 최소 불변조건은 같다.

- Canonical commit transaction 중간을 임의 중단하지 않는다.
- non-idempotent Side Effect의 `Prepared → external call → outcome` window를 임의 재실행 가능한 상태로 만들지 않는다.
- buffered output/result fragment는 bounded하게 flush/reference하거나 명시적으로 폐기한다.
- Provider Host activity/permit/cancellation cleanup 경로를 보장한다.
- Core Lease fencing을 유지한다.
- yield 결과는 old Execution의 immutable history로 남긴다.

Harness/Provider별 safe point와 native cancellation/steering capability는 feature negotiation을 통해 표현한다.

## 9. Suspend / Resume

Suspend는 dependency 기반 `Waiting`과 다르다. 의미는 **operator/user control에 의해 실행을 보존하고 admission에서 제외한 상태**다.

Suspend 완료 조건:
1. durable suspend intent가 commit됨
2. active Execution에 cooperative interruption 요청
3. safe point에서 checkpoint/Continuation/Artifact refs를 보존
4. Provider Host activity/permit/Core Lease가 owner 규칙대로 release/quiesce
5. Task가 suspended semantic state로 durable하게 관찰 가능

Resume:
- expected revision + idempotency를 사용
- consumed suspend checkpoint/Continuation을 한 번만 사용
- 새 Execution attempt/revision snapshot을 만든다
- crash/retry에서 duplicate resume를 만들지 않는다

정확한 suspended state enum과 checkpoint body는 ADR/Open Question으로 남기되 위 의미는 P0이다.

## 10. Reprioritize / Resource Hint

사용자가 “Core 하나 더 붙여”, “이 작업을 먼저 처리해”라고 요청해도 Bot/Task schema에 fixed Core count를 넣지 않는다.

Control은 다음과 같은 policy input으로 변환할 수 있다.
- Task priority revision
- parallelism hint/range
- deadline/latency class
- resource budget adjustment request
- scheduler rebalance request

실제 Core Lease 수와 admission은 Scheduler/Resource Governance가 결정한다. resource hard ceiling을 control request가 우회할 수 없다.

## 11. Report / Inspect Freshness

외부 Provider 호출 중 즉시 최신 내부 token-level 상태를 얻을 수 없을 수 있다. report/inspect 응답은 가능한 범위에서 다음을 분리한다.

- last committed canonical state/revision
- live Runtime state
- active Execution/Core/Provider binding
- last heartbeat/progress marker
- observation timestamp/freshness
- stale/degraded flag
- pending Directive/ack state

“실시간”을 Hard Real-Time으로 표현하지 않는다. Provider 호출이 cooperative cancellation을 지원하지 않으면 stale/awaiting-safe-point를 명시한다.

## 12. Control Priority와 Starvation

Control Channel은 일반 Work Queue와 별도 bounded queue/permit class를 사용하고 resource safety reserve를 가진다.

최소 보장:
- inspect/report/cancel/safety revoke가 일반 backlog 때문에 무기한 starvation되지 않음
- control flood가 일반 work를 영구 starvation시키지 않음
- 한 Bot/Thread의 control storm이 다른 Bot/Thread control availability를 고갈시키지 않도록 fairness/rate limit
- overflow는 silent drop하지 않고 explicit rejected/coalesced/superseded semantics

정확한 precedence와 quota 숫자는 Policy SSOT/Open Question에서 결정한다.

## 13. Race Semantics

### Redirect vs Complete
같은 target revision에서 completion이 terminal commit을 먼저 얻으면 redirect는 completed target에 대한 conflict 또는 새 follow-up revision으로 처리한다. redirect commit이 먼저면 old Execution의 late completion은 new revision을 성공으로 덮지 못한다.

### Redirect vs Cancel
Cancel terminal/safety semantics가 확정된 뒤 redirect가 Task를 암묵 부활시키지 않는다. 동시 commit은 expected revision/precedence policy로 하나의 결과를 만든다.

### Suspend vs Complete
completion이 먼저 terminal이면 suspend는 no-op/conflict다. suspend가 먼저 commit되면 late old Execution result는 fencing/merge 규칙을 따른다.

### Suspend vs Side Effect
Side Effect outcome이 불명확한 window에서는 `Unknown/Reconciliation Required`를 보존하며 “suspended이므로 다시 실행”하지 않는다.

### Resume vs Duplicate Resume
suspend continuation revision + resume guard/idempotency로 새 Execution이 하나만 생성된다.

### Reprioritize vs Admission
Scheduler는 revisioned priority snapshot을 읽는다. 이미 발급된 Lease를 임의 mutate하지 않고 policy에 따라 rebalance/yield를 요청한다.

### Simultaneous Supervisor Commands
동일 target revision에 대한 상충 command는 idempotency/expected revision/precedence로 결정하며 arrival wall-clock만으로 semantic winner를 정하지 않는다.

## 14. Security / Authority

- Main Conversation의 자연어가 곧 control authority가 아니다. authenticated principal과 Bot/Thread/Task scope authorization을 거친다.
- prompt injection, Provider output, Tool output이 redirect/cancel/suspend 권한을 생성하지 못한다.
- destructive cancel/redirect/fork, resource budget 확대는 policy에 따라 approval을 요구할 수 있다.
- control reason/actor/target revision을 audit한다.
- Provider/Plugin이 Runtime Control Channel에 임의 Directive를 삽입하지 못한다.

## 15. Recovery

restart 시:
1. committed pending Directive load
2. target Task/Execution revision 확인
3. terminal/acknowledged/superseded Directive dedup
4. suspended Task checkpoint/Continuation 검증
5. in-flight Execution/Core/Provider activity reconcile
6. redirect가 old Execution을 yield시켰으나 new Execution 미생성인 경우 idempotent scheduling 재개
7. acknowledgement 전 crash한 command는 DirectiveId/idempotency로 중복 적용 방지

Runtime notification 손실을 Canonical Directive 손실로 취급하지 않는다.

## 16. Observability

Correlation:

`Bot → Main Conversation → Thread → Task → Directive → Execution → Core Lease → Capability Binding/Provider`.

metric/event 후보:
- control queue depth/bytes/latency/reject/coalesce
- report freshness/staleness
- directive commit→ack latency
- redirect commit→old execution yield→new execution start latency
- suspend→quiesced latency
- duplicate/superseded/conflict count
- safe-point reason/provider capability
- control-induced Scheduler rebalance/yield

## 17. 금지 패턴

- control command를 일반 Work Queue 뒤에만 넣음
- running Execution의 prompt/config/provider binding mutation
- Core Lease count를 사용자 명령 값으로 직접 고정
- Provider native steering을 Canonical Task state로 사용
- control queue unbounded
- external Side Effect window에서 hard kill 후 무조건 retry
- UI가 Runtime 내부 worker handle을 직접 조작
- suspend를 in-memory pause flag 하나로 구현

## 18. 검증 기준

- **AT-CTRL-002** Work Queue 포화 상태에서도 report/inspect가 일반 작업 뒤에서 starvation되지 않는다.
- **AT-CTRL-003** Running Execution N을 mutation하지 않고 redirect commit→yield→Task Revision/Execution N+1로 전환한다.
- **AT-CTRL-004** suspend 후 crash/restart해도 보존 상태에서 한 번만 resume된다.
- **AT-CTRL-005** Thread A redirect가 병렬 Thread B/C Execution/Memory/Core Lease에 영향을 주지 않는다.
- cancel/complete, redirect/complete, suspend/side-effect, duplicate resume race가 deterministic test로 판정된다.
- control flood에서 queue/resource cap이 유지되고 unrelated Bot의 control path가 starvation되지 않는다.
- Provider가 hard-preemption 미지원일 때 stale/awaiting-safe-point 상태를 명시하고 immutable Execution invariant를 유지한다.

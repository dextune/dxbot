---
title: "CLI Reference Interface"
document_id: "DXB-IFC-041"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029"]
---

# CLI Reference Interface

## 1. 목적

`dxb` CLI를 v0.7의 첫 번째 공식 **Reference Interface**로 완성한다.

목적은 두 가지다.
1. 사용자가 DXBOT 주요 기능을 별도 UI 없이 운영할 수 있다.
2. `DXB-IFC-040`이 Backend internal shortcut 없이 충분한 Application boundary인지 검증한다.

CLI는 shared Control Client/Application Contract를 사용하며 Domain/Storage/Scheduler/Provider internals에 직접 의존하지 않는다. Runtime이 아직 실행되지 않은 bootstrap 단계에는 Domain과 분리된 narrow Host Lifecycle Client만 사용할 수 있다.

## 2. Process Independence / Bootstrap

```text
                 ┌─ Host Lifecycle Client ─→ service manager / launcher
                 │
dxb CLI ──────────┤
                 │
                 └─ Control Client ─→ Control Endpoint ─→ Running Runtime Host
```

불변조건:
- CLI process lifetime ≠ Runtime lifetime
- shell/session lifetime ≠ Bot/Task/Process lifetime
- CLI SIGINT ≠ target Task cancel
- CLI disconnect/crash ≠ Bot deactivate
- CLI connection loss ≠ committed Command rollback
- explicit `cancel`, `suspend`, `runtime stop` 등 owner-defined operation만 Runtime state/lifecycle에 영향을 준다.

`dxb runtime start`는 Runtime이 아직 없을 수 있으므로 Host Lifecycle Client를 통해 process/service를 시작한 뒤 readiness/endpoint discovery를 수행한다. 이 client는 Bot/Task/Memory/Scheduler/Provider state를 읽거나 mutate하지 않는다. Runtime Ready 이후 모든 Domain use-case는 Control Client/Application Contract로만 수행한다.

## 3. Command Tree 방향

최종 명칭/alias는 schema freeze에서 확정하되 logical coverage는 다음을 기준으로 한다.

```text
dxb
├─ runtime      start | status | stop | doctor
├─ bot          create | list | show | activate | deactivate | archive | restore | delete
├─ conversation show | send | history
├─ project      create | list | show | archive | restore | members | memory | channels
├─ channel      create | list | show | archive | restore | join | leave | members | roles | authorities | send | history | threads | memory | tasks | status
├─ thread       create | list | show | send | history | archive | restore | branch | tasks
├─ goal         create | list | show | pause | resume | close
├─ task         submit | list | show | graph | inspect | report | redirect | suspend | resume | cancel | retry | reprioritize | delegate | result | watch
├─ memory       put | get | search | history | propose | promote | correct | archive | forget | compact | index
├─ process      list | show | watch | control
├─ core         list | show | watch
├─ routine      create | list | show | update | enable | disable | run-now | occurrences
├─ message      send | ask | delegate | inbox | outbox | trace
├─ capability   list | show | test
├─ provider     list | show | select-preview | deprecate | drain | detach
├─ plugin       list | show | install | validate | enable | disable | upgrade | rollback | uninstall
├─ approval     list | show | approve | deny
├─ reconcile    list | show | side-effect | waiting | suspension | directive | membership | memory
├─ config       show | validate | diff | reload
├─ trace        show | follow | export
└─ version
```

모든 하위 명령을 동시에 P0로 강제하지 않는다. P0 기준은 representative Canonical use-case를 CLI만으로 완결할 수 있는가다.

## 4. Runtime Lifecycle Command 의미

- `runtime start`: Host Lifecycle Client가 process/service start + readiness discovery. Domain mutation 없음.
- `runtime status`: process/service status와 Runtime Contract status를 구분해 표시. Runtime이 Ready하면 Control Query를 병합할 수 있음.
- `runtime stop`: 가능하면 running Runtime에 graceful shutdown/lifecycle request를 전달하고 service manager는 process supervision을 담당. kill을 Domain cancel 대체로 사용하지 않음.
- `runtime doctor`: Runtime이 Ready하면 Control Query 기반 진단, 미기동 상태에서는 host/endpoint/config 수준 진단만 수행.

exact platform service manager/IPC는 ADR 대상이다.

## 5. P0 E2E — Bot-only

```text
runtime start
→ bot create
→ conversation send
→ thread create
→ task submit
→ task inspect/watch
→ memory inspect
→ runtime restart
→ same Bot/Conversation/Thread/Task/Memory restore 확인
```

## 6. P0 E2E — Project / Channel Collaboration

```text
project create
→ channel create
→ membership/role/authority 설정
→ channel message
→ collaboration task/delegation
→ task/process watch
→ result/evidence 확인
→ scoped memory proposal/promotion
→ final result
```

Role과 effective Authority를 별도 field로 표시하며 Role label로 local permission을 판정하지 않는다.

## 7. P0 E2E — Control / Recovery

```text
running Task
→ report
→ suspend/resume or redirect
→ provider/runtime fault
→ reconcile/doctor
→ recovered state 확인
```

Runtime restart/control endpoint recreation 뒤에도 same Domain identity를 조회한다.

## 8. P0 E2E — Runtime Memory Pressure

```text
load
→ runtime status/resources
→ pressure state 확인
→ admission rejection/degradation 확인
→ Canonical state 손실 없이 recovery 확인
```

CLI가 pressure threshold/policy를 자체 계산하지 않는다.

## 9. Human Output vs Machine Output

기본 방향:

```text
human-readable default
--json
--jsonl   # streaming/multi-record where appropriate
```

원칙:
- stdout = requested result/data
- stderr = diagnostic/error/progress
- JSON/JSONL stdout에 spinner/progress/log 혼입 금지
- terminal width/color가 machine semantic을 변경하지 않음
- machine field/schema compatibility를 관리
- secret/raw sensitive field는 default redaction
- human output은 automation contract로 간주하지 않음

## 10. Exit Code Contract

script/CI가 사용할 stable error class를 정의한다.

최소 class 후보:
- success
- usage/validation error
- authorization/approval required
- conflict/stale revision
- resource/admission rejection
- runtime unavailable
- timeout/local wait cancelled
- incompatible protocol/version
- internal/recovery-required

exact 숫자는 하나의 CLI ADR/registry에서 freeze한다. human error 문자열 parsing을 automation contract로 만들지 않는다.

## 11. Mutation Safety

- mutation마다 필요한 CommandId/IdempotencyKey/expected revision을 전달한다.
- retry가 새 logical mutation을 만들지 않도록 key를 보존한다.
- destructive action은 explicit target/revision을 표시한다.
- stale conflict를 자동 overwrite하지 않는다.
- interactive confirmation과 Runtime authorization/approval은 별개다.
- TTY-aware confirmation을 사용하며 automation이 prompt에서 hang하지 않도록 `--yes`/`--non-interactive` 의미를 ADR에서 고정한다.
- `--yes`는 Permission/ActionGrant를 생성하지 않는다.

## 12. Watch / Follow / Streaming

`task watch`, `process watch`, `trace follow` 등은 다음을 지킨다.

- bounded local buffer
- cursor resume
- reconnect gap detection/resync
- SIGINT는 local subscription 종료, target Task/Process cancel 아님
- slow stdout consumer가 Runtime internal queue를 막지 않음
- broken pipe를 Runtime failure로 오판하지 않음
- server disconnect 후 current state를 재조회할 수 있음

## 13. Large Data / Pagination

`--all`을 지원하더라도 full dataset `Vec`/huge JSON array materialization을 기본으로 사용하지 않는다.

- history/memory/task/member list = paginated incremental render
- large Artifact = bounded stream/file output
- huge machine stream = JSONL 또는 paged output 우선
- stdout pipe backpressure 반영
- per-command bytes/items ceiling
- Runtime Memory Safety 우회 금지

## 14. Security / Credential

- local principal/profile은 Runtime `PrincipalRef`/SecurityDomain으로 resolve된다.
- ambient shell privilege를 Runtime Authority로 간주하지 않는다.
- command-line argv secret를 기본 credential 입력으로 사용하지 않는다.
- config/doctor/trace/export에서 secret redaction
- debug/verbose가 redaction을 자동 해제하지 않음
- approval/high-risk action은 Runtime ActionGrant/Approval semantics 사용

## 15. Runtime / Provider / Projection Observability

P0 후보:

```text
dxb runtime status
dxb runtime doctor
dxb task show/watch
dxb process show/watch
dxb reconcile ...
dxb trace show/follow
dxb provider list/show
dxb memory show/history
```

출력은 다음을 구분한다.
- host process/service state
- Canonical Runtime state
- Provider availability
- Projection/index freshness
- Derived Presence
- Runtime Memory pressure/admission
- current/stale/degraded

Doctor read-only diagnosis와 repair mutation을 분리한다.

## 16. Packaging / Compatibility

Runtime binary/service artifact와 `dxb` CLI binary는 process상 독립될 수 있다. 설치 편의와 lifecycle ownership을 혼합하지 않는다.

- old CLI ↔ compatible current Runtime
- current CLI ↔ previous compatible Runtime
- incompatible pair explicit failure
- protocol/schema/runtime/client version 표시
- host/service packaging version과 Domain data schema version 분리

## 17. Future Interface Gate

v0.7에서는 CLI까지만 정식 구현한다. 다른 product Interface는 **v0.7 M0~M5/DoD, Runtime Host 안정성, Application Contract compatibility, CLI E2E/resource/security evidence가 충분히 안정화된 뒤 별도 버전에서 계획을 재개**한다. CLI 구현 안에 미래 Interface 전용 route/view/BFF state를 선행 추가하지 않는다.

## 18. 금지 패턴

- CLI→Domain Store direct read/write
- CLI→Scheduler/Core Lease mutation
- CLI→Provider direct call
- Host Lifecycle Client→Domain mutation
- CLI local Role/Authority policy
- Runtime Canonical State local replica
- full result aggregation by default
- CLI SIGINT가 implicit Task cancel
- TUI/Web 기능을 CLI 구현과 함께 선행 개발

## 19. 검증 기준

### AT-CLI-001 — End-to-End Reference Operation
CLI만으로 Runtime/Bot/Conversation/Thread/Project/Channel/Task/Process/Memory/Control/Recovery representative P0 use-case 완결.

### AT-CLI-002 — Process Independence
CLI 정상 종료/crash/SIGINT/disconnect가 explicit command 없이 Bot/Task/Process lifecycle을 변경하지 않음.

### AT-CLI-003 — Machine Output Stability
JSON/JSONL stdout 혼입 0, stable schema/exit class, stderr diagnostic, terminal rendering과 semantic 분리.

### AT-CLI-004 — Bounded Large Output
full result heap materialization 0, paging/streaming bounded, slow consumer unbounded Runtime queue 0, CLI RSS가 total dataset과 선형 동기 증가하지 않음.

### AT-CLI-005 — No Client Policy Duplication
stale Role/Authority/Memory conflict/Scheduler pressure를 CLI가 local policy로 추측하지 않고 Runtime decision을 표시.

### AT-CLI-006 — Runtime Recovery / Reconnect
Runtime restart/endpoint recreation 뒤 same Bot/Thread/Task/Memory/Process identity를 조회하며 CLI/Provider Session을 Domain identity로 오인하지 않음.

### AT-CLI-007 — Active Interface Scope
v0.7 active Interface normative docs는 `DXB-IFC-040/041`만 존재하고 superseded Interface 전용 milestone/gate가 release blocker로 남지 않음.

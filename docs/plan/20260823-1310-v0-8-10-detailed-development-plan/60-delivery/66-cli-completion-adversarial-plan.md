---
title: "CLI 개발 완료 적대적 종료 플랜"
document_id: "DXB-DEL-066"
version: "0.8.10"
status: "Accepted"
normative: false
priority: "P0"
last_updated: "2026-08-29"
depends_on: ["DXB-DEL-060", "DXB-DEL-061", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-RUN-033", "DXB-RUN-035"]
---
# CLI 개발 완료 적대적 종료 플랜

## 1. 목적

이 문서는 DXBOT v0.8.10의 CLI를 **기능 확장 없이 실제 사용자 관점에서 완료 상태로 수렴**시키기 위한 종료(closeout) 실행 계획이다.

현재 CLI의 parser, contract projection, journal, recovery, rendering, selector, confirmation, directive 등 개별 구성요소는 상당 부분 구현되어 있고 기존 Acceptance evidence도 존재한다. 그러나 제품 완료 판정은 component/unit test 통과가 아니라 실제 `dxb` binary가 verified Runtime endpoint를 선택하고, authenticated Control boundary를 통과하며, Application/Runtime owner까지 요청을 전달하고, 실패·재시작·복구 후에도 동일 operation identity를 보존하며, human/machine output 계약을 끝까지 지키는지로 판정한다.

이 문서는 다음 Canonical Owner를 변경하지 않는다.

- CLI surface, local selection, rendering, local journal 의미: [DXB-IFC-041](../40-interfaces/41-cli.md), [DXB-IFC-043](../40-interfaces/43-cli-user-journeys.md)
- typed input, preflight, command projection: [DXB-IFC-042](../40-interfaces/42-cli-input-contract.md)
- milestone과 기존 Acceptance registry: [DXB-DEL-060](60-implementation-roadmap.md), [DXB-DEL-061](61-acceptance-traceability.md)
- recovery semantics: `DXB-RUN-033`
- Runtime bootstrap/discovery semantics: `DXB-RUN-035`

이 문서는 위 계약을 복제하거나 재정의하지 않고, **기존 계약이 실제 production binary 경로에서 실행 가능한지 판정하는 추가 종료 Gate**만 정의한다.

## 2. Scope Lock

### 2.1 허용 범위

이번 closeout에서 허용하는 변경은 다음뿐이다.

1. 기존 63개 P0 command를 실제 binary에서 접근 가능하게 연결한다.
2. 기존 CLI parser/help/preflight/Control/Host client/renderer/journal/recovery 계약의 미연결 경로를 연결한다.
3. 기존 owner가 이미 정의한 operation을 실제 Application/Runtime 경계까지 전달한다.
4. component test를 binary/subprocess/integration/fault evidence로 승격한다.
5. 중복 parser, 임시 transport, fixture-only persistence가 production path에 남아 있으면 제거하거나 test-only로 격리한다.
6. 실행 가능성을 위해 필요한 최소 dependency wiring과 private adapter를 추가한다.
7. 발견된 계약 위반, false-success, identity drift, output contamination, recovery ambiguity를 수정한다.

### 2.2 금지 범위

다음은 CLI 완료 작업에 포함하지 않는다.

- 신규 CLI command 또는 command group
- 63-operation registry 외 신규 public operation
- wizard, shell completion subsystem, TUI/Web UI
- generic RPC/IDL/framework 도입
- Plugin management CLI
- hidden current Bot/Project 같은 암묵 authority context
- Runtime 자동 시작 확대
- stale CAS 자동 mutation retry
- 새로운 Provider 제품 추가
- CLI가 Domain state, Authority, Receipt lifecycle을 소유하도록 하는 변경
- 문서 완료를 이유로 실제 binary evidence 없이 `CLI Complete`를 선언하는 행위

기존 operation을 실행 가능하게 만들기 위해 owner 내부 구현이 부족한 경우 이는 신규 기능 추가가 아니라 **기존 frozen contract의 executable closure**로 분류한다.

## 3. 현재 Baseline과 Blocking Findings

기준 commit은 `d537e5273c7b1468f5c3606b65b8e1847737eabe`다. 구현 시작 전 최신 `main`과 비교하여 finding이 이미 해소됐는지 다시 확인한다.

### BF-CLI-001 — production entrypoint의 미연결 command

현재 `crates/cli/src/main.rs`는 다수 command를 `not_wired()`로 종료한다. 일부 first-use command도 payload projection 성공 후 실제 authenticated Runtime submission 대신 `RuntimeUnavailable`로 끝난다.

**위험:** component projection test가 PASS여도 사용자는 실제 operation을 만들 수 없다.

**종료 조건:** production binary path의 `not_wired()` 호출 0개. 지원하지 않는 command가 있다면 registry 자체가 잘못된 것이므로 registry/owner contract와 함께 해결하며 false-success 또는 silent omission을 허용하지 않는다.

### BF-CLI-002 — hardcoded Instance/Principal

production entrypoint가 실제 discovery 결과 대신 기본 `InstanceId`/`PrincipalRef`를 직접 구성하면 multi-instance 선택, transport authentication, journal identity와 충돌할 수 있다.

**종료 조건:**

- selected Instance는 `DXB-RUN-035` precedence를 통과한 verified descriptor에서 나온다.
- authenticated Principal은 trusted local transport boundary에서 유도한다.
- CLI payload의 principal field는 consistency binding일 뿐 authentication evidence가 아니다.
- production path에 hardcoded authenticated principal이 없다.

### BF-CLI-003 — Control transport 부재

`SubmissionClient`는 submission/replay identity와 fail-closed transport semantics를 갖지만 production CLI가 verified local Control endpoint에 연결되는 concrete path가 없다.

**종료 조건:** 실제 `dxb` subprocess가 authenticated local Control boundary를 통해 request/result 왕복을 수행한다. 새 generic RPC platform은 만들지 않는다.

### BF-CLI-004 — durable CLI journal과 submission fixture 분리

CLI에는 file-backed `LocalJournal`이 있으나 `control-client`의 `JournalStore`는 in-memory fixture다. production submission이 fixture store만 사용하면 process restart recovery와 local durable journal이 하나의 실행 경로로 연결되지 않는다.

**종료 조건:** production submission은 CLI가 소유한 durable journal semantics를 사용하고, in-memory store는 test fixture로만 남는다. exact Rust trait/callback 형태는 구현 세부이며 public contract로 고정하지 않는다.

### BF-CLI-005 — command registry와 binary dispatch drift 가능성

63개 command registry는 `application-contract`에 존재하지만 binary entrypoint에 별도의 수작업 group/subcommand dispatch와 help 문자열이 존재한다.

**종료 조건:** command 존재 여부, typed input, wait/security/output metadata의 canonical source는 기존 registry다. CLI path mapping에 필요한 최소 metadata view만 사용하고 동일 semantic metadata를 두 번째 표로 복제하지 않는다.

### BF-CLI-006 — Application/Runtime owner의 실행 coverage 부족 가능성

CLI가 payload를 만들 수 있어도 downstream owner가 해당 command를 실제 처리하지 않으면 binary는 완료되지 않는다.

**종료 조건:** 63개 command 각각에 대해 `Parse → Preflight → Dispatch → Owner → Result/Stream → Recovery → Render` 경로가 존재하거나 해당 kind에 맞는 Host/Query/Stream 경로가 존재한다.

### BF-CLI-007 — 기존 PASS evidence와 product completion의 수준 차이

기존 Acceptance `Passed`는 유지한다. 다만 component/projection evidence를 실제 CLI product completion과 동일시하지 않는다.

**종료 조건:** 아래 Completion Level의 `Journey PASS`까지 별도 evidence가 존재한다.

### BF-CLI-008 — parser-side identity helper의 production 오용 위험

`application-contract::CliInput`에는 local option에서 `InstanceId`와 `PrincipalRef`를 구성하는 convenience helper가 존재한다. profile은 Instance identity가 아니고 client-supplied principal은 authentication evidence가 아니므로 이 helper를 production selection/authentication source로 사용하면 계약을 위반한다.

**종료 조건:** production path의 Instance는 verified discovery 결과에서, authenticated Principal은 trusted transport에서만 온다. convenience helper가 test/projection 용도라면 그 경계를 명확히 하고 production orchestration에서 호출하지 않는다.

### BF-CLI-009 — version/compatibility representation drift 가능성

CLI discovery와 control-client가 protocol/schema version을 서로 다른 문자열 형태로 소유하면 local version 출력과 remote compatibility 판정이 어긋날 수 있다.

**종료 조건:** version/protocol/schema compatibility의 Canonical Owner를 하나로 유지하고 CLI/Control client는 동일 representation/source를 사용한다. 표면 문자열을 임의로 normalize하거나 추측하지 않는다.

### BF-CLI-010 — global option parsing과 실제 동작 분리

production entrypoint가 `--profile`, `--instance`, `--color`, `--wait`, `--timeout` 값을 소비만 하고 execution/selection/rendering에 적용하지 않으면 사용자는 option이 적용됐다고 오판한다.

**종료 조건:** 등록된 global option은 계약대로 실제 동작하거나, 구현되지 않은 상태에서는 성공처럼 무시하지 않고 fail closed한다. option parsing과 execution semantics를 같은 binary test에서 검증한다.

### BF-CLI-011 — group/command help의 binary 계약 미폐쇄

계약은 `dxb <group> --help`, `dxb <command> --help`가 Runtime 없이 exit 0이어야 하지만 top-level help만 동작하고 subcommand 위치의 `--help`가 일반 subcommand로 해석되면 계약 위반이다.

**종료 조건:** top/group/command help가 canonical metadata에서 생성되고 Runtime 접근 없이 exit 0을 반환한다.

## 4. Completion Level

CLI 종료 판정은 다음 계층을 구분한다.

| Level | 의미 | 대표 evidence | CLI Complete 판정 |
|---|---|---|---|
| L0 Contract | registry/schema/document 관계 일치 | validator/golden | 불충분 |
| L1 Projection | `CliInput → CommandPayload` 정확 | unit/component | 불충분 |
| L2 Component | selector/journal/recovery/renderer 개별 invariant | crate tests | 불충분 |
| L3 Boundary | verified endpoint/authenticated Control/Host 경계 왕복 | integration | 불충분 |
| L4 Binary | 실제 build된 `dxb` subprocess가 명령 실행 | subprocess tests | 필수 |
| L5 Journey | 사용자 여정 정상·실패·복구를 binary로 종료 | E2E/fault | **최종 완료 조건** |

기존 `DXB-DEL-061`의 상태를 소급 변경하지 않는다. 이 문서는 CLI 제품 완료에 필요한 추가 종료 Gate를 정의한다.

## 5. Canonical Execution Spine

최종 production 경로는 다음 책임 흐름을 유지해야 한다.

```text
argv/stdin/file/artifact
        ↓
CLI path resolution
        ↓
CliInput
        ↓
verified Instance selection
        ↓
bounded preflight / canonical target + CAS materialization
        ↓
CommandPayload
        ↓
CommandId + OperationId + IdempotencyKey + RequestDigest
        ↓
durable Prepared
        ↓
durable Dispatching
        ↓
authenticated Control/Host/Query/Stream boundary
        ↓
Application / Runtime / Provider owner
        ↓
Receipt / Result / Stream event
        ↓
Observed / Terminal 또는 recovery-required
        ↓
human | json | jsonl renderer
```

다음 우회는 금지한다.

```text
CLI → direct Domain mutation
CLI → fabricated success payload
CLI → unverified endpoint
CLI → client-supplied principal을 authentication으로 신뢰
CLI → in-memory-only production journal
CLI → provider-specific branch
```

## 6. 실행 단계

### C0 — Completion Evidence 재분류

**목적:** 기존 PASS를 무효화하지 않으면서 product completion 오판을 제거한다.

#### 작업

- 기존 Acceptance evidence를 L0~L5 중 어느 수준인지 표시할 수 있는 closeout matrix를 만든다.
- `AT-CLI-CORE-001`, `AT-CLI-AUTOMATION-001`, `AT-CLI-RECOVERY-001`, `AT-SCHEMA-001` 등 CLI 관련 acceptance가 실제 binary evidence를 포함하는지 확인한다.
- library method 직접 호출만 검증하는 test는 L1/L2로 유지한다.
- `std::process::Command` 또는 동등한 실제 binary subprocess evidence만 L4로 인정한다.
- 여러 binary invocation과 persisted restart를 포함해야 L5로 인정한다.

#### Gate

- CLI 관련 Acceptance마다 현재 evidence level이 명확하다.
- `Passed` 문자열만 보고 CLI Complete를 선언하는 경로가 없다.

### C1 — Parser/Help/Registry 단일화

**목적:** 수작업 parser/help와 frozen command registry의 drift 제거.

#### 작업

- 63개 command key와 CLI path의 exact coverage를 자동 검증한다.
- `dxb`, group help, command help가 Runtime 없이 동작하는지 검증한다.
- required/optional/local/source/CAS/wait 정보가 canonical metadata와 일치하도록 한다.
- `main.rs`의 command semantics 재정의를 제거하고 entrypoint는 path resolution과 orchestration만 담당하게 한다.
- crate boundary 때문에 metadata view가 필요하면 기존 registry에서 생성/노출하며 두 번째 canonical registry를 만들지 않는다.

#### 적대적 입력

- unknown group/subcommand
- typo suggestion threshold 경계
- duplicate option
- global option 순서 permutation
- option value 누락
- `--` 이후 positional
- content source 복수 지정
- selector 복수 지정
- 잘못된 `--wait`, `--format`, `--color`
- command help에서 Runtime 접근 시도
- non-UTF8/pathological argv는 플랫폼 지원 범위에서 fail closed

#### Gate

- registry 63개 중 binary parser 접근 불가 command = 0
- registry에 없는 success path = 0
- help/runtime dependency = 0

### C2 — Instance Selection과 Authenticated Principal 수렴

**목적:** local discovery hint와 실제 Runtime identity/authentication을 분리하면서 정확히 연결.

#### 작업

- `--instance → profile binding → exactly one verified endpoint` precedence를 production entrypoint에 적용한다.
- stale/unverified descriptor를 정상 endpoint처럼 사용하지 않는다.
- Runtime restart generation 변화 시 stale descriptor를 거부하거나 재검증한다.
- profile은 selection hint일 뿐 Authority/journal identity가 아님을 유지한다.
- principal은 local authenticated transport에서 유도하고 payload principal과 일치 여부를 서버 경계에서 검증한다.

#### 적대적 시나리오

- verified Instance 0개
- verified Instance 2개 이상
- explicit unknown Instance
- profile collision
- stale host generation
- endpoint file corruption
- endpoint는 존재하지만 peer authentication 실패
- client payload principal spoof
- suspended/revoked principal
- selected Instance와 request Instance mismatch

#### Gate

- silent fallback = 0
- hardcoded production Instance = 0
- hardcoded authenticated Principal = 0
- authentication 실패 후 journal Dispatching/network send 발생 = 0

### C3 — Local Control/Host/Query/Stream Transport 연결

**목적:** projection과 Runtime 사이의 끊긴 production spine을 연결.

#### 원칙

새 범용 RPC/IDL을 도입하지 않는다. 기존 endpoint descriptor와 Control/Host/Application contract에 필요한 최소 local transport만 구현한다.

#### command kind별 경계

| Kind | 실행 경계 |
|---|---|
| `C` | durable submission → authenticated Control → Application mutation |
| `Q` | authenticated bounded query |
| `S` | authenticated subscription/stream + cursor |
| `H` | Runtime Host control |
| `H/Q` | Host/query diagnostic |

모든 command를 mutation transport 하나로 왜곡하지 않는다.

#### 작업

- `control-client`에 production local transport adapter를 연결한다.
- transport는 complete `OperationRequest` identity를 보존한다.
- RuntimeUnavailable, auth failure, connection reset, incompatible protocol을 typed error로 매핑한다.
- transport retry가 unknown side effect를 새 operation으로 재발행하지 않게 한다.
- Host command와 Application command ownership을 분리한다.
- Query/Stream은 mutation journal semantics를 억지로 복제하지 않는다.

#### Gate

- `not_wired()` production call = 0
- fake/in-memory transport production use = 0
- request identity regeneration = 0

### C4 — Durable Journal과 Submission Protocol 수렴

**목적:** file-backed CLI journal과 submission/replay protocol을 하나의 production execution path로 결합.

#### Canonical Owner

- local durable journal 파일/retention/lock/hash-chain 의미: CLI
- submission/replay identity protocol: control-client
- canonical operation/result: Runtime/Application

#### 구현 제약

- `JournalStore`는 test fixture임을 유지한다.
- production `SubmissionClient`가 durable journal port를 사용하도록 한다.
- port의 Rust 형태는 trait/callback/adapter 중 최소 형태로 구현하고 public product contract로 승격하지 않는다.
- 동일 state를 두 저장소에 dual-write하지 않는다.
- `Prepared` 이후 CommandId/OperationId/IdempotencyKey/RequestDigest를 재생성하지 않는다.

#### crash window

1. identity materialization 전
2. Prepared append 전
3. Prepared fsync 직후
4. Dispatching append 전
5. Dispatching fsync 직후 / first network byte 전
6. server commit 직후 / client response 전
7. Observed append 전
8. Observed 후 / Terminal 전
9. Terminal 후 process kill
10. restart + takeover/recovery

#### 핵심 invariant

server commit 후 response loss가 발생해도 다음 실행은 binding lookup/replay 규칙으로 원 operation을 찾는다. 새 IDs로 동일 mutation을 다시 수행하면 실패다.

#### Gate

- production in-memory-only journal = 0
- dual canonical journal = 0
- response loss 후 duplicate domain mutation = 0
- crash/restart 후 operation identity drift = 0

### C5 — 63 Operation Executability Closure

**목적:** frozen P0 surface 전체에 대해 실행 경로를 증명.

각 command는 아래 matrix의 모든 적용 가능 column을 evidence로 채운다.

| Column | 판정 |
|---|---|
| Path | user-facing CLI path가 registry key에 정확히 매핑 |
| Parse | typed input validation |
| Select | Instance/principal selection |
| Preflight | 필요한 selector/CAS materialization |
| Dispatch | kind별 production boundary |
| Owner | Application/Runtime/Provider owner가 실제 처리 |
| Result | output schema/exit semantics |
| Recovery | mutation/stream에 적용되는 recovery |
| Binary | 실제 `dxb` subprocess evidence |

#### 63-command inventory

```text
runtime-start
runtime-status
runtime-stop-graceful
runtime-stop-host
runtime-doctor
version

bot-create
bot-list
bot-show
bot-activate
bot-deactivate
bot-archive
bot-restore

conversation-show
conversation-send
conversation-history

thread-create
thread-list
thread-show
thread-send
thread-history
thread-branch

task-submit
task-list
task-show
task-watch
task-cancel
task-suspend
task-resume
task-redirect
task-result

memory-get
memory-search
memory-history
memory-propose
memory-promote

project-create
project-list
project-show
project-archive
project-restore
project-member-set
project-member-remove
project-member-list

channel-create
channel-list
channel-show
channel-member-set
channel-member-remove
channel-member-list
channel-send
channel-history

process-show
process-watch

operation-show
operation-reconcile

approval-list
approval-show
approval-approve
approval-deny

provider-list
provider-show

side-effect-reconcile
```

#### Gate

- inventory count = 63
- registered-but-unreachable = 0
- reachable-but-unregistered = 0
- owner 없는 operation = 0
- unsupported path를 success처럼 렌더링 = 0

### C6 — Rendering / Automation Closure

**목적:** 실제 Runtime 결과와 실패가 human/machine contract를 오염시키지 않게 한다.

#### 반드시 검증할 모드

```text
human × TTY
human × non-TTY
json × TTY
json × non-TTY
jsonl × TTY
jsonl × non-TTY
```

#### failure matrix

- runtime unavailable
- provider unavailable
- permission denied
- approval required
- CAS conflict
- ambiguous target
- recovery required
- version incompatible
- stream cursor gap
- local timeout
- SIGINT/SIGTERM
- broken pipe/pager exit
- `--all` local ceiling
- destination already exists
- disk full/write failure
- corrupt local journal

각 case에서 아래를 함께 확인한다.

| 검증 | 요구 |
|---|---|
| exit code | canonical exit registry와 일치 |
| stdout | result/stream schema만 존재 |
| stderr | diagnostic/progress만 존재 |
| next action | typed action code + command key + args |
| secret | content/credential/policy 내부 유출 없음 |
| operation continuity | local observation 종료와 Runtime cancel을 혼동하지 않음 |

#### Gate

- machine stdout human contamination = 0
- schema-bearing stderr payload = 0
- nonzero exit에서 unparsable machine error = 0
- local interrupt의 implicit Runtime cancel = 0

### C7 — UJ-001~UJ-010 Binary Journey 승격

새 user journey를 만들지 않는다. [DXB-IFC-043](../40-interfaces/43-cli-user-journeys.md)의 기존 UJ-001~UJ-010을 실제 subprocess E2E로 승격한다.

#### 최소 first-use chain

```text
dxb runtime start
dxb bot create alpha
dxb conversation send alpha --stdin
dxb task submit --owner alpha --stdin
dxb task show <selector>
dxb task result <selector>
```

#### Journey별 최소 evidence

| Journey | Binary evidence |
|---|---|
| UJ-001 | fresh state에서 help/version/runtime start/selection |
| UJ-002 | bot create → Main Conversation send |
| UJ-003 | selector mutation + stale CAS conflict + safe resubmit guidance |
| UJ-004 | non-TTY json/jsonl automation |
| UJ-005 | approval-required → show → approve/deny |
| UJ-006 | provider unavailable → doctor provider |
| UJ-007 | ambiguity → visible exact candidate 선택 |
| UJ-008 | process crash/interrupt → journal scan → original operation recovery |
| UJ-009 | pagination/`--all`/safe output/partial resume |
| UJ-010 | multi-instance exact selection + compatibility |

library API 직접 호출만으로는 이 Gate를 통과할 수 없다.

#### Gate

- UJ-001~UJ-010 L5 PASS = 10/10
- journey 중간에 fixture-only backdoor 호출 = 0

### C8 — Adversarial Fault/Concurrency Review

**목적:** 정상 입력이 아니라 경계 조건에서 completion invariant를 깨뜨린다.

sleep 길이에 의존하는 race test를 금지한다. barrier/failpoint/fake clock/process crash를 우선한다.

#### 공격 목록

| 공격 | 기대 invariant |
|---|---|
| 두 CLI가 동일 command를 동시에 recover/replay | 하나의 operation identity |
| Dispatching fsync 직후 kill | restart 후 binding lookup |
| server commit 후 response loss | duplicate mutation 없음 |
| corrupt middle journal record | fail closed |
| truncated final journal record | 마지막 완전 record까지만 허용 |
| Runtime generation 교체 | stale generation fencing |
| stale CAS | 자동 retry 금지 |
| local timeout | observation만 종료, operation continuity 표시 |
| SIGINT during wait | implicit cancel 없음 |
| stream cursor future/gap | explicit resync |
| huge page/`--all` | item/byte ceiling |
| huge stdin/input-file | Prepared 전 bounded materialization |
| malicious output path/symlink | overwrite/follow 금지 |
| disk full during journal append | network send 전 fail closed |
| auth principal mismatch | owner mutation 전 거부 |
| provider connection reset | provider-specific branch 없이 typed failure |
| shutdown under load | new admission과 drain 경계 보존 |

#### Gate

- probabilistic-only race evidence = 0
- unbounded queue/buffer/scan = 0
- unknown side effect를 transient retry로 숨김 = 0

### C9 — Removal Review와 Final Hardening

완료 직전 새 코드를 추가하기보다 임시 구조를 제거한다.

#### 삭제/격리 대상 검사

```text
production not_wired call
hardcoded default Instance
hardcoded authenticated Principal
duplicate command semantic registry
fixture-only transport in production
in-memory-only production journal
success-like unsupported path
dead compatibility shim
orphan config/dependency
unused recovery fallback
```

`CoreCommands` 등 convenience projection이 canonical parser/executor와 의미를 중복하면 축소 또는 제거한다. 단, 테스트 편의를 위해 남길 경우 production owner를 가리는 두 번째 policy source가 아니어야 한다.

#### Gate

모든 항목 0 또는 명시적 test-only 근거가 존재한다.

## 7. 구현 Wave와 의존관계

단계 번호는 검수 영역이며 실제 구현은 아래 순서가 효율적이다.

```text
Wave 0
C0 baseline/evidence classification

Wave 1 — execution spine
C2 Instance/Auth
  ↓
C3 Control/Host/Query/Stream transport
  ↓
C4 durable journal + submission convergence

Wave 2 — full surface
C1 parser/help/registry convergence
  ↓
C5 63-operation owner closure

Wave 3 — user-facing completion
C6 rendering/automation
  ↓
C7 10 binary journeys

Wave 4 — release closure
C8 adversarial fault/concurrency
  ↓
C9 removal review
  ↓
Final two rechecks
```

C3/C4가 닫히기 전에 command별 wrapper를 대량 추가하지 않는다. execution spine이 확정되지 않은 상태에서 63개 dispatch wrapper를 먼저 만들면 중복 transport/recovery logic이 확산될 위험이 크다.

## 8. 예상 영향 범위

확정 파일 목록이 아니라 implementation inventory다.

```text
crates/cli/
crates/application-contract/
crates/control-client/
crates/control-server/
crates/application/
crates/runtime-bootstrap/
crates/runtime-host/
crates/runtime-security/
crates/runtime-audit/
crates/provider-host/          # 기존 provider query/diagnostic 연결 범위만
docs/plan/20260823-1310-v0-8-10-detailed-development-plan/
```

새 crate는 기본안이 아니다. 기존 Canonical Owner 내부에서 닫을 수 없다는 증거가 생길 때만 별도 ADR/계획 변경을 요구한다.

## 9. Test Architecture

### 9.1 계층

```text
deterministic unit/state
→ contract/golden
→ component
→ local transport integration
→ binary subprocess
→ process crash/restart
→ adversarial concurrency/fault
→ optional real Provider canary
```

외부 Provider 비결정성은 CLI correctness acceptance로 사용하지 않는다.

### 9.2 Binary harness 요구

binary test harness는 다음을 격리 가능해야 한다.

- temporary state root
- deterministic Instance identity
- authenticated local endpoint
- Runtime host lifecycle
- fake/reference Provider
- failpoint/crash point
- stdout/stderr capture
- TTY/non-TTY mode
- process kill/restart
- persisted journal/runtime state

test harness가 production 경계를 우회해 Domain API를 직접 호출하면 해당 test는 L4/L5 evidence가 아니다.

## 10. Completion Metrics

최종 보고에는 최소 다음 값을 기록한다.

```text
registered_operations = 63
binary_reachable_operations = 63
user_journeys = 10
binary_passed_user_journeys = 10

production_not_wired = 0
production_hardcoded_instance = 0
production_hardcoded_authenticated_principal = 0
production_fixture_transport = 0
production_in_memory_only_journal = 0
unreachable_registered_command = 0
unregistered_success_path = 0
```

테스트 개수 자체는 완료 지표가 아니다. invariant/경로 coverage가 완료 지표다.

## 11. Verification Commands

구현 환경에서 다음을 모두 수행한다.

```bash
cargo fmt --check
cargo check --workspace
cargo clippy --workspace
cargo test --workspace

python3 scripts/plan-validator.py --active
python3 scripts/plan-validator.py --self-test
```

추가 closeout suite는 repository test naming 규칙에 맞추되 최소 다음 목적을 독립적으로 식별 가능해야 한다.

```text
cli-binary-contract
cli-binary-journeys
cli-binary-recovery
cli-binary-automation
cli-binary-fault
```

명령 이름은 구현 시 기존 test layout에 맞춰 결정하며 이 문서가 Cargo test target 이름을 public contract로 고정하지 않는다.

## 12. Final Recheck 1 — Structural / Consistency

다음을 전부 검토한다.

1. 63 command registry와 binary path 1:1 coverage
2. duplicate parser/help/wait/security/output policy
3. CLI/Control/Application/Runtime Canonical Owner 중복
4. CLI journal과 submission fixture state 중복
5. crate dependency direction과 cycle
6. public schema가 불필요하게 확대되지 않았는지
7. docs/registry/golden/test 관계
8. unused feature/dependency/config
9. lowercase kebab-case / Rust snake_case
10. 기존 v0.8.10 비범위 위반 여부

발견 사항을 수정한 후 동일 범위를 다시 검수한다.

## 13. Final Recheck 2 — Cross-Layer Executability

다음 흐름을 실제 binary evidence로 추적한다.

```text
Input/Command
→ Instance/Auth
→ Preflight
→ Durable Journal
→ Control/Host/Query/Stream
→ Application/Runtime Owner
→ Persistence/Receipt
→ Provider/Process where applicable
→ Crash/Restart Recovery
→ Projection
→ CLI Renderer
```

각 적용 가능 command에 다음 fault를 대입한다.

```text
runtime unavailable
provider unavailable
permission denied
approval required
cancel/timeout
crash/restart
duplicate delivery
partial success
stale revision/generation
journal corruption
resource pressure
shutdown/drain
```

발견 사항을 수정한 후 해당 binary journey와 workspace regression을 다시 수행한다.

## 14. CLI Complete 최종 Gate

아래 조건을 하나라도 만족하지 못하면 `CLI Complete`를 선언하지 않는다.

### Contract/Surface

- 63/63 operation path verified
- command/input/golden registry drift 없음
- 신규 command 0

### Production Wiring

- verified Instance selection 사용
- trusted authenticated Principal 사용
- authenticated local transport 연결
- durable journal/submission convergence
- `not_wired` production path 0

### Correctness/Recovery

- Prepared/Dispatching/Observed/Terminal crash window 검증
- response-loss duplicate 방지
- original replay identity 보존
- stale generation/CAS fail closed
- corrupt/unknown journal auto-replay 금지

### UX/Automation

- UJ-001~UJ-010 binary PASS
- human/json/jsonl contract PASS
- non-TTY prompt 없음
- machine stdout/stderr 분리
- typed next action 유지
- safe output/partial resume PASS

### Quality

- workspace fmt/check/clippy/test PASS
- active plan validator/self-test PASS
- Final Recheck 1 PASS
- Final Recheck 2 PASS
- 발견 사항 수정 후 재검증 PASS

## 15. 중단 조건

다음 상황이 발생하면 구현을 중단하고 plan/ADR 재검토를 먼저 수행한다.

1. CLI 완료를 위해 신규 command/public workflow가 필요해짐
2. generic RPC/IDL/framework가 선행 조건으로 변함
3. CLI가 Authority/Domain canonical state를 소유해야 한다는 설계가 등장함
4. durable journal을 두 owner가 동시에 canonical로 유지하려 함
5. Provider-specific branch가 CLI/Application에 유출됨
6. recovery를 위해 새로운 operation identity를 만드는 방식이 필요하다고 판단됨
7. 63-operation contract 자체의 의미 변경이 필요해짐
8. binary journey를 통과시키기 위해 fixture-only bypass가 production path에 들어감

## 16. 작업 단위와 Evidence 기록

각 change set은 가능한 한 다음 단위로 유지한다.

```text
blocking finding
+ owner 확인
+ 최소 production wiring
+ deterministic component test
+ binary/integration evidence
+ failure/recovery evidence
+ 관련 문서 갱신
+ review finding 수정
```

각 PR/commit 또는 main change에는 최소 다음을 기록한다.

```text
- closeout stage: Cx
- blocking finding IDs
- affected command keys
- Canonical Owner
- public contract impact: none/changed
- binary evidence
- fault/recovery evidence
- resource bound
- remaining blockers
- Recheck 1 result
- Recheck 2 result
```

## 17. 최종 판정 원칙

CLI 개발 종료는 “파일이 존재한다”, “63 command가 registry에 있다”, “component test가 많다”, “workspace test가 한 번 통과했다”로 판정하지 않는다.

최종 판정은 오직 다음 명제로 한다.

> **기존 v0.8.10 P0 surface를 확장하지 않고, 실제 `dxb` binary가 모든 등록 operation을 올바른 Canonical Owner까지 전달하며, 실패·중단·재시작·부분 성공에서도 identity와 recovery semantics를 보존하고, 10개 사용자 여정을 human/machine interface로 재현 가능하다.**

이 명제가 executable evidence로 증명되면 CLI closeout을 완료한다.

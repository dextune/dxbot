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

이 문서는 새로운 제품 의미를 정의하지 않는다. 다음 Canonical Owner가 이미 소유하는 계약을 production binary에서 실제로 실행 가능한지 검증하고 미연결 부분을 폐쇄한다.

- CLI surface, local selection, rendering, local journal: [DXB-IFC-041](../40-interfaces/41-cli.md), [DXB-IFC-043](../40-interfaces/43-cli-user-journeys.md)
- typed input, preflight, command projection: [DXB-IFC-042](../40-interfaces/42-cli-input-contract.md)
- milestone과 기존 Acceptance registry: [DXB-DEL-060](60-implementation-roadmap.md), [DXB-DEL-061](61-acceptance-traceability.md)
- recovery semantics: `DXB-RUN-033`
- Runtime bootstrap/discovery semantics: `DXB-RUN-035`

`CLI Complete`는 이 문서의 Binary/Journey Gate가 executable evidence로 모두 닫힌 뒤에만 선언한다.

## 2. Scope Lock

### 2.1 허용 범위

이번 closeout에서 허용하는 변경은 다음뿐이다.

1. 기존 63개 P0 command를 실제 binary에서 접근 가능하게 연결한다.
2. 기존 parser/help/preflight/Control/Host client/renderer/journal/recovery 계약의 미연결 경로를 연결한다.
3. 기존 owner가 이미 정의한 operation을 실제 Application/Runtime 경계까지 전달한다.
4. component test를 binary/subprocess/integration/fault evidence로 승격한다.
5. 중복 parser, 임시 transport, fixture-only persistence가 production path에 남아 있으면 제거하거나 test-only로 격리한다.
6. 실행 가능성을 위해 필요한 최소 dependency wiring과 private adapter를 추가한다.
7. 발견된 contract violation, false-success, identity drift, output contamination, recovery ambiguity를 수정한다.
8. branch/provenance가 실제 제품 상태를 오판하게 만드는 stale execution branch를 정리한다.

### 2.2 금지 범위

다음은 CLI 완료 작업에 포함하지 않는다.

- 신규 CLI command 또는 command group
- 63-operation registry 외 신규 public operation
- wizard, shell completion subsystem, TUI/Web UI
- generic RPC/IDL/workflow framework 도입
- Plugin management CLI
- hidden current Bot/Project 같은 암묵 authority context
- Runtime 자동 시작 확대
- stale CAS 자동 mutation retry
- 새로운 Provider 제품 추가
- CLI가 Domain state, Authority, Receipt lifecycle을 소유하도록 하는 변경
- 검증용/일회성 branch의 코드를 “ahead commit이 있다”는 이유만으로 `main`에 병합
- binary evidence 없이 문서 또는 component PASS만으로 `CLI Complete`를 선언

기존 frozen operation을 실행 가능하게 만들기 위해 downstream owner 내부 구현이 부족한 경우 이는 신규 기능 추가가 아니라 **기존 contract의 executable closure**로 분류한다. 반대로 operation 의미 자체가 바뀌어야 한다면 구현을 중단하고 plan/ADR 재검토를 먼저 수행한다.

## 3. Baseline과 Repository Branch Audit

### 3.1 코드 분석 baseline

최초 closeout 분석의 코드 기준은 `d537e5273c7b1468f5c3606b65b8e1847737eabe` (`feat(runtime): implement grok adoption A1-A4`)다. 이후 finding은 항상 작업 시작 시점의 최신 `main`에서 다시 확인한다.

### 3.2 2026-08-29 branch audit baseline

branch audit는 `18abe4948ddde29dd31ca1f4e544fb5915604e91` (`docs: add CLI completion adversarial closeout plan`)을 기준으로 수행했다. 당시 `main` 외 원격 branch는 10개였다.

| Branch | Audit 판정 | 처리 원칙 |
|---|---|---|
| `adversarial-review-converged` | `main`의 조상 | merged branch로 정리 대상 |
| `adversarial-review-finalize` | `main`의 조상, PR #4 convergence commit | merged branch로 정리 대상 |
| `develop/m1a-storage-proof` | `main`의 조상 | merged branch로 정리 대상 |
| `docs/agents-md-review` | `main`의 조상 | merged branch로 정리 대상 |
| `docs/grok-adoption-plan-staging` | PR #5로 병합됨 | merged branch로 정리 대상 |
| `docs/v0-2-plan-review` | `main`의 조상 | merged branch로 정리 대상 |
| `adversarial-review-final` | history상 diverged, 해당 membership/control-client 변경은 이후 convergence commit에서 수렴 | 직접 merge 금지, superseded branch로 정리 대상 |
| `adversarial-review-validation` | PR #3 validation-only, 이후 PR #4 convergence가 제품 경로 소유 | 직접 merge 금지, superseded validation branch로 정리 대상 |
| `automation/v0-8-five-pass-import` | `.dxbot-import/*` + one-shot import workflow 전용 | 제품 branch가 아님, 직접 merge 금지 |
| `ci/m1a-validation-b0185d4` | runner availability 진단용, PR #2가 명시적으로 `Do not merge` | 직접 merge 금지 |

위 audit에서 **현재 `main`에 추가 병합해야 할 제품 변경 branch는 0개**로 판정했다. Git commit ancestry의 `ahead_by` 값과 제품 의미의 미병합 여부를 동일시하지 않는다. squash/convergence, one-shot staging, validation-only branch는 content/provenance를 확인한 뒤 판정한다.

### 3.3 Branch audit invariant

CLI closeout 중 branch 상태는 다음을 만족해야 한다.

- 실제 제품 변경이 `main`보다 앞선 branch는 반드시 owner/Acceptance 영향과 함께 분류한다.
- validation/import/runner diagnostic branch는 product code source로 병합하지 않는다.
- superseded branch의 과거 commit을 다시 병합해 convergence fix를 되돌리지 않는다.
- 동일 의미가 다른 history로 존재하면 tree/content와 최신 Canonical Owner를 기준으로 판단한다.
- `CLI Complete` 직전 `unclassified_ahead_branches = 0`이어야 한다.

## 4. Blocking Findings

### BF-CLI-001 — production entrypoint의 미연결 command

현재 `crates/cli/src/main.rs`는 다수 command를 `not_wired()`로 종료한다. 일부 first-use command도 payload projection 성공 후 실제 authenticated Runtime submission 대신 `RuntimeUnavailable`로 끝난다.

**위험:** component projection test가 PASS여도 사용자는 실제 operation을 만들 수 없다.

**종료 조건:** production binary path의 `not_wired()` 호출 0개. 지원하지 않는 command가 있다면 registry/owner contract와 함께 해결하며 false-success 또는 silent omission을 허용하지 않는다.

### BF-CLI-002 — hardcoded Instance/Principal

production entrypoint가 실제 discovery 결과 대신 기본 `InstanceId`/`PrincipalRef`를 직접 구성하면 multi-instance 선택, transport authentication, journal identity와 충돌한다.

**종료 조건:** selected Instance는 `DXB-RUN-035` precedence를 통과한 verified descriptor에서 나오고 authenticated Principal은 trusted local transport boundary에서만 유도한다. payload의 principal은 consistency binding일 뿐 authentication evidence가 아니다.

### BF-CLI-003 — Control transport 부재

`SubmissionClient`는 submission/replay identity와 fail-closed transport semantics를 갖지만 production CLI가 verified local Control endpoint에 연결되는 concrete path가 없다.

**종료 조건:** 실제 `dxb` subprocess가 authenticated local Control boundary를 통해 request/result 왕복을 수행한다. 새 generic RPC platform은 만들지 않는다.

### BF-CLI-004 — durable CLI journal과 submission fixture 분리

CLI에는 file-backed `LocalJournal`이 있으나 `control-client`의 `JournalStore`는 in-memory fixture다. production submission이 fixture store만 사용하면 process restart recovery와 local durable journal이 하나의 실행 경로로 연결되지 않는다.

**종료 조건:** production submission은 CLI가 소유한 durable journal semantics를 사용하고, in-memory store는 test fixture로만 남는다. 동일 state를 두 owner에 dual-write하지 않는다.

### BF-CLI-005 — command registry와 binary dispatch drift

63개 command registry는 `application-contract`에 존재하지만 binary entrypoint에 별도의 수작업 group/subcommand dispatch와 help 문자열이 존재한다.

**종료 조건:** command 존재 여부, typed input, wait/security/output metadata의 canonical source는 기존 registry다. CLI path resolution에 필요한 view는 해당 metadata에서 파생하며 두 번째 semantic registry를 만들지 않는다.

### BF-CLI-006 — downstream Owner 실행 coverage 부족

CLI가 payload를 만들 수 있어도 downstream owner가 해당 command를 실제 처리하지 않으면 binary는 완료되지 않는다.

**종료 조건:** 63개 command 각각에 대해 `Path → Parse → Select → Preflight → Dispatch → Owner → Result/Stream → Recovery → Render`의 적용 가능 경로가 존재한다.

### BF-CLI-007 — 기존 PASS evidence와 product completion의 수준 차이

기존 Acceptance `Passed`는 유지한다. 다만 component/projection evidence를 실제 CLI product completion과 동일시하지 않는다.

**종료 조건:** 아래 Completion Level의 L4 Binary와 L5 Journey evidence를 별도로 기록한다.

### BF-CLI-008 — parser-side identity helper의 production 오용 위험

`application-contract::CliInput`에는 local option에서 `InstanceId`와 `PrincipalRef`를 구성하는 convenience helper가 존재한다. profile은 Instance identity가 아니고 client-supplied principal은 authentication evidence가 아니다.

**종료 조건:** production Instance는 verified discovery에서, authenticated Principal은 trusted transport에서만 온다. convenience helper가 test/projection 용도라면 production orchestration에서 호출하지 않는다.

### BF-CLI-009 — version/compatibility representation drift

CLI discovery와 control-client가 protocol/schema version을 서로 다른 문자열 형태로 소유하면 local version 출력과 remote compatibility 판정이 어긋날 수 있다.

**종료 조건:** version/protocol/schema compatibility의 Canonical Owner를 하나로 유지하고 CLI/Control client가 동일 representation/source를 사용한다.

### BF-CLI-010 — global option parsing과 실제 동작 분리

production entrypoint가 `--profile`, `--instance`, `--color`, `--wait`, `--timeout` 값을 소비만 하고 execution/selection/rendering에 적용하지 않으면 사용자는 option이 적용됐다고 오판한다.

**종료 조건:** 등록된 global option은 계약대로 실제 동작하거나, 미구현 상태라면 성공처럼 무시하지 않고 fail closed한다.

### BF-CLI-011 — group/command help의 binary 계약 미폐쇄

계약은 `dxb <group> --help`, `dxb <command> --help`가 Runtime 없이 exit 0이어야 한다.

**종료 조건:** top/group/command help가 canonical metadata에서 생성되고 Runtime 접근 없이 exit 0을 반환한다.

### BF-CLI-012 — stale/diverged branch가 completion 상태를 왜곡할 위험

Git history상 `ahead_by > 0`인 branch라도 product code가 미병합됐다는 뜻은 아니다. convergence/squash branch, validation-only branch, import staging, CI diagnostic을 무차별 병합하면 최신 owner fix를 되돌리거나 임시 artifact를 제품 history에 넣을 수 있다.

**종료 조건:** 모든 non-main branch는 `merged | superseded | temporary-diagnostic | active-product-work` 중 하나로 분류한다. `active-product-work`만 merge 후보이며, merge 전 최신 `main` 기준 code/docs/test 관계를 재검수한다. CLI Complete 직전 미분류 branch는 0개다.

## 5. Completion Level

| Level | 의미 | 대표 evidence | CLI Complete 판정 |
|---|---|---|---|
| L0 Contract | registry/schema/document 관계 일치 | validator/golden | 불충분 |
| L1 Projection | `CliInput → CommandPayload` 정확 | unit/component | 불충분 |
| L2 Component | selector/journal/recovery/renderer 개별 invariant | crate tests | 불충분 |
| L3 Boundary | verified endpoint/authenticated Control/Host 경계 왕복 | integration | 불충분 |
| L4 Binary | 실제 build된 `dxb` subprocess가 명령 실행 | subprocess tests | 필수 |
| L5 Journey | 정상·실패·복구 사용자 여정을 binary로 종료 | E2E/fault | **최종 완료 조건** |

기존 `DXB-DEL-061` 상태를 소급 변경하지 않는다. 이 문서는 CLI 제품 완료에 필요한 추가 종료 Gate를 소유한다.

## 6. Canonical Execution Spine

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

금지되는 우회:

```text
CLI → direct Domain mutation
CLI → fabricated success payload
CLI → unverified endpoint
CLI → client-supplied principal을 authentication으로 신뢰
CLI → in-memory-only production journal
CLI → provider-specific branch
CLI → stale validation branch를 product source로 재병합
```

## 7. 실행 단계

### C0 — Evidence/Repository Baseline 확정

**목적:** component PASS와 제품 완료를 분리하고 작업 시작점의 branch/provenance를 고정한다.

#### 작업

- CLI 관련 기존 Acceptance를 L0~L5 중 현재 evidence level로 분류한다.
- 실제 binary subprocess를 실행하지 않는 test는 L4/L5로 승격하지 않는다.
- 최신 `main` HEAD와 non-main branch를 audit한다.
- branch는 `merged | superseded | temporary-diagnostic | active-product-work`로 분류한다.
- `active-product-work`가 있으면 code/docs/test/owner 관계를 검토한 뒤에만 병합한다.

#### Gate

- CLI 관련 Acceptance의 evidence level이 명확하다.
- unclassified non-main branch = 0.
- active product branch의 미처리 변경 = 0 또는 명시적 blocker.

### C1 — Parser/Help/Registry 단일화

**목적:** 수작업 parser/help와 frozen command registry drift 제거.

#### 작업

- 63개 command key와 user-facing path의 exact coverage를 자동 검증한다.
- `dxb`, group help, command help가 Runtime 없이 동작하는지 검증한다.
- required/optional/local/source/CAS/wait 정보가 canonical metadata와 일치하게 한다.
- `main.rs`는 path resolution과 orchestration만 담당하고 command semantics를 다시 소유하지 않는다.

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
- 플랫폼 지원 범위의 pathological argv

#### Gate

- registry 63개 중 binary parser 접근 불가 = 0
- registry에 없는 success path = 0
- help/runtime dependency = 0

### C2 — Instance Selection과 Authenticated Principal 수렴

**목적:** hardcoded identity와 client-authentication 혼동 제거.

#### 작업

- `--instance → profile binding → exactly-one verified endpoint` precedence를 production entrypoint에 연결한다.
- selected descriptor의 InstanceId/endpoint/host generation을 execution context로 고정한다.
- authenticated Principal은 local transport가 제공하고 payload binding과 일치 여부만 검사한다.
- stale descriptor, generation mismatch, unknown profile, ambiguity를 fail closed한다.

#### Gate

- production hardcoded Instance = 0
- production hardcoded authenticated Principal = 0
- silent fallback = 0
- stale generation acceptance = 0

### C3 — Control/Host/Query/Stream Production Boundary 연결

**목적:** `not_wired`를 실제 owner 경로로 대체한다.

Command kind를 하나의 mutation transport에 억지로 합치지 않는다.

| Kind | Production boundary |
|---|---|
| `C` | durable submission → authenticated Control → Application |
| `Q` | bounded authenticated query |
| `S` | subscription/cursor stream |
| `H` | Runtime Host control |
| `H/Q` | Host/query diagnostic |

#### 제약

- 새 generic RPC/IDL 금지
- endpoint/authentication은 Runtime owner 계약 재사용
- response identity 검증 유지
- provider-specific route를 CLI/Application에 추가하지 않음

#### Gate

- production `not_wired` = 0
- fake success transport = 0
- kind별 owner 없는 dispatch = 0

### C4 — Durable Journal / Submission Convergence

**목적:** 파일 journal과 submission/replay protocol을 하나의 production 흐름으로 연결한다.

책임은 다음처럼 유지한다.

- local durable journal 파일/retention/lock/hash-chain 의미: CLI
- submission/replay identity protocol: control-client
- canonical operation/result: Runtime/Application

#### 구현 제약

- `JournalStore`는 test fixture로 유지한다.
- production submission은 durable journal adapter를 사용한다.
- exact Rust trait/callback 형태는 private implementation detail이다.
- 동일 state dual-write 금지.
- `Prepared` 이후 CommandId/OperationId/IdempotencyKey/RequestDigest 재생성 금지.

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

#### Gate

- production in-memory-only journal = 0
- dual canonical journal = 0
- response loss 후 duplicate mutation = 0
- crash/restart 후 operation identity drift = 0

### C5 — 63 Operation Executability Closure

각 command는 아래 적용 가능 column을 evidence로 채운다.

| Column | 판정 |
|---|---|
| Path | user-facing path → registry key |
| Parse | typed input validation |
| Select | Instance/authentication context |
| Preflight | selector/CAS materialization |
| Dispatch | kind별 production boundary |
| Owner | Application/Runtime/Provider owner 실제 처리 |
| Result | output schema/exit semantics |
| Recovery | mutation/stream recovery |
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
- unsupported path success rendering = 0

### C6 — Rendering / Automation Closure

검증 모드:

```text
human × TTY
human × non-TTY
json × TTY
json × non-TTY
jsonl × TTY
jsonl × non-TTY
```

failure matrix:

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

각 case에서 `exit code`, `stdout schema`, `stderr contamination`, `typed next action`, `secret redaction`, `operation continuity`를 함께 검증한다.

#### Gate

- machine stdout human contamination = 0
- schema-bearing stderr payload = 0
- nonzero exit에서 unparsable machine error = 0
- local interrupt implicit Runtime cancel = 0

### C7 — UJ-001~UJ-010 Binary Journey 승격

새 journey를 만들지 않고 [DXB-IFC-043](../40-interfaces/43-cli-user-journeys.md)의 기존 UJ-001~UJ-010을 실제 subprocess E2E로 승격한다.

최소 first-use chain:

```text
dxb runtime start
dxb bot create alpha
dxb conversation send alpha --stdin
dxb task submit --owner alpha --stdin
dxb task show <selector>
dxb task result <selector>
```

| Journey | 최소 Binary evidence |
|---|---|
| UJ-001 | fresh state help/version/runtime start/selection |
| UJ-002 | bot create → Main Conversation send |
| UJ-003 | selector mutation + stale CAS conflict + safe guidance |
| UJ-004 | non-TTY json/jsonl automation |
| UJ-005 | approval-required → show → approve/deny |
| UJ-006 | provider unavailable → doctor provider |
| UJ-007 | ambiguity → visible exact candidate 선택 |
| UJ-008 | process crash/interrupt → journal scan → original operation recovery |
| UJ-009 | pagination/`--all`/safe output/partial resume |
| UJ-010 | multi-instance exact selection + compatibility |

#### Gate

- UJ L5 PASS = 10/10
- journey 중 fixture-only backdoor = 0

### C8 — Adversarial Fault/Concurrency Review

sleep 길이에 의존하는 race test를 금지하고 barrier/failpoint/fake clock/process crash를 우선한다.

| 공격 | 기대 invariant |
|---|---|
| 두 CLI가 동일 command를 동시에 recover/replay | 하나의 operation identity |
| Dispatching fsync 직후 kill | restart 후 binding lookup |
| server commit 후 response loss | duplicate mutation 없음 |
| corrupt middle journal record | fail closed |
| truncated final journal record | 마지막 완전 record까지만 허용 |
| Runtime generation 교체 | stale generation fencing |
| stale CAS | 자동 retry 금지 |
| local timeout | observation만 종료, continuity 표시 |
| SIGINT during wait | implicit cancel 없음 |
| stream cursor future/gap | explicit resync |
| huge page/`--all` | item/byte ceiling |
| huge stdin/input-file | Prepared 전 bounded materialization |
| malicious output path/symlink | overwrite/follow 금지 |
| disk full during journal append | network send 전 fail closed |
| auth principal mismatch | owner mutation 전 거부 |
| provider connection reset | typed provider failure |
| shutdown under load | new admission/drain 경계 보존 |

#### Gate

- probabilistic-only race evidence = 0
- unbounded queue/buffer/scan = 0
- unknown side effect를 transient retry로 숨김 = 0

### C9 — Removal / Provenance Review와 Final Hardening

완료 직전 임시 구조와 stale provenance를 제거한다.

검사 대상:

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
unclassified ahead branch
superseded validation/import branch mistaken as product source
```

`CoreCommands` 등 convenience projection이 canonical parser/executor와 의미를 중복하면 축소 또는 제거한다. test 편의를 위해 남길 경우 production policy source가 아님을 검증한다.

#### Gate

- 위 항목 모두 0 또는 명시적 test-only/temporary provenance 근거 존재
- active product branch 미병합 변경 = 0
- branch audit 재실행 PASS

## 8. 구현 Wave

```text
Wave 0 — baseline
C0 evidence + branch/provenance audit

Wave 1 — execution spine
C2 Instance/Auth
  ↓
C3 Control/Host/Query/Stream boundary
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
C9 removal/provenance review
  ↓
Final two rechecks
```

C3/C4가 닫히기 전에 command별 wrapper를 대량 추가하지 않는다. execution spine 확정 전에 63개 dispatch wrapper를 먼저 만들면 transport/recovery 중복이 확산될 위험이 크다.

## 9. 예상 영향 범위

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

새 crate는 기본안이 아니다. 기존 Canonical Owner에서 닫을 수 없다는 증거가 생길 때만 별도 ADR/plan 변경을 요구한다.

## 10. Test Architecture

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

Binary harness는 temporary state root, deterministic Instance, authenticated local endpoint, Runtime host lifecycle, fake/reference Provider, failpoint/crash point, stdout/stderr capture, TTY/non-TTY, process kill/restart, persisted journal/runtime state를 격리할 수 있어야 한다.

harness가 production 경계를 우회해 Domain API를 직접 호출하면 해당 test는 L4/L5 evidence가 아니다.

## 11. Completion Metrics

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

unclassified_ahead_branches = 0
active_product_branches_with_unmerged_changes = 0
```

테스트 개수 자체는 완료 지표가 아니다. invariant/path/provenance coverage가 완료 지표다.

## 12. Verification Commands

구현 환경에서 다음을 모두 수행한다.

```bash
cargo fmt --check
cargo check --workspace
cargo clippy --workspace
cargo test --workspace

python3 scripts/plan-validator.py --active
python3 scripts/plan-validator.py --self-test
```

추가 closeout suite는 최소 다음 목적을 독립적으로 식별 가능해야 한다.

```text
cli-binary-contract
cli-binary-journeys
cli-binary-recovery
cli-binary-automation
cli-binary-fault
```

정확한 Cargo test target 이름은 기존 fixture layout에 맞춰 구현하며 public contract로 고정하지 않는다.

## 13. Final Recheck 1 — Structural / Consistency

다음을 전부 검토한다.

1. 63 command registry와 binary path 1:1 coverage
2. duplicate parser/help/wait/security/output policy
3. CLI/Control/Application/Runtime Canonical Owner 중복
4. CLI journal과 submission fixture state 중복
5. crate dependency direction과 cycle
6. public schema의 불필요한 확대
7. docs/registry/golden/test 관계
8. unused feature/dependency/config
9. lowercase kebab-case / Rust snake_case
10. 기존 v0.8.10 비범위 위반
11. non-main branch 분류와 provenance, superseded/temporary branch의 재병합 위험

발견 사항을 수정한 후 동일 범위를 다시 검수한다.

## 14. Final Recheck 2 — Cross-Layer Executability

실제 binary evidence로 다음 흐름을 추적한다.

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

각 적용 가능 command에 runtime/provider unavailable, permission/approval, cancel/timeout, crash/restart, duplicate/partial success, stale revision/generation, journal corruption, resource pressure, shutdown/drain을 대입한다.

발견 사항 수정 후 해당 binary journey와 workspace regression을 다시 수행한다.

## 15. CLI Complete 최종 Gate

아래 조건을 하나라도 만족하지 못하면 `CLI Complete`를 선언하지 않는다.

### Contract / Surface

- 63/63 operation path verified
- command/input/golden registry drift 없음
- 신규 command 0

### Production Wiring

- verified Instance selection
- trusted authenticated Principal
- authenticated local transport
- durable journal/submission convergence
- production `not_wired` 0

### Correctness / Recovery

- Prepared/Dispatching/Observed/Terminal crash window 검증
- response-loss duplicate 방지
- original replay identity 보존
- stale generation/CAS fail closed
- corrupt/unknown journal auto-replay 금지

### UX / Automation

- UJ-001~UJ-010 binary PASS
- human/json/jsonl contract PASS
- non-TTY prompt 없음
- machine stdout/stderr 분리
- typed next action 유지
- safe output/partial resume PASS

### Repository Provenance

- all non-main branches classified
- unclassified ahead branch = 0
- active product branch의 미병합 변경 = 0
- validation/import/CI diagnostic branch를 product source로 재병합하지 않음

### Quality

- workspace fmt/check/clippy/test PASS
- active plan validator/self-test PASS
- Final Recheck 1 PASS
- Final Recheck 2 PASS
- 발견 사항 수정 후 재검증 PASS

## 16. 중단 조건

다음 상황이 발생하면 구현을 중단하고 plan/ADR 재검토를 먼저 수행한다.

1. CLI 완료를 위해 신규 command/public workflow가 필요함
2. generic RPC/IDL/framework가 선행 조건으로 변함
3. CLI가 Authority/Domain canonical state를 소유해야 한다는 설계가 등장함
4. durable journal을 두 owner가 동시에 canonical로 유지하려 함
5. Provider-specific branch가 CLI/Application에 유출됨
6. recovery를 위해 새로운 operation identity를 만들어야 한다고 판단함
7. 63-operation contract 자체 의미 변경이 필요함
8. binary journey를 통과시키기 위해 fixture-only bypass가 production path에 들어감
9. stale validation/import branch를 병합해야만 최신 기능을 얻을 수 있다고 판단됨 — 먼저 convergence/provenance를 재검증한다.

## 17. 작업 단위와 Evidence

각 change set은 가능한 한 다음 단위를 유지한다.

```text
blocking finding
+ owner 확인
+ 최소 production wiring
+ deterministic component test
+ binary/integration evidence
+ failure/recovery evidence
+ 관련 문서 갱신
+ branch/provenance impact
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
- branch/provenance impact
- remaining blockers
- Recheck 1 result
- Recheck 2 result
```

## 18. 최종 판정 원칙

CLI 개발 종료는 “파일이 존재한다”, “63 command가 registry에 있다”, “component test가 많다”, “workspace test가 한 번 통과했다”, “branch가 main보다 ahead로 보인다” 같은 단일 신호로 판정하지 않는다.

최종 판정은 오직 다음 명제로 한다.

> **기존 v0.8.10 P0 surface를 확장하지 않고, 실제 `dxb` binary가 모든 등록 operation을 올바른 Canonical Owner까지 전달하며, 실패·중단·재시작·부분 성공에서도 identity와 recovery semantics를 보존하고, 10개 사용자 여정을 human/machine interface로 재현 가능하며, repository provenance에도 미분류 제품 변경이 남아 있지 않다.**

이 명제가 executable evidence로 증명되면 CLI closeout을 완료한다.

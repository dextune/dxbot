---
title: "CLI 개발 완료 적대적 종료 플랜"
document_id: "DXB-DEL-066"
version: "0.8.10"
status: "Accepted"
normative: false
priority: "P0"
last_updated: "2026-08-29"
depends_on: ["DXB-DEL-060", "DXB-DEL-061", "DXB-IFC-041", "DXB-IFC-042", "DXB-IFC-043", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-035", "DXB-RUN-038"]
---
# CLI 개발 완료 적대적 종료 플랜

## 1. 목적과 현재 판정

이 문서는 DXBOT v0.8.10 CLI를 **기능 확장 없이 실제 사용자 관점의 완료 상태로 수렴**시키기 위한 closeout 계획과 실행 기록이다.

CLI 완료는 parser/unit/component PASS가 아니라 실제 `dxb` binary가 다음 흐름을 끝까지 만족할 때만 선언한다.

`argv → typed input → verified Instance selection → authenticated LocalPrincipal → bounded preflight → canonical owner → durable result/stream → crash/restart recovery → human/machine render`

2026-08-29 현재 구현 브랜치의 구조 검수 기준선은 `codex/cli-completion-finalize`의 `c6d9b04953d83e422bc7cdafc980d5300ce95efc`이며, 이 문서 갱신 이후 최종 검수에서 다시 head를 고정한다.

**현재 판정은 `CLI Complete 아님`이다.** 다수의 production wiring과 correctness blocker는 폐쇄됐지만, 아래 Open Blocker와 executable evidence gate가 남아 있다. 특히 실행 가능한 `fmt/clippy/test/subprocess` 증거 없이 문서 또는 component test 존재만으로 완료를 선언하거나 `main`에 병합하지 않는다.

## 2. Scope Lock

### 2.1 허용 범위

closeout에서 허용하는 변경은 다음과 같다.

1. frozen 63개 P0 command의 기존 의미를 production binary에서 실행 가능하게 연결한다.
2. parser/help/preflight/Control/Host client/renderer/journal/recovery의 미연결 경로를 폐쇄한다.
3. 기존 Canonical Owner가 정의한 operation을 Application/Runtime/Security/Provider owner까지 전달한다.
4. false-success, identity drift, silent option ignore, output contamination, recovery ambiguity, stale CAS bypass를 수정한다.
5. component evidence를 boundary/binary/journey/fault evidence로 승격한다.
6. 기존 owner 계약을 실행 가능하게 만드는 최소 private adapter, persistence, dependency wiring을 추가한다.
7. 발견된 코드↔문서 불일치를 같은 change set에서 갱신한다.

### 2.2 금지 범위

다음은 closeout에서 새 의미로 만들지 않는다.

- 신규 public CLI command/group
- frozen 63-operation registry 밖의 신규 public operation
- generic RPC/IDL/workflow platform
- CLI-owned Domain/Authority/Receipt/Provider state
- BotId를 `PrincipalRef`로 승격하는 권한 편법
- stale CAS 자동 retry
- Provider 제품 신규 추가
- 승인 정책을 CLI가 임의로 정의하거나 고위험 operation을 임의 분류
- binary evidence 없이 `CLI Complete` 선언
- validation/diagnostic branch를 제품 변경으로 무차별 merge

기존 operation의 downstream owner 구현 부족이 단순 wiring이 아니라 **operation 의미·정책 생성 규칙을 새로 정의해야 하는 문제**라면 closeout 구현으로 추측하지 않고 Canonical Owner blocker로 남긴다.

## 3. Canonical Owner Lock

| 의미 | Canonical Owner | CLI closeout 원칙 |
|---|---|---|
| command inventory, typed input, wait/output contract | `DXB-IFC-041`, `DXB-IFC-042` + `application-contract` registry | 두 번째 command registry 금지 |
| user journey / failure UX | `DXB-IFC-043` | machine stdout, typed next action 보존 |
| Domain mutation/query/receipt binding | `application` | CLI state 복제 금지 |
| LocalPrincipal/Authority/Approval | `runtime-security`, `DXB-RUN-032` | BotId ≠ Principal, deny unknown |
| crash/replay identity | `DXB-RUN-033`, Application binding, CLI durable journal | blind replay 금지 |
| Instance/bootstrap/discovery | `runtime-bootstrap`, Runtime Host, `DXB-RUN-035` | CLI가 Instance/Principal fabricating 금지 |
| Provider registration/readiness | `provider-host` | readiness 판단을 CLI에 복제하지 않음 |
| Process durable state | `application`, `DXB-RUN-038` | CLI show/watch projection만 소유 |
| safe local export | CLI `SafeWriter` | no-follow/no-replace/bounded |

## 4. Completion Level

| Level | 의미 | 대표 evidence | 완료 판정 |
|---|---|---|---|
| L0 Contract | registry/schema/docs 정합 | validator/golden | 불충분 |
| L1 Projection | `CliInput → CommandPayload` 정확 | unit | 불충분 |
| L2 Component | selector/journal/security/persistence/renderer invariant | crate tests | 불충분 |
| L3 Boundary | authenticated Control/Host/owner 왕복 | integration | 불충분 |
| L4 Binary | 실제 build된 `dxb` subprocess | subprocess | **필수** |
| L5 Journey | 정상·실패·재시작·복구 여정 | E2E/fault | **최종 필수** |

기존 `DXB-DEL-061`의 component Acceptance 결과는 소급 무효화하지 않는다. 단, 그 PASS를 L4/L5 완료 증거로 승격해서 해석하지 않는다.

## 5. Adversarial Findings Ledger

상태는 `Resolved | Partial | Open Blocker | Evidence Pending`만 사용한다.

| ID | Finding | 상태 | 폐쇄/잔여 조건 |
|---|---|---|---|
| BF-CLI-001 | production binary command dispatch 미연결 | Resolved | registry-driven C/Q/H/S dispatch와 help path 연결 |
| BF-CLI-002 | parser-side Instance/Principal을 production authority로 오용 | Resolved | verified discovery + transport-derived LocalPrincipal로 override |
| BF-CLI-003 | concrete authenticated Control transport 부재 | Resolved | owner-only Unix socket, handshake, peer UID, generation fence 연결 |
| BF-CLI-004 | durable CLI journal과 production submit/recovery 분리 | Resolved | production submission이 durable local journal과 binding lookup recovery 사용 |
| BF-CLI-005 | registry와 binary dispatch/help drift | Resolved | 63-row registry에서 path/typed/wait/help metadata 파생 |
| BF-CLI-006 | downstream owner coverage 부족 | Partial | 다수 owner 연결 완료, Approval policy continuation/Process producer/bootstrap defaults는 아래 blocker |
| BF-CLI-007 | component PASS를 product completion으로 오판 | Resolved by gate | L4/L5 별도 증거 필수화 |
| BF-CLI-008 | parser identity helper production 오용 | Resolved | `ExecutionContext`가 trusted identity를 덮어씀 |
| BF-CLI-009 | protocol/schema version drift | Resolved | `LOCAL_CONTROL_*` 단일 source, 새 watch/host-stop wire는 schema `v2` |
| BF-CLI-010 | global/local option이 parse만 되고 무시 | Partial | `--all/--output/--color/--wait/--timeout` production 적용, 세부 command-local 필드 잔여 검수 필요 |
| BF-CLI-011 | group/command help Runtime 의존 | Resolved | registry 기반 offline help exit 0 |
| BF-CLI-012 | stale/diverged branch completion 왜곡 | Evidence Pending | final branch audit에서 unclassified ahead branch 0 확인 |
| BF-CLI-013 | `@local` field가 wire/RequestDigest로 누출 | Resolved | shared typed-field DSL parser로 wire projection에서 제거 |
| BF-CLI-014 | `runtime stop`이 receipt만 성공하고 실제 host는 계속 실행 | Resolved | graceful commit 후 server shutdown; host-stop은 generation fenced |
| BF-CLI-015 | Approval/Authority state가 in-memory라 restart 후 소실 | Resolved | durable `SecurityStateStore` + Runtime Host composition |
| BF-CLI-016 | Membership row와 AuthorityBinding이 원자적이지 않고 BotId를 Principal로 오용할 위험 | Resolved | membership-subject binding, durable coordination marker, Bot/Principal 분리 |
| BF-CLI-017 | Membership remove/re-add generation ABA | Resolved | inactive tombstone, App/Security generation 1→2→3, stale generation 거부 |
| BF-CLI-018 | thread/task/memory/operation CAS가 preflight와 commit에서 불일치 | Resolved | source/scope/target-scope/receipt CAS 실제 commit 검증 + server materialized-CAS gate |
| BF-CLI-019 | custom client가 preflight를 건너뛰어 필수 CAS 생략 가능 | Resolved | ControlServer가 `validate_materialized_cas` 재검증 |
| BF-CLI-020 | exact retry가 changed RequestDigest/payload를 기존 commit으로 오인 | Resolved/Recovery recheck | normal request는 exact identity 재대조; crash-marker recovery도 동일 검증으로 최종 고정 필요 |
| BF-CLI-021 | S stream이 buffered output 또는 비단조 cursor 사용 | Resolved | process entrypoint 직접 flush, fixed-width opaque revision cursor, explicit gap/resync |
| BF-CLI-022 | machine JSON/JSONL에 ANSI/progress 오염 가능 | Resolved | color는 human only, machine stdout schema payload only |
| BF-CLI-023 | parser가 named option 값을 positional로 재처리하고 primary/secondary selector를 혼동 | Resolved | argv cursor 단일 소비 + shared typed-field primary-selector metadata |
| BF-CLI-024 | Provider 미구성인데 `bot activate`/`task submit` false-success | Resolved | Control owner에서 Ready `llm-chat` admission fail-closed |
| BF-CLI-025 | failure가 machine-actionable next action을 제공하지 않음 | Resolved/Partial | 공통 renderer가 Runtime/Provider/Recovery/Incompatible/Partial action 보강; owner-specific action 계속 우선 |
| BF-CLI-026 | Runtime bootstrap이 `DXB-RUN-035`의 default policy generations/owner binding/DataSchemaVersion을 원자적으로 만들지 않음 | **Open Blocker** | Runtime/bootstrap Canonical Owner 구현 필요 |
| BF-CLI-027 | Approval decision 이후 원 high-risk operation 재평가/continuation owner 부재 | **Open Blocker** | policy owner가 pending operation 생성·park·re-evaluate를 소유해야 함 |
| BF-CLI-028 | Process show/watch는 있으나 production Process 생성/transition owner가 없음 | **Open Blocker** | `DXB-RUN-038` owner가 실제 Process aggregate를 생성/갱신해야 함 |
| BF-CLI-029 | frozen semantic field 일부가 canonical state/policy owner와 연결되지 않음 | **Open Blocker** | §8 semantic field audit 참조 |
| BF-CLI-030 | `Cargo.lock`이 현재 crate dependency와 동기화되지 않음 | **Evidence Pending** | Cargo로 lockfile regenerate 후 diff 검증 |
| BF-CLI-031 | 실행 가능한 Rust quality/binary evidence 미확보 | **Evidence Pending** | fmt/clippy/test/subprocess/fault 실제 PASS 필요 |

## 6. 구현 폐쇄 상태

### 6.1 Input / Contract

완료된 항목:

- user-facing command path와 63 command metadata를 frozen registry에서 파생한다.
- command typed-field DSL parser를 공유해 invocation/primary-selector/wire-local stripping이 동일 source를 사용한다.
- named value는 정확히 한 번 소비한다.
- primary selector와 secondary selector-typed field를 구분한다.
- `True`, `Bool`, `PageSize`, numeric primitive와 duplicate/unknown field를 Runtime 접속 전에 fail closed한다.
- `@local`은 `CliInput`에만 존재하고 `CommandPayload.semantic_options`, `RequestDigest`, server-owned state에 들어가지 않는다.
- mutation 필수 CAS는 ControlServer에서도 재검증한다.

### 6.2 Discovery / Authentication / Transport

완료된 항목:

- production Instance는 verified discovery 결과에서만 선택한다.
- authenticated Principal은 Unix peer credential과 Instance로 server-side derivation한다.
- client-supplied Principal은 consistency binding일 뿐 authority evidence가 아니다.
- socket type/owner/mode, InstanceId, HostGeneration, protocol/schema를 handshake에서 검증한다.
- local control frame은 bounded size를 가진다.
- watch/host-stop wire 추가에 맞춰 schema version을 `v2`로 갱신했다.

현재 portability 제한:

- safe peer credential 구현은 Linux/Android 경로가 production-ready이며 다른 Unix target은 현재 fail-closed `Unsupported`다. 지원 플랫폼 범위가 확대되기 전에는 silent downgrade하지 않는다.

### 6.3 Submission / Identity / Recovery

완료된 항목:

- `Prepared → Dispatching → Observed/Terminal` durable journal 경계를 production submission에 연결했다.
- CommandId/OperationId/IdempotencyKey/RequestDigest/InstanceId를 replay identity로 보존한다.
- binding lookup을 통해 crash ambiguity를 수렴한다.
- normal exact retry는 기존 result의 operation/instance/receipt/request digest를 다시 검증한다.
- unknown/partial binding을 성공으로 재구성하지 않는다.
- local timeout/SIGINT/broken pipe는 관측 종료일 뿐 Runtime operation cancel 의미가 아니다.

최종 검수 잔여:

- cross-owner Security recovery marker의 committed binding 판정도 normal exact-retry validator와 동일 검증을 사용해야 한다.

### 6.4 Application / Security Cross-Owner UoW

완료된 항목:

- Application membership row와 Security membership-subject AuthorityBinding을 별도 Canonical Owner에 유지한다.
- BotId를 PrincipalRef로 변환하지 않는다.
- `prepare marker → Application durable commit → Security durable publish → marker clear` 순서로 crash window를 폐쇄한다.
- restart 시 marker를 읽고 Application binding 존재 여부에 따라 Security delta apply/discard를 결정한다.
- membership remove는 row를 삭제하지 않고 inactive generation tombstone을 남긴다.
- Security revocation도 generation을 한 번 증가시키며 동일 delta replay는 idempotent하다.
- Approval record는 pending OperationId + action + target + policy generation + revision을 보유한다.
- Approval decision은 revision CAS, approver 검증, durable wakeup, durable AuditIntent를 가진다.

### 6.5 Provider Admission

완료된 항목:

- Provider query는 `provider-host`의 동일 registration state를 읽는다.
- default unconfigured Runtime에서 `bot activate`와 `task submit`은 `ProviderUnavailable`로 fail closed한다.
- explicit Ready `llm-chat` Provider가 있을 때만 admission을 통과한다.
- exact retry는 admission보다 먼저 canonical committed result를 반환하므로 Provider 장애가 과거 commit 의미를 변경하지 않는다.

남은 owner 문제:

- `DXB-RUN-035`가 요구하는 default Provider policy generation과 실제 provider configuration projection은 bootstrap/config Canonical Owner에서 닫혀야 한다.

### 6.6 Query / Pagination / Export

완료된 항목:

- server page size는 bounded이며 oversized 값을 clamp하지 않고 거부한다.
- `--all`은 page/item/byte ceiling과 cursor progress invariant를 가진다.
- partial ceiling 도달은 resume cursor를 포함한 partial/resync error로 종료한다.
- `--output`은 `SafeWriter`의 no-follow/no-replace/fsync publication만 사용한다.
- inactive Membership tombstone은 list projection에서 숨기되 preflight CAS에는 보존한다.

### 6.7 Stream

완료된 항목:

- S-kind command는 production `run_process`에서 output 전체를 메모리에 모으지 않고 event마다 write+flush한다.
- `--format json`은 unbounded stream에 허용하지 않고 `human|jsonl`만 허용한다.
- Task/Process watch cursor는 canonical revision 기반 fixed-width opaque token이다.
- legacy decimal cursor는 입력 호환을 유지한다.
- cursor gap은 explicit resync 오류이며 silent event loss를 성공으로 보지 않는다.
- restart 후 watcher state가 없어도 canonical Task/Process revision에서 resync할 수 있다.

`application::subscription`의 retained in-memory registry는 기존 component acceptance fixture/API이며 production CLI watch continuity의 Canonical Owner로 사용하지 않는다. production continuity를 위해 별도 subscriber state를 이 fixture와 이중 저장하지 않는다.

### 6.8 Runtime Stop

완료된 항목:

- graceful stop은 Application commit receipt를 만든 뒤 Runtime Host serve loop에 shutdown을 반영한다.
- host-stop은 authenticated endpoint owner와 exact HostGeneration을 요구한다.
- response loss가 발생하더라도 committed graceful shutdown 의도를 되돌리지 않는다.

## 7. Frozen 63 Command Route Matrix

`route`는 production dispatcher가 command를 어디로 보내는지를 뜻한다. `owner-blocked`는 public command가 없다는 뜻이 아니라 downstream Canonical Owner의 실제 제품 의미가 아직 완결되지 않았다는 뜻이다.

### 7.1 Host / Host-Query

| Command | Kind | Route | 상태 |
|---|---:|---|---|
| runtime-start | H | Runtime Host bootstrap/spawn | Partial — BF-CLI-026 |
| runtime-status | H/Q | discovery + authenticated handshake | Wired |
| runtime-stop-graceful | C | Control→Application→Host shutdown | Wired |
| runtime-stop-host | H | LocalControl HostGeneration action | Wired |
| runtime-doctor | H/Q | discovery/control/provider diagnostic | Partial — section/page semantics 재검수 |
| version | Q | offline contract constants | Wired |

### 7.2 Bot / Conversation / Thread

| Command family | Route | 상태 |
|---|---|---|
| bot create/list/show | Application C/Q | Wired; bot policy-default semantics는 BF-CLI-026/029 |
| bot activate/deactivate/archive/restore | Control admission + Application | Wired; activate Provider fail-closed |
| conversation show/send/history | Application | Wired |
| thread create/list/show/send/history/branch | Application | Wired; source/conversation revision CAS enforced |

### 7.3 Task / Process

| Command family | Route | 상태 |
|---|---|---|
| task submit | Control Provider admission→Application | Partial — optional delegation/deadline/budget owner semantics BF-CLI-029 |
| task list/show/result | Application Q | Partial — artifact-specific result semantics BF-CLI-029 |
| task watch | Control S→Application canonical revision watch | Wired |
| task cancel/suspend/resume/redirect | Application C | Partial — reason/supervision semantics BF-CLI-029 |
| process show | Application Q | Owner-blocked producer — BF-CLI-028 |
| process watch | Control S→Application | Owner-blocked producer — BF-CLI-028 |

### 7.4 Project / Channel / Membership

| Command family | Route | 상태 |
|---|---|---|
| project create/list/show/archive/restore | Application, create additionally Security UoW | Wired |
| project member list/set/remove | Application + Security UoW | Wired, generation tombstone enforced |
| channel create/list/show/history/send | Application | Wired |
| channel member list/set/remove | Application + Security UoW | Wired, generation tombstone enforced |

### 7.5 Memory / Approval / Provider / Recovery

| Command family | Route | 상태 |
|---|---|---|
| memory get/search/history/propose | Application | Partial — optional scope semantics BF-CLI-029 |
| memory promote | Application | Partial — target-scope CAS wired, declassification semantics BF-CLI-029 |
| approval list/show | Security Q | Wired |
| approval approve/deny | Application operation binding + Security durable delta | Partial — decision durable, original operation continuation BF-CLI-027 |
| provider list/show | Provider Host Q | Wired |
| operation show/reconcile | Application Q/C | Wired, receipt revision CAS enforced |
| side-effect reconcile | Application C | Partial — operation-selector linkage owner data BF-CLI-029 |

## 8. Semantic Field Audit — Silent Ignore 금지

다음 frozen field는 parser가 허용하지만 현재 제품 owner 의미가 완전하지 않다. 이 표의 항목은 **성공처럼 무시된 채 CLI Complete가 될 수 없다.** owner 구현을 연결하거나 해당 operation이 그 의미를 이미 다른 canonical state에서 충족한다는 executable evidence가 필요하다.

| Command / field | 현재 상태 | 필요한 Canonical closure |
|---|---|---|
| bot-create `brain_policy`, `permission_policy`, `resource_policy`, `provider_policy` | parsed/wire, Domain BotState 미보유 | `DXB-RUN-035` default policy generations 및 Bot policy binding owner |
| task-submit `delegate_to_bot`, `requested_sender_bot` | parsed/wire, Task aggregate 미반영 | delegation/authority/sender policy owner |
| task-submit `deadline`, `budget` | parsed/wire, TaskState 미반영 | Task execution aggregate budget/deadline semantics |
| task-cancel/suspend `reason` | parsed/wire, durable directive reason 미보유 | supervision/directive audit semantics |
| task-result `artifact_id` | parsed/wire, result projection 미선택 | result/artifact canonical reference owner |
| memory-get optional `scope` | parsed/wire | memory selector scope disambiguation/validation |
| memory-promote `declassification_ref` | parsed/wire | information-label/declassification owner |
| approval-deny `reason` | parsed/wire | durable approval decision/audit reason semantics |
| runtime-doctor `section`, `page_size`, `cursor` | parsed local/query | diagnostic section/page contract 완결 |
| side-effect selector by Operation | parser 계약 존재, state linkage 부족 | SideEffect aggregate의 Operation linkage/lookup owner |

이 표를 해결하지 않고 단순히 field를 wire에서 제거하거나 성공으로 무시하는 것은 금지한다.

## 9. Open Canonical-Owner Blockers

### 9.1 Runtime bootstrap defaults — BF-CLI-026

`DXB-RUN-035` first-init atomicity는 최소한 다음을 요구한다.

- new persistent `InstanceId`
- initial `HostGeneration`
- LocalPrincipal / owner AuthorityBinding
- default Brain/Permission/Resource/Provider policy generations
- DataSchemaVersion
- publish-before-bind 금지 및 restart continuity

현재 Runtime bootstrap은 Instance/Host/discovery 중심으로 연결되어 있으나 위 default policy/security/data-version package 전체를 생성하지 않는다. CLI가 자체 default policy를 발명하면 Owner 중복이므로 Runtime/bootstrap owner에서 먼저 닫아야 한다.

### 9.2 Approval parking / continuation — BF-CLI-027

Approval record/decision/persistence/wakeup은 연결됐지만, full journey는 다음을 추가로 요구한다.

1. high-risk operation owner가 policy 평가로 pending Approval을 생성한다.
2. 원 operation을 park하고 operation identity를 유지한다.
3. approval Approved 후 wakeup이 원 operation 재평가를 트리거한다.
4. 재평가 시 stale CAS이면 승인됐더라도 mutation은 실패해야 한다.

CLI가 어떤 operation이 high-risk인지 새 규칙을 만들 수 없으므로 Security/Control/Application policy owner closure가 필요하다.

### 9.3 Process producer — BF-CLI-028

Process durable projection/show/watch는 존재하지만 실제 production Process 생성·transition producer가 없다. CLI에 새 Process create command를 추가하는 것은 금지 범위다. `DXB-RUN-038`의 internal process owner가 aggregate를 생성하고 lifecycle/revision을 갱신해야 show/watch journey가 실제로 성립한다.

## 10. Execution Closeout Stages

| Stage | Gate | 종료 조건 |
|---|---|---|
| C0 | Repository/branch audit | active product branch와 superseded/diagnostic branch 구분 |
| C1 | Parser/contract | 63 path, primary/secondary selector, typed field, local/wire split 검증 |
| C2 | Identity | verified Instance + authenticated Principal only |
| C3 | Dispatch | H/HQ/C/Q/S 모두 owner route 존재, false-success 없음 |
| C4 | Preflight/CAS | mutable path materialized CAS를 server에서도 검증 |
| C5 | Owner coverage | 63 row 각각 owner 또는 명시적 canonical blocker 분류 |
| C6 | Persistence/recovery | Application/Security/journal crash windows, exact retry, restart 검증 |
| C7 | Output | machine stdout clean, stream direct flush, SafeWriter, partial cursor |
| C8 | Binary/Journey | built `dxb` subprocess로 정상·실패·복구 여정 PASS |
| C9 | Two reviews | Structural/Consistency + Cross-Layer Executability 각각 완료 |

C0~C7의 코드가 존재해도 C8/C9가 없으면 `CLI Complete`가 아니다.

## 11. Mandatory Binary/Journey Evidence

최소 evidence set은 다음을 포함해야 한다.

1. offline top/group/command help와 version
2. first Runtime start / already-running / ambiguous Instance / explicit unknown Instance
3. authenticated peer Principal 및 spoof 거부
4. bot create/show, provider-unavailable activate, Ready-provider activate
5. conversation/thread send/history와 CAS conflict
6. task submit/list/show/watch/control/result의 적용 가능한 owner 경로
7. `--all` pagination ceiling/resume cursor
8. safe `--output`, overwrite/symlink 거부
9. machine JSON/JSONL parseability와 ANSI/stderr contamination 부재
10. graceful stop/host stop generation fence
11. process watch restart/resync — Process producer가 존재한 뒤
12. membership set/remove/re-add crash/restart와 generation 1→2→3
13. Approval decision restart, wakeup 복원, 원 operation re-evaluate — policy owner closure 뒤
14. Prepared/Dispatching response-loss recovery, exact retry, changed-digest conflict
15. corrupt/stale discovery/security/application/journal/coordination state fail-closed

## 12. Quality Gate

최종 head에서 아래를 모두 실제 실행해야 한다.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

추가로 built `dxb` binary/subprocess journey와 fault/restart evidence를 수행한다.

현재 execution 환경에는 `cargo` toolchain이 없고 GitHub network resolution도 사용할 수 없으므로 위 명령의 **실행 PASS 증거는 아직 없다**. 저장소 규칙에 따라 GitHub Actions를 실행해 이를 대체하지 않는다. 이 상태는 BF-CLI-031이며, `main` 병합 금지 조건이다.

`Cargo.lock` 또한 current dependency graph에서 regenerate/검증되어야 한다. lockfile을 checksum/전이 dependency 추측으로 손수 조작해 quality gate를 우회하지 않는다.

## 13. Recheck 1 — Structural / Consistency Checklist

첫 번째 재검수는 다음을 독립적으로 검사한다.

- file/path naming과 module loading
- crate dependency / Cargo.lock / feature orphan
- 63-row registry와 parser/help/dispatch의 단일 source
- Domain/Security/Provider/Runtime Canonical Owner 중복 없음
- BotId/Principal/Authority 경계
- Application/Security membership generation과 crash marker 일치
- protocol/schema version 및 snapshot compatibility
- local field/wire field/RequestDigest 관계
- docs/Acceptance/risk가 current implementation과 모순되지 않음
- test가 production semantic bypass fixture를 사용하지 않음

발견 사항 수정 후 이 checklist를 다시 통과해야 Recheck 1 완료로 기록한다.

## 14. Recheck 2 — Cross-Layer Executability Checklist

두 번째 재검수는 같은 파일 목록을 읽는 것이 아니라 실제 흐름을 다음 순서로 추적한다.

`Input/Command → Contract → Discovery/Auth → Control → Application/Security → Persistence → Runtime → Provider Host → Recovery → Projection → CLI`

각 흐름에 다음 adversarial condition을 대입한다.

- Provider none/unavailable/Ready/replaced
- stale revision/generation/HostGeneration
- response loss before/after durable commit
- duplicate delivery / changed digest / changed operation id
- SIGINT/broken pipe/local timeout
- process restart / Runtime restart
- Security/Application half-commit
- membership remove/re-add ABA
- cursor gap/reconnect/restart
- output ceiling/resource pressure
- corrupted/stale durable state

L4/L5 executable evidence까지 통과해야 Recheck 2를 최종 완료로 기록한다.

## 15. Main Merge / Completion Gate

다음 조건을 **모두** 만족하기 전에는 `main`으로 push/merge하지 않는다.

- [ ] BF-CLI-026 Runtime bootstrap default owner closure
- [ ] BF-CLI-027 Approval parking/continuation closure
- [ ] BF-CLI-028 Process production producer closure 또는 release scope의 canonical 재판정
- [ ] BF-CLI-029 frozen semantic field audit 0 silent-ignore
- [ ] BF-CLI-030 Cargo.lock regenerate/verified
- [ ] BF-CLI-031 fmt/clippy/test/binary/journey executable PASS
- [ ] Recheck 1 완료 + 발견 사항 재확인
- [ ] Recheck 2 완료 + 발견 사항 재확인
- [ ] final branch audit: unclassified ahead product branch = 0
- [ ] final head SHA와 evidence artifact/log를 기록

이 gate가 모두 닫힌 뒤에만 `CLI Complete`를 선언하고 `main`에 반영한다.

## 16. Stop Conditions

다음 상황에서는 구현을 임의로 확대하지 않는다.

1. frozen operation 의미를 바꿔야만 해결되는 경우
2. 새로운 authority principal mapping을 발명해야 하는 경우
3. high-risk approval policy를 새로 정의해야 하는 경우
4. Process public create/control command가 새로 필요하다고 추정되는 경우
5. Provider 제품을 추가해야 한다고 추정되는 경우
6. executable validation 없이 release/main merge를 강행해야 하는 경우

이 경우 해당 Canonical Owner plan/ADR을 먼저 갱신하고 closeout은 그 결과를 소비한다.

## 17. Evidence Ledger

| 날짜 | 기준 | Evidence | 판정 |
|---|---|---|---|
| 2026-08-29 | `codex/cli-completion-finalize` structural baseline `c6d9b049...` | parser/identity/transport/security/persistence/provider/watch/output static + test-source review | Recheck 1 진행 중 |
| 2026-08-29 | local execution environment | `cargo`/`rustc` executable unavailable; GitHub network resolution unavailable | executable gate 미충족 |
| 2026-08-29 | repository policy | GitHub Actions 실행 금지 | CI를 로컬 검증 대체물로 사용하지 않음 |

최종 ledger에는 문서 갱신 이후의 최종 head와 실제 실행 결과를 추가한다.

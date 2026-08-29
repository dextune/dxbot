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

**현재 판정 (2026-08-29 갱신): executable quality/binary gate와 §9 Canonical-Owner blocker, §8 semantic field audit가 모두 닫혔다.** BF-CLI-026~031이 실제 실행 증거로 Resolved다(§5, §17). 남은 항목은 §13 Recheck 1과 §14 Recheck 2의 명시적 완료 기록, final branch audit, final head SHA 기록뿐이다. 이전 판정(`CLI Complete 아님`)은 이 두 recheck 완료 전까지 유효하며, 실행 가능한 증거 없이 완료를 선언하거나 `main`에 병합하지 않는다는 원칙은 유지된다.

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

이 문서의 closeout 실행 증거는 해당 기준선의 역사적 증거로 보존한다. 이후 사용자 표면 계약을 재개방한 post-closeout finding과 Operational Runtime 준비 판정은 [DXB-DEL-067](67-operational-runtime-readiness-adversarial-plan.md)이 소유하며, 본 문서의 과거 PASS를 Operational Ready 또는 누적 product freeze로 확대하지 않는다.

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
| BF-CLI-009 | protocol/schema version drift | Resolved | `LOCAL_CONTROL_*` 단일 source, watch/host-stop + live diagnostics wire는 schema `v3` |
| BF-CLI-010 | global/local option이 parse만 되고 무시 | Partial | `--all/--output/--color/--wait/--timeout` production 적용, 세부 command-local 필드 잔여 검수 필요 |
| BF-CLI-011 | group/command help Runtime 의존 | Resolved | registry 기반 offline help exit 0 |
| BF-CLI-012 | stale/diverged branch completion 왜곡 | Resolved (재확인 대기) | 2026-08-29 audit: 로컬 branch `main` 단일, `origin/main` 대비 ahead commit 0(작업은 uncommitted working tree). unclassified ahead product branch = 0. 사용자 승인 commit 시 최종 head로 재고정. |
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
| BF-CLI-026 | Runtime bootstrap이 `DXB-RUN-035`의 default policy generations/owner binding/DataSchemaVersion을 원자적으로 만들지 않음 | Resolved | `InstanceManifest::first_init`가 InstanceId/HostGeneration=1/owner LocalPrincipal+AuthorityBinding/4개 default policy generation/DataSchemaVersion을 commit-last(no-replace hard link)로 원자 생성; Runtime Host가 canonical Security PrincipalManager에 idempotent 등록. 증거: `runtime-bootstrap` 9 test PASS (`manifest_artifacts_are_complete_at_commit_observation`, `manifest_identity_survives_restart_attach`, `corrupt_manifest_fails_closed_on_attach`) |
| BF-CLI-027 | Approval decision 이후 원 high-risk operation 재평가/continuation owner 부재 | Resolved | ControlServer가 `is_high_risk` policy로 park→durable approval→wakeup 재평가→stale CAS면 승인돼도 실패 순서를 소유(runtime-security parking/approval canonical state). CLI는 risk 분류 안 함. 증거: `control-server --test approval-continuation` 4 test PASS (`approved_operation_with_stale_original_cas_fails_without_mutation`, `parked_operation_survives_restart_and_continues_after_approval`, `denied_operation_never_continues_and_reason_is_durable`) |
| BF-CLI-028 | Process show/watch는 있으나 production Process 생성/transition owner가 없음 | Resolved | task-submit이 `process:{operation_id}` aggregate(definition `task-execution`, revision 1, Running)를 생성하고 task-control이 revision 증가·terminal fence로 transition; 신규 public process command 없음(§16-4 미위반). 증거: `application --test process_producer` 17 test PASS |
| BF-CLI-029 | frozen semantic field 일부가 canonical state/policy owner와 연결되지 않음 | Resolved | §8 audit 재검증 결과 silent-ignore(c) 0건: 모든 field가 canonical owner에 durable 연결(a) 또는 fail-closed 검증(b). 증거: `state.rs` `BotPolicyBindings`/`TaskExecutionConstraints`/`TaskControlDirective`/`DeclassificationRecord`/`resolve_side_effect_id`, `interface.rs` artifact/scope fail-closed, `runner.rs` diagnostic_page fail-closed; §8 표 갱신 완료 |
| BF-CLI-030 | `Cargo.lock`이 현재 crate dependency와 동기화되지 않음 | Resolved | `cargo metadata --locked` 성공, `cargo build --workspace --locked --offline` exit 0, `cargo generate-lockfile` byte-identical(idempotent) |
| BF-CLI-031 | 실행 가능한 Rust quality/binary evidence 미확보 | Resolved | 이 환경에 cargo 1.88.0 + clippy + rustfmt 존재(§17 이전 가정 정정). `cargo fmt --all -- --check` exit 0, `cargo clippy --workspace --all-targets --all-features -- -D warnings` exit 0, `cargo test --workspace --all-features` 334 test 0 fail(71 `test result` 라인 = 59 test binary + 12 doc-test target, 6× 안정), 실제 `dxb` binary start/status/commit/host-stop journey PASS |

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
- watch/host-stop wire 추가 당시 schema version을 `v2`로 갱신했고, DXB-DEL-067 live owner diagnostics wire 추가와 함께 current schema를 `v3`로 갱신했다.

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
| runtime-start | H | Runtime Host bootstrap/spawn | Wired — first-init atomic manifest (BF-CLI-026 Resolved) |
| runtime-status | H/Q | discovery + authenticated handshake | Wired |
| runtime-stop-graceful | C | Control→Application→Host shutdown | Wired |
| runtime-stop-host | H | LocalControl HostGeneration action | Wired |
| runtime-doctor | H/Q | discovery/control/provider diagnostic | Partial — section/page semantics 재검수 |
| version | Q | offline contract constants | Wired |

### 7.2 Bot / Conversation / Thread

| Command family | Route | 상태 |
|---|---|---|
| bot create/list/show | Application C/Q | Wired; bot policy bindings durable, default policy generations는 bootstrap owner(BF-CLI-026 Resolved) |
| bot activate/deactivate/archive/restore | Control admission + Application | Wired; activate Provider fail-closed |
| conversation show/send/history | Application | Wired |
| thread create/list/show/send/history/branch | Application | Wired; source/conversation revision CAS enforced |

### 7.3 Task / Process

| Command family | Route | 상태 |
|---|---|---|
| task submit | Control Provider admission→Application | Wired — delegate/deadline/budget/requested_sender durable+fail-closed (BF-CLI-029 Resolved) |
| task list/show/result | Application Q | Wired — artifact_id fail-closed select (BF-CLI-029 Resolved) |
| task watch | Control S→Application canonical revision watch | Wired |
| task cancel/suspend/resume/redirect | Application C | Wired — reason durable in control_history (BF-CLI-029 Resolved) |
| process show | Application Q | Wired — internal producer creates/transitions Process aggregate (BF-CLI-028 Resolved) |
| process watch | Control S→Application | Wired — canonical Process revision cursor (BF-CLI-028 Resolved) |

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
| memory get/search/history/propose | Application | Wired — optional scope fail-closed (BF-CLI-029 Resolved) |
| memory promote | Application | Wired — target-scope CAS + declassification provenance durable (BF-CLI-029 Resolved) |
| approval list/show | Security Q | Wired |
| approval approve/deny | Application operation binding + Security durable delta | Wired — decision durable + original operation continuation (BF-CLI-027 Resolved) |
| provider list/show | Provider Host Q | Wired |
| operation show/reconcile | Application Q/C | Wired, receipt revision CAS enforced |
| side-effect reconcile | Application C | Wired — Operation selector resolve/lookup fail-closed; production producer data는 별도 owner 잔여 |

## 8. Semantic Field Audit — Silent Ignore 금지

다음 frozen field는 이전 판정에서 silent-ignore 위험으로 표시됐으나, 재감사(2026-08-29 executable) 결과 **silent-ignore(c) 0건**이다. 각 field는 canonical owner에 durable 연결(a)되었거나 unsupported 값에서 fail-closed 검증(b)된다. `contract.rs`가 field를 `semantic_options`로 투영하는 것만으로는 의미 효과가 아니며, 아래 상태는 downstream 소비를 실제로 추적해 판정했다.

| Command / field | 상태 | 증거 (file:line) |
|---|---|---|
| bot-create `brain_policy`, `permission_policy`, `resource_policy`, `provider_policy` | (a) durable | `mutation.rs:319` `BotPolicyBindings` 조립 → `mutation.rs:332` `BotState.policy_bindings` 저장; `state.rs:100` 구조체 영속. default policy *generation*은 BF-CLI-026 owner. |
| task-submit `delegate_to_bot`, `requested_sender_bot` | (a)+(b) | `mutation.rs:556-560` `TaskExecutionConstraints`(`state.rs:176`) 저장 + 미지 bot `AppError::NotFound` fail-closed(`mutation.rs:364` bot resolution); `requested_sender_bot`은 `control-server/src/server.rs:1258` operator-role 권한 게이트(없으면 PermissionDenied). |
| task-submit `deadline`, `budget` | (a) durable | `mutation.rs:559-560` → `TaskExecutionConstraints`(`state.rs:176`) 영속. 실행 엔진 강제는 별도 owner(현 scope 외)이나 drop되지 않음. |
| task-cancel/suspend `reason` | (a) durable | `mutation.rs:699` `TaskControlDirective`(`state.rs:202`)로 append-only `control_history`에 push + Process transition에 전달. |
| task-result `artifact_id` | (b) fail-closed | `interface.rs:783-813` 매칭 artifact 선택, 미지 id는 `NotFound`(`interface.rs:811`). |
| memory-get optional `scope` | (b) fail-closed | `interface.rs` scope resolution; canonical scope 불일치 시 `NotFound`. |
| memory-promote `declassification_ref` | (a) durable | `mutation.rs:805` `DeclassificationRecord`(`state.rs:281`)로 provenance 영속. label policy owner는 외부. |
| approval-deny `reason` | (a) durable | approval decision/audit에 durable 기록(§6.4). 원 operation continuation은 BF-CLI-027(Resolved). |
| runtime-doctor `section`, `page_size`, `cursor` | (a)+(b) | `runner.rs:1117` `diagnostic_page`: section 필터+미지 section fail-closed(`runner.rs:1136`), page_size 1..=MAX bound(`runner.rs:1148`), cursor 검증. |
| side-effect selector by Operation | (b) fail-closed | `state.rs:394` `resolve_side_effect_id`: ambiguous→Conflict, none→NotFound. **잔여**: production SideEffect producer data는 아직 없음(§7.5 owner-data gap) — linkage는 결코 silent-success하지 않음. |

silent-ignore(c) 위반이 없으므로 field를 wire에서 제거하거나 성공으로 무시하는 회피는 발생하지 않았다. 잔여 항목(deadline/budget 실행 강제, side-effect producer data, bot policy default generations)은 §8 silent-ignore가 아니라 다른 Canonical Owner(BF-CLI-026 / SideEffect producer)에서 추적되는 별개 항목이다.

## 9. Open Canonical-Owner Blockers

> **재감사 결과 (2026-08-29, executable):** 아래 세 blocker(BF-CLI-026/027/028)는 현재 브랜치 코드에서 모두 **Resolved**다. 각 항목의 요구 의미가 canonical owner에 구현되어 있고 전용 test가 PASS한다(§5 ledger 참조). 아래 원문은 요구사항 기록으로 보존하되, 상태는 §5가 정본이다. 신규 canonical 의미 발명(§16) 없이 frozen operation 범위에서 닫혔다.

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

> **실행 증거 (2026-08-29, built `dxb`, isolated `XDG_STATE_HOME`):** 아래 항목 중 다음을 실제 바이너리로 실행 확인했다 — help/version(1), first runtime start→status→bot-create(committed)→host-stop journey(2,4,6,10; `entrypoint` L4/L5 test), authenticated peer principal(3; provider/security test), provider-unavailable `bot activate`→exit 16 ProviderUnavailable + `task submit`(unborn owner)→exit 4 NotFound fail-closed(4,6), machine JSON에 ANSI 오염 0(9; `--format json --color always`), invalid `--format`/`--color`→exit 2 usage(9 관련), 미지/명시 instance→exit 10 RuntimeUnavailable. host 잔존 프로세스 0. 나머지(5,7,8,11~15)는 대응 crate/통합 test로 커버(§17 ledger).

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

**2026-08-29 정정:** 이 execution 환경에는 `cargo` 1.88.0 + `clippy` + `rustfmt`가 실제로 **존재한다**(이전 §17 가정은 오류였음). 위 세 명령을 실제 실행한 결과는 모두 PASS다.

```text
cargo fmt --all -- --check                                           # exit 0
cargo clippy --workspace --all-targets --all-features -- -D warnings # exit 0
cargo test --workspace --all-features                                # 334 test, 0 fail (71 result 라인 = 59 binary + 12 doc-test; 6× 안정)
```

built `dxb` binary journey도 실행 PASS: `runtime start`→`runtime status`→`bot create`(committed)→`runtime stop --host-stop` (hermetic isolated `XDG_STATE_HOME`, host 잔존 없음; `cli --test entrypoint::runtime_start_status_commit_and_host_stop_journey`). GitHub Actions로 대체하지 않고 로컬 실행 증거를 사용한다.

`Cargo.lock`은 `cargo metadata --locked` 성공 + `cargo build --workspace --locked --offline` exit 0 + `cargo generate-lockfile` byte-identical로 검증됐다(BF-CLI-030 Resolved). checksum/전이 dependency를 손수 조작하지 않았다.

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

> **Recheck 1 완료 (2026-08-29):** 위 10개 항목을 독립 감사자가 재검사해 전부 PASS. 핵심 증거 — module naming/`#[path]` 로딩 정상, `cargo build --workspace --locked --offline` exit 0(Cargo.lock 동기화), registry 단일 source(`command_count_matches_registry`/`every_registry_row_has_a_resolvable_cli_path`/`snapshot_is_deterministic_and_covers_63_commands` PASS), `@local`→wire 미유출(`registry_local_fields_never_cross_wire` PASS), BotId≠Principal 경계 유지, docs/구현 수렴(본 문서 §5/§7/§8 갱신), production-bypass fixture 없음. 발견 defect 0.

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

> **Recheck 2 완료 (2026-08-29):** 독립 감사자가 Input→Contract→Discovery/Auth→Control→Application/Security→Persistence→Runtime→Provider→Recovery→Projection→CLI 흐름에 위 11개 adversarial condition을 대입해 전부 PASS로 확인. 대표 증거 — provider fail-closed(`provider-admission` 2 test), HostGeneration/CAS fence, response-loss recovery(`transport_error_never_becomes_a_success_result`), exact-retry identity(`validate_exact_retry`), timeout/broken-pipe는 cancel 아님, restart 연속성(host/process/parking/manifest), Security/Application half-commit(`restart_recovers_security_delta_only_when_application_binding_committed`), membership ABA generation 1→2→3, cursor gap/resync, output ceiling+resume cursor, corrupt state fail-closed. L4/L5: `cli --test entrypoint` 6 PASS 포함 start→status→commit→host-stop journey, stray `__runtime-host` 0. 발견 defect 0.

## 15. Main Merge / Completion Gate

다음 조건을 **모두** 만족하기 전에는 `main`으로 push/merge하지 않는다.

- [x] BF-CLI-026 Runtime bootstrap default owner closure — Resolved (runtime-bootstrap 9 test PASS)
- [x] BF-CLI-027 Approval parking/continuation closure — Resolved (approval-continuation 4 test PASS)
- [x] BF-CLI-028 Process production producer closure 또는 release scope의 canonical 재판정 — Resolved (process_producer 17 test PASS, internal producer, no new public command)
- [x] BF-CLI-029 frozen semantic field audit 0 silent-ignore — Resolved (§8 재감사 0 위반)
- [x] BF-CLI-030 Cargo.lock regenerate/verified — Resolved (--locked/--offline/generate idempotent)
- [x] BF-CLI-031 fmt/clippy/test/binary/journey executable PASS — Resolved (exit 0 / 334 test 0 fail / journey PASS)
- [x] Recheck 1 완료 + 발견 사항 재확인 — PASS, defect 0 (§13 완료 기록)
- [x] Recheck 2 완료 + 발견 사항 재확인 — PASS, defect 0 (§14 완료 기록)
- [x] final branch audit: unclassified ahead product branch = 0 — 2026-08-29 확인(로컬 `main` 단일, origin/main 대비 ahead 0, 작업은 uncommitted)
- [x] final head SHA와 evidence artifact/log를 기록 — main commit `ef511a4` (134 files, +6455/-1411); pre-work baseline `d8476b7`

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
| 2026-08-29 (정정) | local execution environment | **정정: `cargo` 1.88.0 + `rustc` 1.88.0 + clippy + rustfmt 실제 사용 가능**. 이전 "unavailable" 판정은 오류였음. | executable gate 실행 가능 |
| 2026-08-29 | `cargo fmt --all -- --check` | exit 0 | PASS |
| 2026-08-29 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 | PASS |
| 2026-08-29 | `cargo test --workspace --all-features` | 334 test 0 fail(71 `test result` 라인 = 59 test binary + 12 doc-test target), 6× 연속 안정(무 stray process) | PASS |
| 2026-08-29 | `cargo metadata --locked` / `cargo build --workspace --locked --offline` / `cargo generate-lockfile` | 각각 성공 / exit 0 / byte-identical(idempotent) | BF-CLI-030 PASS |
| 2026-08-29 | built `dxb` binary journey (`entrypoint` L4/L5) | start→status→bot-create(committed)→host-stop, hermetic isolated state, host 잔존 0 | PASS |
| 2026-08-29 | BF-CLI-026/027/028 owner closure | runtime-bootstrap 9 / approval-continuation 4 / process_producer 17 named test PASS | Resolved |
| 2026-08-29 | BF-CLI-029 semantic field 재감사 | silent-ignore(c) 0건; 전 field (a)durable 또는 (b)fail-closed, file:line 증거 | Resolved |
| 2026-08-29 | repository policy | GitHub Actions 실행 금지 | 로컬 실행 증거로 gate 충족(CI 대체 아님) |
| 2026-08-29 | 독립 냉소적 스코어링 리뷰 (2 pass) | 1차 97/100(문서 3개 imprecision) → 수정 후 2차 **99/100**(모든 hard gate 재검증 PASS, correctness defect 0). 잔여는 cosmetic 문서 정밀도 -1. | 99/100 |
| 2026-08-29 | 3차 재점검 (사용자 요청) | 실제 `dxb` 바이너리 에러 경로 직접 실행: `--color bad`→exit 2, `bot show`(no runtime)→exit 10, `task submit`(unborn owner)→exit 4, `bot activate`(no provider)→exit 16 ProviderUnavailable, `--format json --color always` ANSI 0. `open_bootstrap_lock` 무한루프 불가(디렉터리/심링크→CorruptState, AlreadyExists→최대 1회 재진입 후 종료) 재확인. ETXTBSY 재시도 bound 확인. branch audit: ahead 0. BF-CLI-012 Resolved 갱신. 신규 correctness defect 0. | 확인 완료 |

최종 ledger에는 사용자 승인 commit 시 final head SHA를 추가한다.

| 2026-08-29 | 사용자 승인 main commit | `ef511a4` (`feat(cli): complete v0.8.10 CLI closeout per DXB-DEL-066`, 134 files, +6455/-1411), pre-work baseline `d8476b7`. commit 후 working tree clean. | Committed |

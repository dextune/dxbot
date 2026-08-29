---
title: "CLI 이후 DXBOT 실운영 준비 적대적 개발 플랜"
document_id: "DXB-DEL-067"
version: "0.8.10"
status: "Accepted"
normative: false
priority: "P0"
last_updated: "2026-08-29"
depends_on: ["DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-066", "DXB-ARC-010", "DXB-ARC-011", "DXB-ARC-018", "DXB-ARC-019", "DXB-DOM-021", "DXB-DOM-022", "DXB-DOM-023", "DXB-DOM-024", "DXB-DOM-025", "DXB-RUN-031", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-034", "DXB-RUN-035", "DXB-RUN-038", "DXB-IFC-041", "DXB-IFC-043"]
---
# CLI 이후 DXBOT 실운영 준비 적대적 개발 플랜

## 1. 목적

이 문서는 `DXB-DEL-066`의 CLI closeout 이후 DXBOT을 **실제 지속형 AI Bot Runtime으로 운영 가능한 상태**까지 수렴시키기 위한 비규범 실행 플랜이다.

두 문제를 명시적으로 분리한다.

1. **CLI closeout 잔여 계약 결함**: `dxb → Control → Application/Security → durable result/recovery` 실행 spine은 상당히 닫혔지만, post-closeout 적대적 재감사에서 사용자 표면 계약 누락이 다시 발견됐다.
2. **Operational Runtime gap**: CLI가 Task/Process를 만들 수 있는 것과 Brain/Context/Scheduler/Core/Provider가 실제 AI 실행을 수행하는 것은 별개다. 현재 후자는 production composition 기준으로 아직 닫히지 않았다.

이 문서는 신규 제품 의미를 임의로 정의하지 않는다. 이미 Accepted인 Domain/Runtime/Provider 계약을 실제 production composition과 executable journey로 연결하는 순서, blocker, evidence gate를 소유한다.

## 2. Baseline과 판정 범위

### 2.1 Repository baseline

2026-08-29 post-closeout 적대적 재감사 기준은 GitHub `main`의 다음 상태다.

- main HEAD: `2ef238af75aa1390bcabf915f38cc5be9b10377d`
- CLI closeout implementation commit: `ef511a4941fff8fa5793e8a173d54bcc52acff30`
- closeout evidence owner: [DXB-DEL-066](66-cli-completion-adversarial-plan.md)

`DXB-DEL-066`에 기록된 fmt/clippy/test/binary evidence는 역사적 실행 증거로 보존한다. 본 재감사는 GitHub `main`의 코드·계약 관계를 다시 추적한 **독립 static adversarial review**이며, 기존 실행 로그를 새로 실행한 것으로 간주하지 않는다.

### 2.2 현재 판정

**CLI execution spine: Conditional PASS. CLI contract-complete 선언: Reopened. Operational DXBOT: NOT READY.**

강한 부분:

- 63-row registry 기반 path/typed input/dispatch
- verified Instance selection과 authenticated LocalPrincipal boundary
- durable CLI journal과 exact retry/recovery
- Application/Security cross-owner coordination
- Approval parking/continuation
- bounded query/`--all`/safe output
- Task/Process watch direct flush와 cursor/gap/resync
- machine JSON/JSONL ANSI 오염 방지
- Runtime bootstrap identity/default-generation manifest

그러나 아래 §3 CLI residual finding이 열려 있고, §4의 실제 AI execution spine이 production Runtime Host에 연결되지 않았으므로 `dxb task submit` 성공을 곧바로 "Bot이 실제 AI 작업을 수행한다"는 의미로 확대해서는 안 된다.

## 3. Post-closeout CLI Adversarial Findings

상태는 `Open | Resolved | Evidence Pending`만 사용한다.

| ID | Finding | 상태 | 근거 / 종료 조건 |
|---|---|---|---|
| BF-OR-CLI-001 | `dxb version`이 live endpoint가 있어도 remote compatibility를 추가하지 않음 | Open | `DXB-IFC-043`은 endpoint가 있으면 remote compatibility를 추가하도록 요구한다. 현재 `Discovery::show_version()`은 항상 `remote_compatibility=None`; production `render_version()`은 endpoint handshake를 시도하지 않는다. offline version은 계속 성공해야 하며 endpoint가 있을 때만 bounded best-effort compatibility projection을 추가한다. |
| BF-OR-CLI-002 | `runtime start --ready-at`이 frozen local field인데 production에서 silent-ignore | Open | registry는 `ready_at:ReadyAt?@local{process,storage,runtime,control}=control`을 소유한다. 현재 `start_runtime()`은 값을 읽지 않고 항상 Control handshake readiness까지 대기한다. 모든 allowed value를 실제 readiness predicate로 구현하거나 지원하지 않는 값은 fail closed해야 한다. |
| BF-OR-CLI-003 | human mutation output이 `DXB-IFC-043`의 first-use 정보 순서를 충족하지 못함 | Open | 현재 `render_operation()`은 command/status/OperationId만 출력한다. selected Instance, committed resource refs(`BotRef/MainConversationRef/TaskRef/ProcessRef` 등), receipt state, next safe action을 안전하게 보여줘야 한다. raw content/secret은 echo하지 않는다. |
| BF-OR-CLI-004 | human failure가 owner/renderer의 typed next action을 사용자에게 노출하지 않음 | Open | machine error는 `next_actions`를 보강하지만 human `render_error()`는 code/message만 출력한다. RuntimeUnavailable/ProviderUnavailable/ApprovalRequired/RecoveryRequired/Conflict/Partial 등에 대해 next safe action을 human에서도 보여주되 raw shell string을 stable contract로 만들지 않는다. |
| BF-OR-CLI-005 | unknown command의 bounded distance-thresholded suggestion이 없음 | Open | `DXB-IFC-041` 요구와 달리 `resolve_cli_path()`는 `unknown command path`만 반환한다. 63-row registry에서만 bounded candidate를 만들고 threshold 밖이면 suggestion을 생략한다. 두 번째 semantic command table을 만들지 않는다. |
| BF-OR-DOC-001 | M6 cumulative freeze evidence 문구와 M5 Provider Acceptance 상태가 모순 | Open | `DXB-DEL-060`은 누적 gate지만 `DXB-DEL-061`의 Provider/DeepSeek 13개 Acceptance는 `Executable`, `AT-SCHEMA-001` evidence는 "all prior milestones passed"라고 기록한다. AT-SCHEMA 자체 결과와 M6 cumulative freeze를 분리하고 실제 provider evidence 전에는 full product freeze를 주장하지 않는다. |

### 3.1 CLI 재폐쇄 원칙

위 finding은 CLI의 security/recovery/transport 기반을 전면 재작성하라는 의미가 아니다. 기존 closeout evidence를 재사용하고 다음 최소 회귀 증거만 추가한다.

- live Runtime이 있는 `dxb version --format json`에서 `remote_compatibility`가 채워짐
- Runtime이 없을 때 version은 여전히 offline exit 0
- `runtime start --ready-at <allowed>` 각 predicate가 실제 의미를 가짐
- `bot create` human output에서 BotRef/MainConversationRef/Instance/Operation을 다음 명령에 사용할 수 있음
- ProviderUnavailable/ApprovalRequired/RecoveryRequired human output에 next safe action이 보임
- unknown path suggestion은 bounded/thresholded하며 ambiguous 또는 먼 후보를 강제하지 않음

## 4. Operational Runtime Blocking Findings

### BF-OR-RUN-001 — Production Provider composition 부재

현재 `LocalRuntimeHost`는 `ControlServer::with_persistence(..., ProviderHost::new(), ...)`로 빈 Provider Host를 조립하고 discovery에 `provider_id="unconfigured"`, `provider_ready=false`를 publish한다.

`provider-host`에는 HTTP transport, protocol handler, `RealProvider`, DeepSeek adapter와 `execute_task()`가 존재하지만 production Runtime start가 이를 구성하지 않는다.

**종료 조건:** owner-only production configuration에서 provider endpoint/model/capability/generation과 credential reference를 resolve해 **동일 ProviderHost instance**에 transport/real provider를 등록한다. secret material은 discovery/Application/CLI journal에 복제하지 않는다. ReferenceProvider는 explicit test policy 외 production fallback으로 쓰지 않는다.

### BF-OR-RUN-002 — Task와 Execution이 동일 의미로 축약됨

`DXB-DOM-023`은 Task를 durable work intent, Execution을 immutable attempt, Core Lease를 transient resource로 구분한다. 현재 `task submit`은 durable Task와 Process를 만들지만 canonical Execution attempt가 없다.

**종료 조건:** admitted Task마다 explicit Execution identity/generation과 immutable Context Plan binding을 만들고 Task lifecycle과 Execution attempt lifecycle을 분리한다. retry/recovery가 기존 Task를 덮어쓰지 않고 새 attempt 또는 safe resume 의미를 가진다.

### BF-OR-RUN-003 — Brain / Context Plan production builder 부재

`DXB-DOM-021`은 Application Context Plan owner가 current canonical revisions를 읽어 candidate plan을 구성하고 Task/Execution commit에서 immutable snapshot/ref를 영속화하도록 요구한다.

**종료 조건:** Bot identity/policy, Conversation/Thread/Project/Channel refs, Memory assertion refs, capability requirement, provider binding generation, permission/resource/deadline budget을 하나의 bounded Context Plan으로 조립한다. Runtime/Provider는 committed plan을 소비할 뿐 수정하지 않는다.

### BF-OR-RUN-004 — Resource Governor / Dynamic Core Scheduler production path 부재

`DXB-DOM-024`의 admission flow는 authorization/capability → memory/cost/concurrency admission → fair scheduling → Core Lease + Provider activity → Execution run → release/accounting 순서다. 현재 workspace에서 이 흐름을 production Task runner가 소유하는 경로가 닫혀 있지 않다.

**종료 조건:** 기존 crate 경계를 우선 재사용하여 Runtime composition 내부에 Resource Governor와 Scheduler owner를 연결한다. crate-per-concept로 쪼개지 않는다. Core Lease는 canonical Domain state 복사본이 아니라 generation-fenced transient lease이며 cancellation/shutdown/restart 뒤 permit leak가 없어야 한다.

### BF-OR-RUN-005 — Provider execution → Task Result/Evidence atomic commit 부재

ProviderHost `execute_task()`는 존재하지만 submitted Task를 가져와 실행하고 결과를 Task/Process/Receipt/Evidence에 반영하는 production coordinator가 없다.

**종료 조건:** one admitted Execution이 Context Plan을 소비해 Provider activity를 획득하고 bounded result/evidence를 반환하며, Application owner가 stale ExecutionGeneration을 fence한 뒤 Task Result와 Process progress/outcome refs를 commit한다. Provider thread/callback이 DomainState를 직접 mutate하지 않는다.

### BF-OR-RUN-006 — Process lifecycle이 실제 execution checkpoint와 연결되지 않음

현재 task submit은 즉시 `ProcessLifecycle::Running` aggregate를 만들지만 실제 provider activity/current step/waiting checkpoint가 존재하지 않는다.

**종료 조건:** Process가 `DXB-RUN-038`대로 current step/activity ref, waiting condition/continuation, child refs, progress/outcome refs를 통해 실제 실행 진행을 표현한다. Task lifecycle을 복제하지 않고 restart 시 canonical child state에서 continuation을 재구성한다.

### BF-OR-RUN-007 — Memory가 Context와 실행 결과에 폐루프되지 않음

현재 Memory CRUD/proposal/promotion은 존재하지만 `DXB-DOM-022`의 epistemic/provenance/temporal validation, derived index watermark, Context retrieval, execution Evidence→Memory Proposal 흐름이 production execution과 연결되어 있지 않다.

**종료 조건:** Context Builder가 bounded candidate retrieval 후 canonical revision/scope/epistemic/temporal/authorization을 재검증하고, Task Result는 Memory와 자동 동일시하지 않는다. 실행 결과에서 장기 기억이 필요하면 typed Memory Proposal을 거쳐 promotion한다.

### BF-OR-RUN-008 — Multi-Bot delegation이 recipient durable execution으로 폐쇄되지 않음

`application::DelegationManager` component는 membership/scope checks를 제공하지만 production `task submit delegate_to_bot`은 현재 constraint를 저장하는 수준이다. `DXB-DOM-025`는 delegation을 recipient Bot의 durable Task intent로 정의한다.

**종료 조건:** sender authority를 server-side로 검증하고 recipient Task를 Application UoW에서 생성한다. recipient Scheduler/Core/Memory를 sender가 직접 조작하지 않는다. duplicate delegation은 duplicate Task effect를 만들지 않고 fan-out/item/byte/deadline/budget cap을 적용한다.

### BF-OR-RUN-009 — Side Effect Ledger producer linkage 불완전

CLI closeout은 side-effect selector/reconcile 경로를 fail-closed로 연결했지만 production side-effect producer data는 별도 owner gap으로 남아 있다.

**종료 조건:** 외부 observable effect 전 `Prepared`, dispatch 후 `Dispatched`, evidence 확인 후 `Confirmed/Failed/Unknown`을 Application/side-effect owner가 기록한다. `Unknown` blind retry를 금지하고 Process/Execution/Operation ref로 reconcile한다.

### BF-OR-RUN-010 — Operational persistence / backup / migration gate 미폐쇄

현재 로컬 Runtime은 Application/Security JSON state를 durable하게 복원할 수 있으나, 장기 운영의 snapshot/versioned row/tombstone/compaction/disk-pressure/migration 기준은 `DXB-ARC-018`에서 별도 executable evidence를 요구한다.

**종료 조건:** DB 제품을 먼저 고정하지 말고 실제 operational workload에서 atomic UoW, reader/writer fencing, snapshot watermark, nonterminal recovery retention, disk-full fail-closed, backup/restore, old-schema migration/rollback rehearsal를 통과한 storage profile을 선택한다.

### BF-OR-RUN-011 — Daemon/service와 운영 관측 표면 미폐쇄

`dxb runtime start`는 local host process를 만들 수 있지만 24/7 운영에는 supported supervisor lifecycle, health/readiness, shutdown drain, restart, backup, resource pressure, audit integrity 상태가 필요하다.

**종료 조건:** private `__runtime-host`를 외부 운영 계약으로 승격하지 않고 supported Runtime Host entrypoint를 통해 system supervisor와 연동한다. `runtime status/doctor`는 discovery snapshot만이 아니라 각 canonical owner의 live health projection을 bounded하게 집계한다.

## 5. Canonical Owner Matrix

| 의미 | Canonical Owner | 구현 원칙 |
|---|---|---|
| CLI path/input/render/journal | `application-contract` registry + `cli` | 새 command table 금지 |
| Runtime bootstrap/Instance/default generations | `runtime-bootstrap` / Runtime Host | Provider/Domain state 복제 금지 |
| Provider registration/lifecycle/transport | `provider-host`, `DXB-ARC-019` | Runtime Host는 composition만 수행 |
| credential/secret policy | Security/Config owner | raw secret를 Domain/journal/discovery에 저장 금지 |
| Task/Execution/Result | `application`, `DXB-DOM-023` | Provider가 직접 mutate 금지 |
| Brain/Context Plan | Application Context Plan owner, `DXB-DOM-021` | immutable snapshot/ref |
| Resource admission/Core Lease | Runtime Resource Governor/Scheduler, `DXB-RUN-031`, `DXB-DOM-024` | transient lease, generation fence |
| Process progress/recovery | Application Process owner, `DXB-RUN-038` | child state 복제 금지 |
| Memory assertions | Application Memory owner, `DXB-DOM-022` | index/cache는 Derived State |
| Membership/Delegation | Application + Security authority boundary | BotId ≠ PrincipalRef |
| Approval/Authority | `runtime-security` | CLI policy invention 금지 |
| Audit | `runtime-audit`, `DXB-RUN-034` | required AuditIntent fail-closed |
| Side Effect Ledger | Application side-effect owner | unknown blind retry 금지 |

## 6. Development Stages

각 단계는 선행 gate가 닫힌 뒤 진행한다. 병렬 개발은 owner가 독립적일 때만 허용하고 동일 canonical state를 두 구현이 동시에 소유하지 않는다.

### R0 — CLI residual closeout 재폐쇄

목표: §3의 5개 CLI 계약 결함을 기존 abstraction 위에서 최소 수정한다.

작업:

1. `version` offline/live dual path를 구현한다.
2. `runtime-start ready_at` predicate를 Runtime readiness stage와 연결한다.
3. human operation renderer가 committed refs/Instance/receipt/next action을 노출하도록 개선한다.
4. human error renderer가 typed next actions를 안전하게 표시한다.
5. registry-derived unknown-command suggestion을 bounded threshold로 추가한다.
6. entrypoint binary regression을 추가한다.

Gate:

- 기존 63 registry unchanged
- machine JSON/JSONL shape unchanged unless version-compatible additive field already defined
- no new Runtime auto-start
- no semantic retry
- 관련 CLI binary tests PASS

### R1 — Acceptance / release-evidence 정합성 복구

목표: CLI Complete와 Operational Runtime/Product Freeze를 문서와 evidence에서 분리한다.

작업:

1. `AT-SCHEMA-001` 자체 fixture PASS와 M6 cumulative product freeze를 구분한다.
2. `AT-PROVIDER-INFRA-001~008`, `AT-DEEPSEEK-001~005`는 실제 local execution evidence 전까지 `Executable`로 유지한다.
3. M5 Provider acceptance가 모두 Passed 되기 전 "all prior milestones passed" 또는 full operational freeze 문구를 사용하지 않는다.
4. `DXB-DEL-066`은 CLI closeout historical evidence를 보존하고 본 문서의 reopened residual finding으로 연결한다.

### R2 — Production Provider configuration / composition

목표: Runtime start 후 실제 production Provider가 같은 ProviderHost owner에 등록되고 readiness가 live하게 관측된다.

선행 결정:

- Provider registry/credential/config는 `DXB-ARC-019` 비범위였으므로 구현 전에 Canonical config/secret owner를 문서/ADR로 고정한다.
- config는 Provider generation/capability/endpoint/model/credential reference를 표현하되 secret value를 discovery나 Application state에 복제하지 않는다.

작업:

1. Runtime Host가 owner-only config를 읽어 Common `HttpTransport`를 한 번 구성한다.
2. DeepSeek 등 `RealProvider` adapter를 등록한다.
3. registration generation/status/capability를 ProviderHost가 소유한다.
4. health/readiness 결과에서 discovery/status/doctor projection을 갱신한다.
5. provider replace/reload가 필요하면 old generation activity drain/fence를 먼저 정의한다.
6. provider list/show와 doctor가 같은 live owner state를 읽도록 수렴한다.

Gate:

- unconfigured → typed ProviderUnavailable
- configured Ready → provider list/show/doctor 모두 동일 generation/status
- bad credential/endpoint → secret 비노출 + fail-closed
- ReferenceProvider production fallback 0
- AT-PROVIDER-INFRA-001~008, AT-DEEPSEEK-001~005 local evidence PASS

### R3 — Task Admission → Execution → Context Plan

목표: `task submit`이 단지 row를 만드는 데서 끝나지 않고 실행 가능한 immutable attempt를 생성한다.

작업:

1. submitted Task의 canonical lifecycle을 `DXB-DOM-023`와 정합화한다.
2. admission owner가 authorization/capability/resource/deadline/budget을 검증한다.
3. explicit `ExecutionId` / generation / attempt identity를 만든다.
4. Application Context Plan builder가 current canonical revisions에서 bounded plan을 조립한다.
5. Execution 생성 commit에 immutable Context Plan snapshot/ref와 Provider binding generation을 저장한다.
6. retry는 Task identity를 유지하되 attempt identity를 새로 만들거나 documented safe resume를 사용한다.

금지:

- Task row를 Execution mutable scratchpad로 사용
- raw Provider session을 Bot/Brain state에 저장
- Context full-history/full-memory 무제한 materialize

### R4 — Resource Governor / Dynamic Core Scheduler

목표: 여러 Bot/Task execution을 자원·정책 한도 내에서 안전하게 병렬 실행한다.

작업:

1. process-wide Resource Governor를 Runtime composition에 연결한다.
2. global/scope/Bot concurrency, memory, cost, deadline, recovery headroom admission을 구현한다.
3. fair scheduling과 starvation 방지 정책을 기존 `DXB-DOM-024` 의미 안에서 구현한다.
4. `CoreLeaseId + ExecutionId + SchedulerGeneration + ResourceGrantRef` 기반 transient lease를 발급한다.
5. same Execution authoritative active Core Lease는 하나만 허용한다.
6. suspend/cancel/shutdown/provider drain에서 permit/activity를 반드시 release한다.

Gate:

- over-limit submit은 semantic work 시작 전 Deferred/Rejected
- stale scheduler generation result commit 불가
- leak test: cancellation/restart/shutdown 후 active permit 0
- bounded queue/item/byte cap

### R5 — Provider Activity → Result / Evidence / Side Effect

목표: admitted Execution이 실제 모델 호출을 수행하고 결과를 canonical owner에 안전하게 commit한다.

작업:

1. Scheduler가 Core Lease와 Provider activity를 함께 획득한다.
2. committed Context Plan으로 `TaskDescription/ProviderRequest`를 만든다.
3. deadline/cancellation/output cap을 Common Provider protocol에 전파한다.
4. Provider output을 bounded Result/Evidence로 정규화한다.
5. Application commit에서 ExecutionGeneration/TaskRevision/ProcessRevision을 fence한다.
6. Task Result, Process progress/outcome refs, receipt progress를 owner transaction으로 수렴한다.
7. external side effect가 있다면 Side Effect Ledger를 Prepared부터 연결한다.

Gate:

- real provider known prompt → non-empty Task Result
- response loss 후 duplicate result effect 0
- late provider result가 newer execution을 overwrite하지 않음
- deadline/cancel은 provider activity 종료 + lease release + typed terminal state
- output cap 초과가 partial success로 위장되지 않음

### R6 — Process recovery / restart continuation

목표: Runtime process가 죽어도 동일 Task/Process identity에서 안전하게 복구한다.

작업:

1. Process current step/activity/waiting condition/continuation ref를 실제 execution과 연결한다.
2. startup에서 nonterminal Task/Execution/Process/SideEffect/Approval을 reconcile한다.
3. transient Core Lease/provider session/timer를 복원하지 않고 canonical state에서 재등록한다.
4. uncertain provider/side-effect outcome은 RecoveryRequired/Unknown으로 두고 blind retry하지 않는다.
5. duplicate/late activity result를 idempotently ignore 또는 conflict로 기록한다.

Gate:

- kill/restart during admission, provider call, result commit 각각 재현
- same ProcessId/DefinitionVersion continuity
- no duplicate provider-visible side effect
- `task/process watch` reconnect/resync 정상

### R7 — Brain / Memory closed loop

목표: persistent Bot Identity/Memory가 실제 다음 Execution의 Context에 영향을 주되 execution-local state를 global Brain에 복제하지 않는다.

작업:

1. accepted/current Memory assertion retrieval을 Context Plan 후보에 연결한다.
2. scope/revision/provenance/epistemic/temporal/information-label을 canonical owner가 재검증한다.
3. index/cache는 revision/watermark/policy/schema에 결박된 Derived State로 둔다.
4. Provider Result/Evidence에서 필요한 지식은 typed Memory Proposal로 만들고 자동 Accepted 처리하지 않는다.
5. promotion/declassification/audit boundary를 유지한다.
6. context token/byte/item budget을 강제한다.

Gate:

- restart 뒤 same Bot Memory continuity
- stale/retracted/unauthorized memory가 current truth로 주입되지 않음
- index 삭제/rebuild 후 canonical Memory 동일
- large memory set에서도 bounded context construction

### R8 — Multi-Bot production delegation

목표: Bot 간 협업을 session fan-out이 아니라 durable recipient Task로 실행한다.

작업:

1. sender identity를 operator act-as 또는 current Execution에서 server-side derive한다.
2. membership/authority/scope/deadline/budget을 pre-accept 검증한다.
3. recipient Bot에 별도 durable Task/Execution intent를 만든다.
4. sender Process에는 child Task ref만 기록한다.
5. result/evidence는 typed ref로 join하며 recipient execution-local state를 복제하지 않는다.
6. duplicate delegation idempotency와 bounded fan-out을 적용한다.

Gate:

- planner→coder→reviewer 같은 3-Bot chain restart continuity
- recipient Provider failure가 sender Task state를 임의 overwrite하지 않음
- duplicate delivery로 recipient Task duplicate 0
- unauthorized act-as/scope crossing deny

### R9 — Operational storage / audit / service lifecycle

목표: 개인 로컬 및 장기 실행에서 데이터 유실·무제한 자원 사용·운영 불능을 막는다.

작업:

1. `DXB-ARC-018` operational storage profile을 실제 workload/failpoint로 재평가한다.
2. backup/snapshot/restore와 schema migration rehearsal을 추가한다.
3. disk-full/compaction/retention에서 nonterminal recovery state를 보존한다.
4. execution/provider/security-sensitive action의 required AuditIntent/outbox를 보장한다.
5. Runtime Host supported service entrypoint와 supervisor integration 원칙을 고정한다.
6. graceful shutdown에서 admission stop → active activity drain/cancel policy → checkpoint → endpoint unpublish 순서를 검증한다.
7. `runtime status/doctor`에 control/provider/storage/audit/resource/recovery health를 bounded projection한다.

## 7. Operational End-to-End Journeys

최종 운영 가능 판정은 component test가 아니라 아래 built-binary journey를 통과해야 한다.

### OJ-001 — First real useful task

```text
dxb runtime start
→ provider ready 확인
→ dxb bot create main
→ dxb bot activate main
→ conversation send
→ task submit
→ Task Admitted/Running
→ Context Plan
→ Core Lease
→ real Provider
→ Task Result/Evidence commit
→ task watch/result
```

성공 조건: ReferenceProvider 없이 실제 configured Provider가 non-empty result를 만들고 Operation/Task/Process/Execution/Provider generation을 추적할 수 있다.

### OJ-002 — Restart continuity

```text
Bot/Task/Process 실행 중 Runtime kill
→ Runtime restart
→ same Instance/Bot/Task/Process identity restore
→ nonterminal reconcile
→ safe resume/retry decision
→ terminal result
```

### OJ-003 — Provider failure matrix

- unconfigured
- bad credential
- transport unavailable
- rate limited
- upstream 5xx
- deadline exceeded
- output exceeded
- provider replacement generation race

각 실패는 typed error/outcome, no secret leak, no blind retry, no lease leak를 만족한다.

### OJ-004 — Memory continuity

Task A result → Memory Proposal → authorized promotion → Runtime restart → Task B Context Plan에서 동일 accepted revision을 bounded retrieval한다.

### OJ-005 — Multi-Bot collaboration

Project/Channel membership → sender Task → recipient durable delegated Task → independent Execution/Core Lease → result ref join → restart 후 동일 graph 유지.

### OJ-006 — Recovery / side effect

Provider 또는 외부 side effect response loss → Unknown/RecoveryRequired → operation/side-effect reconcile → duplicate external effect 0.

### OJ-007 — Long-running operation

Runtime을 supervisor 아래 반복 restart하면서 Task/Process/Memory/Approval/Audit continuity, disk/resource ceiling, graceful drain을 검증한다.

## 8. Verification Matrix

| Layer | 필수 증거 |
|---|---|
| Contract/CLI | 63 registry snapshot, R0 five residual binary regressions |
| Provider Common | AT-PROVIDER-INFRA-001~008 |
| Real Provider | AT-DEEPSEEK-001~005 또는 동일 Stable Contract를 만족하는 production adapter evidence |
| Application | Task/Execution/Context Plan/Result UoW, stale CAS/generation, duplicate result |
| Scheduler | admission/fairness/generation fence/leak/resource-pressure tests |
| Process | crash/restart/wait/resume/late result |
| Memory | provenance/revision/index rebuild/context budget/security |
| Delegation | recipient Task identity/idempotency/scope/act-as |
| Side Effect | Prepared/Unknown/reconcile/no blind retry |
| Storage | crash/disk-full/snapshot/backup-restore/migration |
| Audit/Security | required intent, approval, authority, redaction, secret canary |
| Binary | OJ-001~007 built `dxb` subprocess/fault journeys |

최종 quality 명령은 최소 다음을 포함한다.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

GitHub Actions 실행을 증거로 사용하지 않는다. Provider live test가 외부 endpoint/credential을 요구하면 local explicit test profile에서 실행하고 secret은 artifact/log에 기록하지 않는다.

## 9. Stop Conditions

다음 상황에서는 편의 구현을 멈추고 Canonical Owner 문서/ADR을 먼저 갱신한다.

1. 새로운 public CLI command가 필요하다고 판단되는 경우
2. Provider registry/credential/config authority가 불명확한 경우
3. Task와 Execution을 같은 mutable aggregate로 합쳐야만 구현되는 경우
4. BotId를 PrincipalRef로 변환해야만 delegation/authority가 작동하는 경우
5. Core Lease 또는 Provider session을 durable Bot/Brain state에 저장하려는 경우
6. ReferenceProvider를 production 장애 fallback으로 사용하려는 경우
7. Provider callback이 Application canonical state를 직접 mutate하려는 경우
8. Memory index/cache가 canonical assertion state를 소유하려는 경우
9. Unknown side effect를 blind retry하려는 경우
10. unbounded queue/history/context/provider output를 허용해야만 기능이 동작하는 경우
11. storage 제품 선택을 evidence 없이 public contract로 freeze하려는 경우
12. executable evidence 없이 M5/M6 또는 Operational Ready를 선언하려는 경우

## 10. Operational Ready Definition

다음 조건을 **모두** 만족해야 `DXBOT Operational Ready`를 선언한다.

- [ ] BF-OR-CLI-001~005 폐쇄 및 binary regression PASS
- [ ] BF-OR-DOC-001 정합성 수정
- [ ] production Provider config/credential/registration/readiness owner 폐쇄
- [ ] ReferenceProvider 없는 real Provider E2E PASS
- [ ] explicit Execution identity + immutable Context Plan 구현
- [ ] Resource Governor + Dynamic Core Lease Scheduler 구현
- [ ] Provider result/evidence → Application atomic commit 구현
- [ ] Process restart/recovery 실제 activity checkpoint와 연결
- [ ] Memory retrieval/promotion → next Context Plan closed loop PASS
- [ ] recipient durable Task 기반 Multi-Bot delegation PASS
- [ ] Side Effect Ledger producer/reconcile PASS
- [ ] operational storage backup/restore/migration/disk-pressure evidence PASS
- [ ] required AuditIntent/redaction/secret canary PASS
- [ ] OJ-001~007 built-binary journey PASS
- [ ] workspace fmt/clippy/test PASS
- [ ] Recheck 1 Structural / Consistency PASS
- [ ] Recheck 2 Cross-Layer Executability PASS

## 11. Recheck 1 — Structural / Consistency

완료 직전 독립적으로 다음을 검수한다.

- file/path naming과 crate/module placement
- crate-per-concept 도입 여부
- Task/Execution/Process/Core/Provider/Memory owner 중복 여부
- Provider configuration과 credential secret duplication 여부
- policy/default/generation의 single source
- Application/Security/Audit/SideEffect transaction 관계
- snapshot/schema/migration/backup 관계
- `DXB-DEL-060/061` milestone/status 정합
- CLI registry/contract와 R0 변경 관계
- docs/test/config/schema link와 stale reference

발견 사항 수정 후 같은 범위를 다시 통과해야 완료다.

## 12. Recheck 2 — Cross-Layer Executability

같은 파일을 다시 읽는 방식이 아니라 실제 흐름을 다음 순서로 추적한다.

`CLI Input → Control/Auth → Application Task → Admission → Context Plan → Execution → Resource Governor → Core Lease → Provider Host → Real Provider → Result/Evidence → Process/Memory/SideEffect → Persistence/Audit → Recovery → Projection/CLI`

각 흐름에 다음 조건을 대입한다.

- Provider none/bad credential/unavailable/replaced
- stale TaskRevision/ExecutionGeneration/SchedulerGeneration/ProviderGeneration
- deadline/cancel/SIGINT/broken pipe
- Runtime crash before/during/after provider activity
- response loss before/after result commit
- duplicate/late provider result
- Memory index loss/rebuild/stale assertion
- delegation duplicate/unauthorized act-as/scope crossing
- audit/storage disk pressure
- backup/restore and schema migration
- graceful shutdown/drain/restart

L4/L5에 해당하는 built `dxb` journey까지 통과해야 Recheck 2를 완료한다.

## 13. Initial Evidence Ledger

| 날짜 | 기준 | Evidence | 판정 |
|---|---|---|---|
| 2026-08-29 | GitHub main `2ef238af...` | `DXB-DEL-066` historical closeout evidence 확인 | CLI execution spine evidence 보존 |
| 2026-08-29 | `cli/src/runner.rs`, `cli/src/discovery.rs`, `application-contract` registry | version remote compatibility, runtime-start `ready_at`, human render, human next-action, unknown suggestion 재감사 | BF-OR-CLI-001~005 Open |
| 2026-08-29 | `runtime-host/src/local_runtime.rs` | empty `ProviderHost::new()` composition + discovery unconfigured | BF-OR-RUN-001 Open |
| 2026-08-29 | `application/src/mutation.rs`, `state.rs` | Task Pending + Process Running 생성, explicit production Execution/Core/Provider result coordinator 부재 | BF-OR-RUN-002~006 Open |
| 2026-08-29 | `provider-host` | Common HTTP/protocol/RealProvider/DeepSeek adapter 존재, production host composition 미연결 | 기반 존재 / R2 필요 |
| 2026-08-29 | `DXB-DEL-060/061` | M5 provider acceptance 13개 Executable vs AT-SCHEMA evidence "all prior milestones passed" | BF-OR-DOC-001 Open |

## 14. 완료 순서 요약

```text
R0 CLI residual closure
→ R1 evidence/gate consistency
→ R2 production Provider composition
→ R3 Task/Execution + Context Plan
→ R4 Resource Governor + Dynamic Core
→ R5 real Provider result/evidence commit
→ R6 crash/restart Process recovery
→ R7 Brain/Memory closed loop
→ R8 Multi-Bot durable delegation
→ R9 storage/audit/service operationalization
→ OJ-001~007 + two final rechecks
→ DXBOT Operational Ready
```

우선순위의 핵심은 **Provider를 먼저 연결하고, 그 다음 Task를 실제 Execution으로 승격한 뒤 Scheduler/Core를 붙이는 것**이다. Memory와 Multi-Bot을 먼저 확장하면 실제 실행 spine이 없는 상태에서 canonical state와 orchestration 복잡도만 커지므로 금지한다.

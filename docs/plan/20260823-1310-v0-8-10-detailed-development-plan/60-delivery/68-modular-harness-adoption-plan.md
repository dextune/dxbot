---
title: "MiniMax M3 기반 공식 DeepSeek Harness 모듈화 후속 개발 플랜"
document_id: "DXB-DEL-068"
version: "0.8.10"
status: "Accepted"
normative: false
priority: "P0"
last_updated: "2026-08-30"
depends_on: ["DXB-GOV-003", "DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-017", "DXB-ARC-019", "DXB-RUN-031", "DXB-RUN-032", "DXB-RUN-033", "DXB-RUN-034", "DXB-RUN-035", "DXB-RUN-038", "DXB-DEL-064", "DXB-DEL-067"]
---
# MiniMax M3 기반 공식 DeepSeek Harness 모듈화 후속 개발 플랜

## 1. 목적과 판정

이 문서는 `DXB-DEL-067`의 local P0 Operational Runtime 이후 Provider/Harness 구조를 **교체·추가·제거 가능한 production framework**로 수렴시키는 비규범 실행 플랜이다. 구현 완료를 선언하지 않으며 단계, gate, 위험, 중단 조건과 executable evidence를 소유한다.

고정 결정은 다음 둘이다.

1. production LLM 모델은 **`MiniMax-M3`** 이며 다른 모델로 변경하거나 silent fallback하지 않는다.
2. 공식 [DeepSeek Harness repository](https://github.com/deepseek-ai/deepseek-harness)를 DXBOT의 공식 Harness adapter 대상으로 추가한다. immutable tag/commit/license/toolchain snapshot의 Single Source of Truth는 [DXB-DEL-064](64-external-reference-snapshot.md)다.

**현재 판정: Plan Accepted / DXB-DEL-068 구현 범위 H0–H11 Implemented and Verified.** exact upstream pin/build, canonical Execute/factory/registration, official DSH ACP subprocess, exact `MiniMax-M3` route, RuntimeComposition/lifecycle/security/effect/resource bridge, shared Conformance와 adapter-removal matrix가 구현됐다. pinned official DSH→MiniMax-M3 canonical live test와 DSH-only built `dxb`의 tool-free/workspace-read task가 non-empty result, exact `deepseek-harness-acp` evidence, child process 0, graceful stop으로 통과했고, 전체 workspace quality gates와 3회 독립 review에서 open High/Medium 0을 확인했다. 이는 이 delivery task의 implementation gate 판정이며 **Harness Modularization Complete를 선언하지 않는다**. HJ-002~011 전체 built-CLI operational campaign, license/SBOM·upgrade rehearsal 등 아래 unchecked Operational Complete 항목은 별도 후속 evidence가 필요하다.

## 2. 용어와 경계

| 용어 | 이 문서의 의미 |
|---|---|
| DXBOT Provider | `ProviderHost`에 등록되어 canonical Execute 계약을 수행하는 교체 가능 실행 구현 |
| Harness adapter | 외부 agent harness를 DXBOT Provider 계약으로 변환하는 compatibility boundary |
| DeepSeek Harness 또는 DSH | 공식 Node/Cordis 기반 외부 제품; DXBOT Core/Brain/Bot과 동일하지 않음 |
| ACP | DSH의 automation-only ACP v1 JSON-RPC stdio surface |
| direct MiniMax adapter | 현재 Anthropic-compatible Messages를 직접 호출하는 migration/reference implementation |
| canary | production fallback이 아닌 test-only 계약 검증 구현 |
| Provider generation | 등록·교체·drain fencing을 위한 DXBOT runtime identity |
| DSH session id | 외부 execution trajectory 식별자; BotId/TaskId/ExecutionId/ProcessId가 아님 |

DSH의 Session, Agent, tool, storage, approval 의미를 DXBOT Domain public schema에 복제하지 않는다. ACP wire type도 DXBOT Canonical Contract가 아니다. 모든 변환은 adapter 내부에서 끝난다.

## 3. Baseline과 열린 gap

현재 기반은 Common HTTP transport, OpenAI Chat Completions/Anthropic Messages protocol, direct DeepSeek/MiniMax adapters, real Provider execution, Runtime config와 built CLI E2E까지 동작한다. 그러나 범용 Harness framework로는 다음이 열려 있다.

1. `HarnessAdapter`가 외부 Harness가 아닌 synthetic test canary다.
2. `ReferenceProvider`, canary, `RealProvider`가 서로 다른 실행 계약을 사용한다.
3. `ProviderHost`가 여러 real provider를 보관하지만 protocol slot은 하나다.
4. runtime config가 concrete adapter 이름을 `match`한다.
5. `RuntimeComposition`이 실제 `LocalRuntimeHost` startup owner가 아니다.
6. register 이후 drain/replace/unregister lifecycle이 없다.
7. rich canonical Harness request/event와 실제 `TaskDescription/TaskResult` SPI가 수렴하지 않았다.
8. production adapter 전체에 동일 Conformance suite가 적용되지 않는다.
9. adapter source/config/feature 제거 후 workspace build 증거가 없다.
10. `ProviderHost`가 자체 Tokio runtime과 synchronous `block_on`을 소유해 host lifecycle/cancellation과 분리돼 있다.

## 4. Canonical Ownership Matrix

| 의미 | Canonical Owner | 금지 |
|---|---|---|
| Bot/Task/Execution/Process/Result | Application/Domain owners | DSH Session/Agent를 Domain identity로 승격 |
| immutable Context Plan | Application Context Plan owner | adapter가 history/memory를 임의 확장 |
| ExecuteRequest/Event/Result | `DXB-ARC-012/013/019`을 갱신한 Shared Provider Contract | ACP/DSH type re-export |
| registration/generation/drain | `provider-host` + Runtime lifecycle contract | factory 또는 adapter가 Domain state 직접 mutate |
| factory inventory/config decoding | Runtime composition/config owner | concrete adapter hardcoded `match` 분산 |
| protocol/transport | 각 immutable Provider registration | host-wide mutable protocol singleton |
| model route | deployment config + Provider registration | MiniMax-M3 이외 모델 선택 또는 자동 fallback |
| credential reference/value | config reference / Security credential resolver | raw key를 patch, Domain, log, evidence에 저장 |
| approval/authority | DXBOT Security/SideEffect owner | DSH approval을 최종 권한 근거로 신뢰 |
| external effect disposition | Side Effect Ledger | process crash 후 blind retry |
| DSH session/log | adapter-owned external evidence | Bot Memory 또는 Process canonical state로 사용 |
| upstream pin/license/integrity | `DXB-DEL-064` | `latest`, unpinned `npx`, 중복 hash owner |

Canonical Execute schema 변경은 먼저 architecture owner 문서를 갱신하고 code/schema/config/conformance를 같은 change unit에서 수렴시킨다.

## 5. 고정 통합 구조

### 5.1 Process boundary

공식 DSH는 Rust process에 Cordis/Node를 embed하지 않는다. DXBOT adapter가 exact-pinned executable을 shell 없이 다음 형태로 spawn한다.

```text
DXBOT Runtime
  → ProviderHost / DeepSeekHarnessAcpProvider
  → managed child: dsh --profile acp --patch <DXBOT-owned audited patch>
  → ACP v1 newline-delimited JSON-RPC over stdin/stdout
  → DSH agent loop/tools
  → @deepseek-ai/dsh-llm-pi-ai
  → MiniMax anthropic-messages endpoint / MiniMax-M3
```

stdout은 ACP frame 전용이다. stderr는 protocol과 분리하고 bounded/redacted diagnostic으로만 수집한다. ACP `authenticate`의 immediate success는 보안 인증이 아니므로 local child ownership, scrubbed environment, absolute workspace, OS isolation과 DXBOT authority가 trust boundary다.

P0는 **Execution attempt마다 새 process와 새 DSH session**을 사용한다. process pooling, 한 connection의 multi-session multiplex, 자동 `session/resume`은 비용/복구/격리 evidence 전까지 금지한다. 이 선택은 공식 `dsh-subagent-acp`의 fresh-process-per-run pattern과 정렬한다.

### 5.2 MiniMax M3 route

DXBOT-owned audited profile patch는 공식 `@deepseek-ai/dsh-llm-pi-ai` plugin의 hand-declared route를 사용한다.

```yaml
providers:
  minimax:
    apiKeyEnv: MINIMAX_API_KEY
    api: anthropic-messages
    baseURL: https://api.minimax.io/anthropic
    models:
      - id: MiniMax-M3
```

ACP session의 provider/model은 `minimax` / `MiniMax-M3`로 고정하고 session publication 전 advertised configuration과 exact selection을 검증한다. credential은 DXBOT의 `env:MINIMAX_API_KEY` reference에서 child의 scrubbed environment로 해당 변수 하나만 주입한다. patch와 diagnostics에는 raw value가 없어야 한다.

정확한 context/output/reasoning capability 값은 추측하지 않는다. pinned DSH catalog와 live MiniMax probe가 일치하는 값을 H7에서 freeze한다. 공식 `dsh-llm-pi-ai`가 required MiniMax semantics를 만족하지 못하면 구현을 멈춘다. 다음 선택은 DXBOT-owned out-of-tree DSH plugin/patch에 대한 별도 review이며 upstream fork나 모델 변경이 아니다.

### 5.3 Tool와 security boundary

DSH가 model-generated command/code, file, process, network, MCP에 접근할 수 있다는 사실을 정상 위험으로 취급한다.

- DXBOT Security/Resource Governor/SideEffect Ledger가 최종 권한·자원·effect owner다.
- DSH permission request는 one-shot DXBOT approval decision으로 map하며 unattended default는 reject다.
- DSH sandbox/approval은 defense in depth일 뿐 DXBOT 권한 증거가 아니다.
- child는 assigned workspace, explicit executable set, bounded network policy와 least-privilege identity에서 실행한다.
- ambient credential, user DSH home, SSH/cloud/package-manager credential은 전달하지 않는다.
- client supplied MCP command/env/URL/header를 production ExecuteRequest가 직접 주입하지 못한다.
- tool side effect 전 Ledger `Prepared`, dispatch 후 `Dispatched`, 확인 후 `Confirmed/Failed/Unknown`을 유지한다.

### 5.4 Spawn-time asset 무결성 recheck (composition→spawn TOCTOU 축소)

Runtime composition은 secret resolution 전에 on-disk `command`/`patch`의 digest·mode·symlink·크기를 검증하고, patch가 compile-time audited digest와 일치하는지 확인한다. 그러나 composition과 실제 `Command::new` spawn 사이에는 시간 간격이 있어 asset이 교체·변조될 수 있다. 이를 축소하기 위해 provider는 **모든 spawn 직전에**(live execution path에서) asset을 다시 검증한다.

- `command`와 `patch`를 `symlink_metadata`로 다시 열어(symlink follow 금지) direct regular file, 올바른 mode(command owner-execute, 둘 다 group/other-writable 금지), bounded size를 재확인하고, SHA-256을 streaming으로 다시 계산해 설정 digest와 **constant-time** 비교한다.
- patch digest는 추가로 **canonical audited digest**와 비교한다. 이 audited digest는 Runtime config owner(`runtime-host`)가 소유하는 단일 값이며 provider로 그대로 전달된다. provider가 audited digest를 독립적으로 재계산하는 second owner는 없다(중복 금지).
- 두 asset의 모든 상위 directory를 walk해 **trusted barrier**에 도달할 때까지 symlink 또는 group/other-writable ancestor를 거부한다. barrier의 정확한 규칙은 (1) group/other-writable가 아닌 ancestor chain을 통해 filesystem root에 도달하거나, (2) effective uid가 소유한 owner-only(`0700`-equivalent, group/other bit 없음) directory 중 하나다. 이 규칙은 sticky world-writable `/tmp` 아래 `mkdtemp` `0700` root를 유효한 barrier로 허용하면서(그 위의 `/tmp`는 검사하지 않음), barrier 아래의 group/other-writable ancestor는 거부한다.
- 모든 위반은 path·digest·mode·size·uid를 노출하지 않는 단일 category-only `ExecuteError`로 fail-closed한다.

**Limitation (정직한 명시):** 이 recheck는 **path 기반**이며 kernel `fd`-bound execution이 아니다. composition→spawn TOCTOU window를 크게 축소하고 `symlink_metadata`로 관측 가능한 leaf swap과 writable-ancestor swap을 막지만, binding을 provably race-free로 만들지는 **않는다**. 최종 `symlink_metadata`/hash와 kernel의 `execve` resolution 사이 남은 micro-race를 정밀하게 이기는 공격자는 여전히 content를 치환할 수 있다. 따라서 `command`/`patch`와 그 barrier까지의 ancestor directory가 **immutable, root-owned deployment path**(service uid·group·other가 쓸 수 없음)에 있어야 한다는 배포 요구는 유효하며 선택 사항이 아니다. path 기반 recheck는 그 요구 위의 defense in depth이지 대체물이 아니다.

## 6. 목표 Execute 계약

H2에서 architecture owner와 code를 함께 갱신해 reference/canary/direct/DSH production 경로가 하나의 async contract를 구현하게 한다. 이름은 owner review에서 확정하되 의미는 다음을 포함한다.

### ExecuteRequest

- `ExecutionRef`, attempt/generation, immutable Context Plan ref/snapshot
- selected ProviderId/ProviderGeneration/capability and exact `MiniMax-M3`
- bounded task content and approved workspace mapping
- deadline/cancellation, resource grant, permission/side-effect policy refs
- input/output/event/frame limits and correlation/audit refs
- no raw credential, no mutable Domain handle, no DSH session identity

### ExecuteEvent

- monotonic adapter-local sequence and ProviderActivityRef
- admitted/started/progress/assistant-output/tool-lifecycle/usage/warning/terminal categories
- bounded provider-neutral payload; raw ACP/provider delta는 adapter 내부에 유지
- tool/effect events carry correlation and disposition, not authority invention
- at most one terminal event; no event after terminal

### ExecuteResult

- typed outcome: succeeded/rejected/cancelled/timed-out/failed/recovery-required
- bounded final output and EvidenceRefs
- usage/resource facts, ProviderId/Generation/ActivityRef
- side-effect disposition including `Unknown`
- safe diagnostic category and optional opaque external session evidence ref
- no provider exception text, stderr, path, prompt, secret, raw protocol payload

실행 API는 Runtime의 async lifecycle을 사용하고 ProviderHost가 소유하던 Tokio runtime과 그 nested `block_on`을 제거한다. OS-thread 기반 Scheduler가 async Provider SPI를 호출하는 유일한 동기 bridge는 Runtime Host가 소유한 `tokio::runtime::Handle::block_on`이며, adapter/ProviderHost는 별도 runtime을 생성하지 않는다. cancellation, timeout, backpressure와 shutdown은 동일 Runtime-owned task tree에서 전파한다.

## 7. 개선사항 1~9 Traceability

| # | 개선사항 | 주 단계 | 완료 증거 |
|---|---|---|---|
| 1 | `HarnessAdapter` test-only canary 정리 | H1 | production feature/source graph에서 canary 0, testkit에서만 사용 |
| 2 | reference/canary/production 실행 계약 통합 | H2~H3 | 같은 trait와 Conformance suite |
| 3 | Provider별 protocol binding | H4 | 동시 Anthropic/ACP registration isolation test |
| 4 | hardcoded match → static ProviderFactory registry | H5 | registry-derived config parse/list/build test |
| 5 | RuntimeComposition → LocalRuntimeHost lifecycle | H8 | built runtime startup/shutdown가 composition 하나만 사용 |
| 6 | drain/replace/unregister | H9 | generation race, drain timeout, no-leak tests |
| 7 | canonical ExecuteRequest/Event/Result와 SPI 수렴 | H2 | owner docs/schema/code/conformance 동시 diff |
| 8 | 모든 production adapter 동일 Conformance | H3, H11 | adapter matrix 전체 PASS |
| 9 | adapter 제거 build | H11 | DSH/direct/canary 각각 제거 variant build PASS |

## 8. Development Stages H0~H11

각 단계는 선행 gate를 통과한 뒤 진행한다. gate evidence 없이 다음 단계 완료로 표시하지 않는다.

### H0 — Decision, pin, license, supply-chain baseline

**선행:** 본 plan Accepted.
**변경:** `DXB-DEL-064`에 repository/tag/commit/version/MIT/Node/pnpm/developer-preview 경고를 단일 snapshot으로 기록한다. release input은 exact package version, lockfile integrity, transitive SBOM, license notice와 executable digest를 가진다. upgrade는 별도 PR과 conformance/fault review다.
**금지:** `npx latest`, floating branch, runtime install, mutable global `~/.dsh`, package postinstall을 무검토 허용.
**Gate:** clean environment에서 offline reproducible install/build; runtime evidence가 exact DSH version/digest를 secret 없이 출력; license/SBOM scan PASS.

### H1 — Synthetic canary 격리

**선행:** H0.
**변경:** 현재 `HarnessAdapter`를 `TestCanaryProvider` 등 오해 없는 이름으로 이동하고 `cfg(test)`/test-support surface에 한정한다. `ProviderHost.adapter` special slot을 제거한다. `ReferenceProvider`도 explicit test fixture로 분류한다.
**금지:** canary/reference를 production ProviderUnavailable fallback으로 사용; production config allowlist에 노출.
**Gate:** production dependency graph 및 built `dxb` symbol/config에서 canary 0; test canary는 공통 Execute contract regression PASS.

### H2 — Canonical async Execute contract 수렴

**선행:** H1, architecture owner review.
**변경:** §6 계약을 canonical owner에 반영하고 `TaskDescription/TaskResult`, `RealProvider`, reference/canary path를 하나의 async SPI로 수렴한다. immutable request, bounded event stream, one terminal, generation/deadline/cancel semantics를 type/test로 닫는다. host-owned runtime을 사용한다.
**금지:** ACP DTO re-export, Provider/adapter-owned runtime과 nested `block_on`, Runtime scheduler boundary 밖의 sync bridge, callback의 Domain mutation, adapter별 별도 terminal vocabulary.
**Gate:** compile-time single SPI; cancellation/timeout/output overflow/late event tests; old duplicate trait 및 host-local runtime 제거.

### H3 — Unified Conformance Testkit

**선행:** H2.
**변경:** 모든 adapter에 같은 black-box suite를 parameterize한다. deterministic fake transport/process/clock/credential/workspace를 제공하고 production adapter가 test-only shortcut 없이 같은 public registration path로 진입한다.
**금지:** adapter 이름별 assertion 분기, live network만으로 correctness 입증, secret-bearing fixture.
**Gate:** canary, direct MiniMax migration adapter, DSH adapter skeleton이 공통 matrix를 통과하며 새 production factory는 suite 등록 없이는 compile/CI gate 실패.

### H4 — Per-registration protocol/transport ownership

**선행:** H2~H3.
**변경:** registration이 immutable `ProtocolBinding + TransportBinding + Generation + Limits`를 소유한다. OpenAI Chat, Anthropic Messages, ACP v1 stdio는 서로 다른 binding이다. in-flight call은 captured registration을 끝까지 사용한다.
**금지:** host-wide `Option<ChatCompletionProtocol>`, replace 중 protocol mutation, adapter가 다른 registration transport 참조.
**Gate:** Anthropic direct와 ACP DSH 동시 등록/실행/교체에서 cross-route 0; old generation call이 old binding으로 종료하고 new call만 new binding 사용.

### H5 — Static ProviderFactory registry

**선행:** H4.
**변경:** compile-time factory inventory가 adapter key, config schema/version, capability, constructor와 conformance hook을 등록한다. runtime config는 registry를 조회하고 factory가 완전한 candidate registration을 만든 뒤 atomic publish한다.
**금지:** `provider_config.rs` concrete `match`, runtime dynamic native ABI, reflection/dlopen, unknown key fallback.
**Gate:** duplicate key compile/startup failure, unknown key typed failure, registry-derived list/schema tests, factory 하나 제거 시 unrelated factory build 유지.

### H6 — Official DSH ACP subprocess adapter

**선행:** H0, H2~H5.
**변경:** shell 없이 exact executable/args/cwd/env를 spawn한다. initialize→capability/version verify→authenticate semantic 확인→session/new→prompt/update→close 순서를 state machine으로 구현한다. request id/session id를 execution-local table에 보관하고 stdout/stderr를 분리한다. process tree 종료는 EOF grace→TERM→KILL 및 reap까지 bounded다.
**금지:** Cordis in-process embed, stdout logging 허용, shared user DSH home, process pooling, automatic resume, arbitrary MCP injection.
**Gate:** deterministic fake ACP server로 spawn/handshake/update/result/cancel/close/crash/malformed/oversized/contaminated stdout matrix PASS; child/grandchild leak 0.

### H7 — MiniMax M3 profile/credential composition

**선행:** H6.
**변경:** §5.2의 official `dsh-llm-pi-ai` anthropic-messages route와 ACP config를 audited patch로 생성한다. route/model을 session publication 전 exact match하고 `MINIMAX_API_KEY`만 scrubbed child env에 materialize한다. model/tool-call/cancellation/usage capability를 pinned build와 live canary로 확인한다.
**금지:** 다른 모델 fallback, raw key in YAML/argv/log, ambient provider discovery, unverified context/output 숫자, user settings가 route를 바꾸는 경로.
**Gate:** deterministic 401/protocol/empty/output-bound tests; exact model response와 tool-call round trip live canary; diff/artifact secret scan; route drift 시 fail closed.

### H8 — RuntimeComposition production wiring

**선행:** H5~H7.
**변경:** `LocalRuntimeHost`가 Application/Security/Audit/Provider/Execution/Control을 직접 중복 생성하지 않고 `RuntimeComposition`의 single owner graph를 소비한다. startup rollback은 역순 disposal, shutdown은 admission stop→execution cancel/drain→provider drain→persistence/audit flush→control unpublish 순서다.
**금지:** test-only composition과 production composition 이중화, partially published Provider, readiness 선행 publish.
**Gate:** built `dxb runtime start/status/doctor/stop`가 같은 composition generation을 관측; 각 startup failpoint에서 published endpoint/process/permit 0.

### H9 — Register/replace/drain/unregister/recovery lifecycle

**선행:** H8.
**변경:** candidate build/health가 끝난 뒤 atomic register/replace한다. `ProviderHost`는 하나의 `Arc<ProviderHost>`로 공유되고 register/replace/begin_drain/unregister/reclaim/drain API는 모두 `&self`로 private `RwLock` slot registry를 동기적으로 변경한다. execute admission은 registry lock 안에서 `Ready`+exact generation을 관측하고 per-slot RAII activity lease(고유 activity id로 in-flight `CancellationToken` 추적)를 원자적으로 획득한 뒤 provider를 clone하고 lock을 해제한 다음 `Handle::block_on`을 호출한다(host/slot lock을 block_on 위에서 보유 금지). replace 시 new generation을 하나의 write lock으로 publish하고 기존 `Ready`를 no-new-admission `Draining`으로 전환하며, zero-lease draining slot은 즉시 완전히 reclaim되어 선택 가능한 old generation을 남기지 않는다(이미 provider+activity를 잡은 in-flight만 완료 가능, 이후 시작 호출은 fencing). unregister도 같은 drain 경로를 쓰고 zero-lease generation을 즉시 제거한다. `drain_to_quiescence`와 all-provider `drain_all_to_quiescence`는 bounded deadline을 쓰고 timeout 시 추적 token을 cancel하며 unbounded-wait하지 않는다. `LocalRuntimeHost`는 `Arc::get_mut`/no-op 없이 공유 host의 real drain/reclaim을 직접 호출하고 verified zero lease 이후에만 provider-drained를 보고하며, 아니면 cancel/reap window 후 shutdown error를 반환한다. late event/result는 generation fence한다.
**금지:** gap 있는 unregister→register, in-flight registration mutation, `&mut ProviderHost`/`Arc::get_mut` 기반 lifecycle, 선택 가능한 zero-lease old generation 유지, force removal 후 orphan child, stale result commit.
**Gate:** Arc를 실제로 공유하고 execute가 park된 동안 replace/unregister하는 deterministic test에서 old admission fence·new generation 동작·old result commit fence·host-triggered cancellation 관측·slot reclaim·stress no-leak PASS; concurrent execute/replace/unregister stress, cancellation race, drain timeout, runtime restart reconciliation에서 duplicate result/effect/process leak/permit leak 0.

### H10 — Tool, approval, side-effect, resource bridge

**선행:** H7~H9.
**변경:** ACP permission/tool lifecycle을 provider-neutral events와 DXBOT approval/effect refs로 map한다. malformed/unattributed tool event는 실행하지 않는다. process CPU/memory/PID/file/network limits와 queue/frame/output bounds를 적용한다.
**금지:** first-allow 자동 승인, DSH sandbox를 유일 control로 간주, unknown effect 자동 retry, model text를 authority로 해석.
**Gate:** reject/allow-once/cancel/timeout/crash-before-during-after-effect matrix; Unknown→RecoveryRequired; duplicate external effect 0; sandbox escape/ambient credential canary 0.

### H11 — Removal build, built journeys, final reviews

**선행:** H0~H10.
**변경:** adapter별 source/config/factory/test fixture를 제거한 build variants를 검증한다. direct MiniMax adapter는 DSH parity와 rollback window 종료 후 migration reference 또는 제거 대상으로 판정하되 production silent fallback으로 남기지 않는다. built CLI journey와 fault/backpressure/upgrade rehearsal을 수행한다.
**금지:** `--all-features`만 통과하고 removal variant 생략, live success 하나로 완료 선언, review finding 미수정.
**Gate:** §12~§16 전체 evidence와 독립 review 2회 PASS.

## 9. ACP Lifecycle과 Failure/Recovery Matrix

Adapter state는 최소 `Created → Spawned → Initializing → SessionOpening → Ready → Running → Cancelling/Draining → Closed`와 terminal `Failed`를 구분한다. session publication 전 failure는 Provider activity를 공개하지 않고 child를 reap한다. publication 후 failure는 partial output/evidence를 보존할 수 있으나 success로 위장하지 않는다.

| Failure | Required outcome | Recovery rule |
|---|---|---|
| executable/spawn 실패 | typed Unavailable, no session publication | config/package 수정 전 retry 금지 |
| initialize/version/capability mismatch | Incompatible, child reap | pin/adapter compatibility review |
| authenticate behavior drift | Incompatible | ACP auth를 DXBOT auth로 간주하지 않음 |
| stdout contamination | ProtocolViolation | session fail, stderr와 혼합 복구 금지 |
| malformed/oversized frame | ProtocolViolation/OutputExceeded | bounded diagnostic, child cancel/reap |
| unknown response id/session id | ProtocolViolation | association 추측 금지 |
| stalled read/prompt | TimedOut | cancel grace 후 process-tree kill/reap |
| local cancellation | Cancelled | `session/cancel`, drain, close; no failure retry |
| DSH crash before tool dispatch | Failed | side-effect evidence 없을 때만 새 attempt 가능 |
| crash during/after possible effect | RecoveryRequired + Unknown | reconcile 전 blind retry 금지 |
| committed final update 후 process crash | result commit fence로 판정 | 이미 commit됐으면 duplicate effect 0; 미commit이면 evidence reconcile |
| Runtime crash | nonterminal reconcile | P0 auto resume 금지; same Task의 explicit new attempt |
| session/list/resume drift | compatibility failure | P0 execution semantics에 사용하지 않음 |
| child close/kill 실패 | RecoveryRequired | whole-tree quiescence를 거짓 주장하지 않음 |

`session/resume` 지원은 compatibility probe로 테스트할 수 있으나 P0 recovery owner가 아니다. 향후 resume 도입은 same exact pin/profile/workspace/Execution binding과 side-effect reconciliation을 증명하는 별도 decision을 요구한다.

## 10. Resource Bounds와 Backpressure

모든 bound는 positive finite default와 deployment maximum을 가지며 config owner 한 곳에서 resolve한다. 숫자는 H10 fault/load evidence로 freeze하고 adapter별 hidden default를 금지한다.

- max concurrent DSH processes and sessions per Runtime/Bot/Provider
- max queued executions and per-execution pending events
- max JSON-RPC line/frame bytes, nesting depth와 string/array cardinality
- max task/context/prompt bytes and ACP update/output/result bytes
- max stderr retained bytes and safe diagnostic bytes
- spawn/initialize/session/prompt/idle/cancel/drain/kill deadlines
- process CPU, memory, PID, open-file, disk/temp와 network policy
- tool concurrency, tool input/result bytes, background child count
- Evidence/session-log retention bytes와 cleanup watermark

stdout reader는 bounded decoder와 bounded channel을 사용한다. consumer가 느리면 read-side backpressure 또는 explicit overflow failure를 발생시키며 unbounded buffer로 전환하지 않는다. terminal/result delivery를 위해 progress를 무한 보존하지 않고 documented coalescing/drop policy를 적용하되 tool/effect/terminal event는 유실하지 않는다.

## 11. Conformance Matrix

모든 production adapter와 test canary가 동일 row를 수행한다. 적용 불가능한 capability는 silent skip이 아니라 descriptor의 explicit negative capability와 rejection test를 가진다.

| Axis | Required evidence |
|---|---|
| registration | unique id, immutable generation, duplicate/unknown rejection |
| request | exact model, immutable Context Plan, bounds, no secret |
| event | monotonic order, bounded payload, one terminal, no post-terminal |
| success | non-empty bounded Result/Evidence and correct provider generation |
| provider rejection | typed auth/quota/rate/protocol categories, safe diagnostics |
| cancellation/deadline | prompt/transport/process cancellation and lease release |
| malformed/overflow | request/frame/update/result limits fail closed |
| lifecycle | register/replace/drain/unregister/idempotent dispose |
| generation | old in-flight completion allowed only under old fence; stale commit blocked |
| side effect | Prepared/Dispatched/Confirmed/Unknown and no blind retry |
| recovery | crash before/during/after activity, no false quiescence |
| security | ambient secret/path/prompt/stderr absent from public output |
| resource | bounded queue/process/child/output under pressure |
| removability | adapter-absent registry/config/workspace build |

Matrix columns은 `TestCanaryProvider`, direct `MiniMaxM3Adapter` migration path, `DeepSeekHarnessAcpProvider` production target이다. Reference fixture도 같은 contract를 사용하지만 production column으로 계산하지 않는다.

## 12. Adapter Removal Build

각 adapter는 다음을 하나의 removable slice로 가진다.

- source module와 exact dependencies
- static factory registry row
- config schema/example/migration
- protocol/transport binding
- conformance instantiation와 adapter-specific wire fixtures
- docs/runbook/SBOM/license entries

필수 variants:

1. canary/reference가 production artifact에 없는 normal build
2. direct MiniMax adapter를 제거하고 DSH adapter만 남긴 build
3. DSH adapter/package/profile을 제거하고 direct adapter test variant만 남긴 build
4. 한 factory 제거 후 unknown config가 fail closed하는 build
5. all production factories enabled build

removal은 dead feature flag나 unreachable hardcoded match를 남기지 않는다. 다른 adapter test가 제거된 adapter fixture/type을 import하면 실패다.

### 12.1 구현 상태와 executable evidence (H11 removal-build slice)

production adapter는 `provider-host`의 removable Cargo feature slice로 분리됐다: `direct-deepseek`, `direct-minimax`, `dsh-acp` (`default`가 셋 전부; `testkit`는 test-only). 각 slice는 module, public re-export, static `ProviderFactory` row, adapter-specific dependency(`dsh-acp`만 `nix`; direct-*는 shared `http-transport`의 `reqwest`), adapter-specific test를 gate한다. `ProtocolKind`는 Common contract로 항상 존재하는 wire descriptor이며 slice 제거 시 factory row가 사라져 해당 variant를 **구성만 못 하게** 한다(impossible cfg enum arm 없음). feature는 `runtime-host`(ACP config decoding path 포함)와 `cli`로 전파되어 normal(no-testkit) / DSH-only(`--no-default-features --features dsh-acp`) / direct-MiniMax-only / all-production CLI가 모두 build된다. `control-server`는 Common contract만 쓰므로 `default-features = false`로 adapter를 강제하지 않아 feature unification이 removal을 무효화하지 않는다. provider-host의 self dev-dependency도 `default-features = false`라 removal test의 feature set을 오염시키지 않는다. 제거된 factory key는 `lookup_factory`에서 `UnknownAdapter`로 fail closed하며, matrix가 각 feature-isolated build에서 exact `removed_<slice>_key_fails_closed` test 이름을 직접 실행한다.

- `scripts/adapter-removal-matrix.sh` — 정확한 removal build/check matrix를 실행하고 안정적인 variant/factory-slice evidence를 출력한다(source mutation 없음). `--check-only` 최근 실행: `PASS=26 FAIL=0 MATRIX=PASS`. `cargo tree` proof로 normal CLI에 `testkit` 부재(ABSENT), DSH-only CLI에 `reqwest`/`testkit`/direct slices 부재를 증명한다.
- shared Conformance는 `provider-host` testkit의 explicit capability descriptor(`ConformanceCapability` 15 axes)와 column(`TestCanary`/direct MiniMax mock HTTP/DSH ACP actual fake subprocess/Reference)로 표현되며 단일 row owner(`conformance_rows`)가 모든 cell을 소유한다. 적용 불가 capability는 silent skip이 아니라 explicit `NotSupported { rejection }`이다(`tests/conformance_matrix.rs`가 완전성·negative 명시를 assert).
- `scripts/secret-scan.sh` — patch/config/argv/log/diagnostic의 raw-credential leak을 정적 스캔하고 provider/runtime secret-canary·diagnostic isolation test를 실행한다. 최근 실행: `SECRET-SCAN=PASS`.

built CLI journey(HJ-001~011)와 live MiniMax canary는 이 slice에 포함하지 않으며 별도 후속 task가 소유한다.

## 13. Built CLI E2E Journeys

| ID | Journey | 필수 관측 |
|---|---|---|
| HJ-001 | start→doctor→Bot activate→Task submit→result→stop | provider=`deepseek-harness-acp`, model=`MiniMax-M3`, non-empty Result/Evidence, child 0 |
| HJ-002 | bad/missing credential | typed unavailable/auth failure, secret 0, no fallback |
| HJ-003 | DSH spawn/version/handshake failure | readiness false, no partial registration/process leak |
| HJ-004 | prompt 중 cancel/deadline | Cancelled/TimedOut, process tree and permits 0 |
| HJ-005 | malformed/oversized/contaminated ACP | ProtocolViolation, bounded diagnostics, Runtime alive |
| HJ-006 | tool permission reject/allow-once | DXBOT authority evidence와 exact effect disposition |
| HJ-007 | crash before/during/after tool effect | Failed 또는 RecoveryRequired/Unknown, blind retry 0 |
| HJ-008 | concurrent executions + provider replace | old/new generation fence, drain, stale commit 0 |
| HJ-009 | Runtime SIGKILL/restart | nonterminal reconciliation, auto resume 0, explicit new attempt |
| HJ-010 | removal build + unknown config | compile PASS와 typed startup failure |
| HJ-011 | pinned DSH upgrade rehearsal | compatibility/conformance/fault/SBOM/license delta review |

Live MiniMax test는 explicit opt-in local profile에서 수행하고 credential/model output을 artifact에 저장하지 않는다. deterministic local fake ACP/MiniMax fixtures가 CI correctness owner다.

## 14. Executable Evidence Gate

최소 명령과 artifact는 다음을 포함한다.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
python3 scripts/plan-validator.py --active --self-test
python3 scripts/agent-guide-validator.py --active --self-test
git diff --check
```

추가로 다음 결과를 release evidence에 남긴다.

- factory registry snapshot과 adapter-removal build matrix
- shared Conformance matrix row/column 결과
- fake ACP fault matrix와 process-tree/permit leak count
- HJ-001~011 built `dxb` output의 secret-safe summary
- exact DSH artifact version/digest, lock integrity, SBOM/license scan
- MiniMax-M3 exact route deterministic tests와 opt-in live canary summary
- resource/backpressure high-water marks and configured bounds
- two independent review reports and finding closure

GitHub Actions 상태만으로 executable evidence를 대체하지 않는다.

## 15. Risk Register

| ID | Risk | Mitigation / Gate |
|---|---|---|
| HR-001 | developer-preview DSH breaking change | exact pin, compatibility adapter, upgrade rehearsal, no auto update |
| HR-002 | Cordis/Node dependency가 Rust lifecycle 오염 | subprocess boundary, managed process tree, no embed |
| HR-003 | MiniMax route/protocol drift | exact model/route verify, deterministic wire tests, live canary, fail closed |
| HR-004 | DSH approval/sandbox를 과신 | DXBOT authority/effect owner + OS isolation + default reject |
| HR-005 | ACP stdout/log contamination | stdout protocol-only, bounded strict decoder, stderr separation |
| HR-006 | process crash 후 side effect 중복 | Ledger Unknown/RecoveryRequired, no blind retry |
| HR-007 | per-run process startup cost | measure first; pooling remains non-goal until evidence |
| HR-008 | session identity가 Domain으로 누출 | opaque EvidenceRef only, schema/API contamination review |
| HR-009 | credential/ambient home leakage | scrubbed env, isolated DSH_HOME, secret canary |
| HR-010 | registry/lifecycle race | atomic publish/replace, captured generation, drain tests |
| HR-011 | unbounded JSON-RPC/tool output | shared finite bounds, backpressure, overflow failure |
| HR-012 | direct adapter가 silent fallback으로 잔존 | explicit migration status, removal build, config rejects fallback |
| HR-013 | supply-chain/license drift | lock integrity, digest, SBOM, MIT notice, upgrade review |
| HR-014 | DSH profile full tool set이 과권한 | audited minimal patch, explicit tool inventory, sandbox/escape tests |

## 16. Stop Conditions

다음이면 workaround를 만들지 말고 구현을 멈춘 뒤 owner/decision review를 수행한다.

1. `MiniMax-M3`를 공식 DSH route로 exact pin할 수 없거나 다른 모델 fallback이 필요하다.
2. official artifact/version/integrity/license를 reproducibly pin할 수 없다.
3. DSH를 in-process embed하거나 upstream fork해야만 진행할 수 있다.
4. raw credential을 argv/profile/state/log에 저장해야만 동작한다.
5. ACP/DSH identity를 Bot/Task/Execution/Process identity로 사용해야 한다.
6. DSH approval/sandbox를 DXBOT Security/SideEffect owner보다 우선해야 한다.
7. unknown external effect를 자동 재실행해야 한다.
8. stdout contamination/malformed frame를 무시해야 interoperability가 유지된다.
9. unbounded queue/frame/output/process/tool concurrency가 필요하다.
10. Provider/adapter-owned nested runtime 또는 Runtime scheduler boundary 밖의 `block_on`을 유지해야 cancellation이 동작한다.
11. RuntimeComposition 밖 production owner graph를 하나 더 만들어야 한다.
12. production adapter가 shared Conformance 또는 removal build를 통과하지 못한다.
13. developer-preview upstream의 security advisory가 isolation policy로 완화되지 않는다.
14. executable evidence 없이 Operational Complete를 선언하려 한다.

## 17. Operational Ready와 Complete Checklist

### Harness Operational Ready

- [ ] H0 exact pin/license/SBOM/reproducible artifact PASS
- [x] H1 canary/reference production exclusion PASS
- [x] H2 canonical async Execute contract와 nested runtime 제거 PASS
- [x] H3 shared Conformance testkit PASS
- [x] H4 per-registration protocol/transport isolation PASS
- [x] H5 static ProviderFactory registry PASS
- [x] H6 pinned DSH ACP subprocess fault matrix PASS
- [x] H7 MiniMax-M3 exact route/credential/tool-call PASS
- [x] H8 RuntimeComposition production startup/shutdown PASS
- [x] H9 register/replace/drain/unregister/recovery PASS
- [x] H10 approval/side-effect/resource/security matrix PASS
- [ ] HJ-001~010 built CLI journeys PASS
- [x] workspace quality/docs validators PASS
- [x] Structural/Consistency review PASS
- [x] Cross-Layer Executability review PASS

### Harness Modularization Complete

Operational Ready에 더해 다음을 만족해야 한다.

- [ ] direct MiniMax migration path의 유지/제거 결정과 silent fallback 0
- [x] adapter-removal build 전체 variant PASS (`scripts/adapter-removal-matrix.sh` → `MATRIX=PASS`)
- [ ] pinned DSH upgrade rehearsal HJ-011 PASS
- [ ] numeric resource bounds load/fault evidence로 freeze
- [ ] runbook에 install/upgrade/rollback/drain/reconcile 절차 반영
- [x] open high/medium review finding 0

## 18. Final Independent Reviews

### Review 1 — Structural / Consistency

다음만 독립적으로 검수한다.

- Canonical Owner 중복과 ACP/DSH type의 Domain 누출
- test canary/reference와 production graph 분리
- factory registry/config/schema/feature/removal slice 정합
- protocol/transport가 registration별로 격리됐는지
- exact pin/license/SBOM Single Source와 stale link
- RuntimeComposition/LocalRuntimeHost owner graph 단일성
- secret/default/model/generation/bounds source 중복

finding 수정 후 validator/build/conformance를 다시 실행한다.

### Review 2 — Cross-Layer Executability

다음 실제 흐름을 별도 reviewer가 추적한다.

```text
CLI Task
→ Application Execution + immutable Context Plan
→ Scheduler/Core Lease
→ ProviderFactory/Registration generation
→ DSH process spawn + ACP initialize/session/prompt
→ MiniMax-M3 route
→ ACP updates/tool permission/effect
→ ExecuteResult/Evidence
→ Application generation fence/commit
→ Process/SideEffect/Audit
→ drain/stop/crash/restart/reconcile
```

각 흐름에 missing credential, incompatible pin, stdout contamination, malformed/oversized frame, cancellation, deadline, process crash, effect Unknown, concurrent replace와 Runtime restart를 대입한다. built `dxb` evidence까지 통과해야 완료다.

## 19. Explicit Non-goals

- dynamic native plugin ABI, `dlopen` 또는 runtime-downloaded adapter
- Rust process 안 Cordis/Node 직접 embedding
- DSH sandbox/approval을 sole security owner로 신뢰
- DSH Session/Agent identity를 Bot/Task/Execution/Process identity로 복사
- `MiniMax-M3` 이외 모델로 전환하거나 silent model/provider fallback
- ReferenceProvider/canary/direct adapter로 production 장애를 숨김
- unpinned `npx latest` 또는 startup-time package install
- P0 process pooling/multi-session multiplex/automatic session resume
- ACP를 DXBOT public Control/Domain protocol로 노출
- DSH upstream fork 또는 upstream 제품 의미의 무비판적 복제
- 새로운 public CLI command나 dynamic plugin management UI

## 20. 완료 순서

```text
H0 pin/supply-chain
→ H1 canary isolation
→ H2 canonical async Execute contract
→ H3 shared Conformance
→ H4 per-registration binding
→ H5 static factory registry
→ H6 official DSH ACP adapter
→ H7 MiniMax-M3 profile/credential
→ H8 RuntimeComposition production wiring
→ H9 lifecycle/recovery
→ H10 security/effect/resource bridge
→ H11 removal build + HJ-001~011 + two reviews
```

핵심 원칙은 **DXBOT이 canonical meaning과 authority를 유지하고, 공식 DSH는 pinned out-of-process execution implementation으로만 교체 가능하게 등록하는 것**이다.

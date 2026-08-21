---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-017", "DXB-DEL-060", "DXB-ENG-052"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

최상위 제품 원칙, 기존 모듈/복구 규칙, v0.3 Common Framework/SPI 규범을 자동화 가능한 Acceptance로 연결한다. 기존 AT-BOT/BRAIN/CORE/CTX/MEM/TASK/NET/CTRL/HAR/IFC/STO/SEC/REC/OBS ID와 v0.2 신규 Acceptance를 모두 유지한다.

## 2. v0.3 신규 SPI 요구

| ID | 요구 | Canonical Owner | Acceptance |
|---|---|---|---|
| FR-SPI-001 | Capability Contract completeness | ARC-012 | AT-SPI-001 |
| FR-SPI-002 | Common-owned Provider Lifecycle | ARC-017 | AT-SPI-002 |
| FR-SPI-003 | Mandatory Provider Host enforcement | ARC-010/017 | AT-SPI-003 |
| FR-SPI-004 | Thin Provider / Minimum Surface | ARC-017, ENG-050 | AT-SPI-004 |
| FR-SPI-005 | Extension Dependency Firewall | ARC-011/017, ENG-054 | AT-SPI-005 |
| FR-SPI-006 | Standard Provider Scaffold | ARC-017, ENG-054 | AT-SPI-006 |
| FR-SPI-007 | Executable Conformance Framework | ARC-017, ENG-052 | AT-SPI-007 |
| FR-SPI-008 | Common Contract Compatibility Discipline | ARC-017, ENG-053 | AT-SPI-008 |
| FR-SPI-009 | Golden Reference Provider | ARC-017, ENG-052 | AT-SPI-009 |
| FR-SPI-010 | Framework-level Provider Removal | ARC-017, ENG-053/054 | AT-SPI-010 |

## 3. AT-SPI-001 — Capability I/O Contract

각 P0 Capability가 최소 다음 semantic field를 가진다.

`Request + Response + Streaming Event + Stable Error/Outcome + Config + Metadata + Cancellation + Deadline + Resource + Idempotency + Side Effect Classification + Version/Compatibility + Conformance`

검증:
1. required contract section 누락이 schema/docs/compile gate에서 탐지된다.
2. Provider-specific DTO/error가 public Capability semantic으로 누출되지 않는다.
3. unsupported optional behavior는 metadata/feature negotiation으로 표현된다.
4. 동일 Capability Provider가 동일 request/outcome semantics를 사용한다.

## 4. AT-SPI-002 — Standard Provider Lifecycle

`Declared → Validated → Starting → Ready → Draining → Stopped`와 `Degraded/Failed/Quarantined/Incompatible`를 Common lifecycle state로 검증한다.

- Ready 이전 신규 selection 0
- Draining 이후 신규 activity 획득 0
- health sample이 lifecycle state를 임의 overwrite하지 않음
- Common in-flight/reference activity가 0이 되기 전 quiesced 처리 0
- stop 후 child/process/channel/activity/permit leak 0
- incompatible/quarantined Provider의 silent auto-enable 0

## 5. AT-SPI-003 — Provider Host Enforcement

Given Runtime Consumer가 Capability를 호출할 때,
When 정상/permission denied/resource exhausted/timeout/cancel/side-effect scenarios를 실행하면,
Then 모든 production call이 Provider Host를 거치며 다음 공통 stage의 evidence가 존재한다.

`validation → selection/generation pin → permission/approval → resource admission → deadline/cancellation → side-effect guard → telemetry/audit → Provider SPI → normalization/output validation → accounting/finalization`.

직접 Provider invocation path가 architecture gate에서 발견되면 실패한다.

## 6. AT-SPI-004 — Thin Provider Boundary

Provider는 provider-specific config, external adapter, capability logic, DTO/error mapping만 소유한다.

실패 조건:
- RuntimeContext/service locator 요구
- Domain Store/Scheduler/Registry mutator/raw Secret Store 접근
- global permission/approval system 구현
- semantic retry/attempt scheduler 구현
- Common resource admission 우회
- Domain persistence 직접 수행
- global telemetry/audit subsystem 중복

Provider-local protocol helper는 semantic contract를 바꾸지 않고 Host/Common authority를 대체하지 않을 때만 허용한다.

## 7. AT-SPI-005 — Forbidden Dependency

`cargo metadata`/architecture scan에서 Provider의 허용 기본 dependency가 다음으로 제한된다.

- Capability Contract
- Provider SDK
- 공개 허용 Kernel value types
- approved external SDK/library

`Provider→Domain/Application/Runtime/Storage/other Provider/UI/private module` edge는 0건이어야 한다. Provider SDK가 금지 API를 transitive re-export하는 것도 실패한다.

## 8. AT-SPI-006 — Provider Scaffold

P1 Scaffold Generator 검증:

1. `dxb-dev new-provider <capability> <provider>` 실행
2. lowercase kebab package와 표준 Rust source 구조 생성
3. `Cargo.toml`, config/provider/error/mapping/conformance/fixture skeleton 존재
4. allowed dependency만 생성
5. 생성 직후 compile 가능
6. Conformance 연결점 포함
7. 동일 입력 재생성이 deterministic
8. 현재 Contract/SDK와 template drift 0

## 9. AT-SPI-007 — Conformance Auto Suite

동일 Capability의 Reference/Provider A/B에 Common suite를 실행한다.

포함:
- normal/streaming
- malformed/unsupported/version mismatch
- timeout/cancel
- transient/permanent provider failure
- partial/large-output/backpressure
- resource exhaustion/permission denied
- side-effect unknown/reconcile
- lifecycle start/degrade/drain/stop
- restart/re-registration
- multi-provider selection
- removal/unavailable

최소 한 suite는 실제 Provider Host 경로를 사용한다.

## 10. AT-SPI-008 — Contract Compatibility

- Additive Compatible 변경에서 지원 Provider가 수정 없이 기존 suite를 통과한다.
- Behavior Clarification은 기존 observable semantic을 의도치 않게 바꾸지 않는다.
- Deprecation은 replacement/removal window를 가진다.
- Breaking Semantic Change가 ADR, Migration/Compatibility, affected Provider inventory, SDK/Conformance/Reference/Acceptance/Risk 갱신 없이 merge되면 실패한다.
- incompatible Contract/SDK Provider는 `Ready` Registry에 publish되지 않는다.

## 11. AT-SPI-009 — Reference Provider Equivalence

- 각 주요 P0 Capability의 deterministic Reference Provider가 허용된 Contract/SDK surface만 사용해 compile한다.
- Reference가 전체 Conformance와 Host-path suite를 통과한다.
- Reference에 privileged internal dependency가 없다.
- intentionally broken negative fixture가 예상된 lifecycle/output/cancel/resource/security gate에서 실패한다.
- Reference와 Conformance가 함께 잘못되는 false-positive를 negative fixture가 탐지한다.

## 12. AT-SPI-010 — Framework Provider Removal

Capability-independent fixture에서 다음을 검증한다.

1. deprecate/new-use block
2. Draining
3. in-flight activity quiesce
4. registry detach/stop
5. stale config/reference explicit diagnostic
6. provider-owned derived data cleanup
7. crate/dependency/registration 제거
8. Provider-free/minimal build
9. existing Core data restore/unrelated acceptance 성공
10. 관련 요청 explicit unavailable/removed
11. orphan dependency/config/feature/registry 0

`AT-MOD-002`는 실제 Harness 등 제품-level Provider vertical slice를 유지하고, `AT-SPI-010`은 Framework 자체의 범용 제거 계약을 검증한다.

## 13. 기존/강화 요구

| ID | 요구 | Owner | Acceptance |
|---|---|---|---|
| FR-MOD-001 | Provider replacement without Domain change | ARC-012/013/017 | AT-MOD-001 |
| FR-MOD-002 | Provider complete removal | ARC-011/012/017, ENG-053/054 | AT-MOD-002 |
| FR-MOD-003 | Multiple Provider explicit selection | ARC-012/017, DOM-024 | AT-MOD-003 |
| FR-ROUTINE-001 | Bot-owned Routine persistence/restart | DOM-020, ARC-015 | AT-ROUTINE-001 |
| FR-TASK-003 | Waiting Continuation crash recovery | DOM-023, RUN-033 | AT-TASK-003 |
| FR-SFX-001 | Side Effect write-ahead/reconcile | DOM-023, ARC-015, RUN-033 | AT-SFX-001 |
| FR-PLUGIN-001 | Plugin lifecycle/data/permission | ARC-016, RUN-032 | AT-PLUGIN-001 |
| FR-REPO-001 | Repository naming/layout | ENG-054 | AT-REPO-001 |
| NFR-MEM-002 | Canonical Memory not evicted by runtime pressure | DOM-022, RUN-031 | AT-MEM-003 |
| NFR-POL-001 | Limit/default SSOT | RUN-031/035 | AT-POL-001 |

## 14. AT-MOD-001 — Provider Replacement

Given Provider A가 기본 Harness이고 동일 Capability Provider B가 존재할 때,
When 신규 Execution의 selector를 B로 변경하면,
Then Bot/Brain/Memory/Task schema와 Domain test를 수정하지 않고 동일 contract outcome을 생성한다. 진행 중 A Execution은 pinned A generation을 유지한다.

## 15. AT-MOD-002 — Product Provider Removal

1. Provider A deprecate/new-use block
2. in-flight drain
3. registry/config/dependency 제거
4. Provider A crate 없는 build
5. existing DB restore
6. Bot Identity/Canonical Memory 동일
7. unrelated Task 성공
8. A 전용 요청은 explicit unsupported/removed
9. stale config silent ignore 0
10. orphan dependency/feature/registry 0
11. Provider Host의 unrelated Provider path 정상 유지

## 16. AT-MOD-003 — Multiple Providers

동시에 A/B/C를 등록하고 `Bot A→A`, `Bot B→B`, `Task C→C`를 명시 선택한다. 동일 priority/input/policy에서 selector가 deterministic하며 한 Execution 중 Provider가 변경되지 않는다. 각 call은 Provider Host를 거친다.

## 17. AT-ROUTINE-001 — Routine Restart

1. enabled Routine 생성
2. occurrence 1이 Task 생성
3. Runtime 종료
4. downtime 중 occurrence 조건 발생
5. restart
6. missed policy에 따라 skip/coalesce/run을 정확히 적용
7. occurrence ID dedup으로 duplicate Task 0
8. archived Bot에서는 신규 Task 0

## 18. AT-TASK-003 — Waiting Continuation Recovery

`Parent → Child A/B/C → A/B complete → Waiting+Continuation commit → kill → restart → C only wait → C result once → Parent resume once`.

완료된 A/B 재실행 0, duplicate C result 통합 1회, missing Continuation은 명시 recovery error다.

## 19. AT-SFX-001 — Side Effect Crash Reconciliation

`Intent/Key commit → external mutate → failpoint before outcome commit → restart`에서 ledger가 Unknown/Reconciliation Required가 되고 automatic duplicate mutate가 발생하지 않는다. status lookup/compensation/manual resolution 후 Reconciled evidence를 남긴다. Provider Host는 필요한 Intent/guard 없이 semantic Side Effect call을 통과시키지 않는다.

## 20. AT-PLUGIN-001 — Plugin Lifecycle

- install package/manifest validate
- permission denied path
- enable 후 Capability Provider 등록
- 제공 Provider가 Standard Lifecycle/Conformance를 통과한 뒤 Ready publish
- in-flight use 중 disable→Provider drain→Plugin quiesce
- actual capability call은 Provider Host 경유
- upgrade generation pinning
- failed migration rollback/disable
- uninstall data policy retain/export/purge/migrate/block 중 manifest 선택 적용
- Core Domain schema/Identity/Memory 무손상
- resource/registry/activity leak 0

## 21. AT-REPO-001 — Repository Naming/Layout

- 사용자 정의 non-Rust path kebab-case
- Rust `*.rs` snake_case 허용
- `Cargo.toml` 등 tool-mandated allowlist 허용
- tool-mandated allowlist 외 예외 거부
- Unicode/invisible/confusable path 거부
- provider/plugin owner root 위반 검출

## 22. AT-MEM-003 — Canonical Memory Pressure

Runtime memory hard-pressure workload에서 cache trim/fan-out/admission 변화는 발생해도 Canonical Memory item/revision이 retention/forget Command 없이 감소하지 않는다.

## 23. AT-POL-001 — Policy SSOT

같은 semantic limit/default가 둘 이상의 Normative owner에서 값으로 중복 정의되지 않으며 Config/Policy ID를 통해 참조된다. config generation 변경이 deterministic digest를 만든다.

## 24. 기존 핵심 Acceptance 유지

- AT-BOT-001 Session independence
- AT-BOT-003 Persistent restore
- AT-BRAIN-001 Single Brain semantics
- AT-CORE-002 Core non-identity
- AT-CORE-005 Dynamic limit
- AT-CTX-001 shared Memory/isolated Working Context
- AT-MEM-002 Working promotion
- AT-TASK-002 retry/cancel
- AT-NET-001 durable delegation
- AT-CTRL-001 Control Plane
- AT-HAR-001 Harness conformance
- AT-IFC-001/002 headless/interface separation
- AT-STO-001 atomic commit
- AT-SEC-001 least privilege
- AT-REC-001 recovery
- AT-OBS-001 end-to-end traceability

## 25. Evidence

test ID, commit/build, Contract/SDK/Provider/Plugin/Config versions, seed/workload, result, metric, trace/artifact digest, dependency graph, platform, waiver를 보존한다.

## 26. 검증 기준

- 모든 v0.3 P0 SPI 요구에 Canonical Owner와 AT가 있다.
- v0.2의 신규 Acceptance 세부 의미가 누락 없이 유지된다.
- AT-SPI-001~010이 `52-testing-verification.md`와 roadmap Phase에 연결된다.
- skip/waiver는 success가 아니며 owner/expiry/risk를 가진다.

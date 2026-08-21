---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-060", "DXB-ENG-052"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

최상위 제품 원칙과 v0.2 모듈/복구 규칙을 자동화 가능한 Acceptance로 연결한다. v0.1의 기존 AT-BOT/BRAIN/CORE/CTX/MEM/TASK/NET/CTRL/HAR/IFC/STO/SEC/REC/OBS ID는 유지한다.

## 2. 신규/강화 요구

| ID | 요구 | Owner | Acceptance |
|---|---|---|---|
| FR-MOD-001 | Provider replacement without Domain change | ARC-012/013 | AT-MOD-001 |
| FR-MOD-002 | Provider complete removal | ARC-011/012, ENG-053/054 | AT-MOD-002 |
| FR-MOD-003 | Multiple Provider explicit selection | ARC-012, DOM-024 | AT-MOD-003 |
| FR-ROUTINE-001 | Bot-owned Routine persistence/restart | DOM-020, ARC-015 | AT-ROUTINE-001 |
| FR-TASK-003 | Waiting Continuation crash recovery | DOM-023, RUN-033 | AT-TASK-003 |
| FR-SFX-001 | Side Effect write-ahead/reconcile | DOM-023, ARC-015, RUN-033 | AT-SFX-001 |
| FR-PLUGIN-001 | Plugin lifecycle/data/permission | ARC-016, RUN-032 | AT-PLUGIN-001 |
| FR-REPO-001 | Repository naming/layout | ENG-054 | AT-REPO-001 |
| NFR-MEM-002 | Canonical Memory not evicted by runtime pressure | DOM-022, RUN-031 | AT-MEM-003 |
| NFR-POL-001 | Limit/default SSOT | RUN-031/035 | AT-POL-001 |

## 3. AT-MOD-001 — Provider Replacement

Given Provider A가 기본 Harness이고 동일 Capability Provider B가 존재할 때,
When 신규 Execution의 selector를 B로 변경하면,
Then Bot/Brain/Memory/Task schema와 Domain test를 수정하지 않고 동일 contract outcome을 생성한다. 진행 중 A Execution은 A를 유지한다.

## 4. AT-MOD-002 — Provider Removal

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

## 5. AT-MOD-003 — Multiple Providers

동시에 A/B/C를 등록하고 `Bot A→A`, `Bot B→B`, `Task C→C`를 명시 선택한다. 동일 priority/동일 input에서 selector가 deterministic하며 한 Execution 중 Provider가 변경되지 않는다.

## 6. AT-ROUTINE-001 — Routine Restart

1. enabled Routine 생성
2. occurrence 1이 Task 생성
3. Runtime 종료
4. downtime 중 occurrence 조건 발생
5. restart
6. missed policy에 따라 skip/coalesce/run을 정확히 적용
7. occurrence ID dedup으로 duplicate Task 0
8. archived Bot에서는 신규 Task 0

## 7. AT-TASK-003 — Waiting Continuation Recovery

`Parent → Child A/B/C → A/B complete → Waiting+Continuation commit → kill → restart → C only wait → C result once → Parent resume once`.

완료된 A/B 재실행 0, duplicate C result 통합 1회, missing Continuation은 명시 recovery error다.

## 8. AT-SFX-001 — Side Effect Crash Reconciliation

`Intent/Key commit → external mutate → failpoint before outcome commit → restart`에서 ledger가 Unknown/Reconciliation Required가 되고 automatic duplicate mutate가 발생하지 않는다. status lookup/compensation/manual resolution 후 Reconciled evidence를 남긴다.

## 9. AT-PLUGIN-001 — Plugin Lifecycle

- install package/manifest validate
- permission denied path
- enable 후 Capability 등록
- in-flight use 중 disable→drain
- upgrade generation pinning
- failed migration rollback/disable
- uninstall data policy retain/export/purge/migrate/block 중 manifest 선택 적용
- Core Domain schema/Identity/Memory 무손상
- resource/registry leak 0

## 10. AT-REPO-001 — Repository Naming/Layout

- 사용자 정의 non-Rust path kebab-case
- Rust `*.rs` snake_case 허용
- tool-mandated allowlist 외 예외 거부
- Unicode/invisible/confusable path 거부
- provider/plugin owner root 위반 검출

## 11. AT-MEM-003 — Canonical Memory Pressure

Runtime memory hard-pressure workload에서 cache trim/fan-out/admission 변화는 발생해도 Canonical Memory item/revision이 retention/forget Command 없이 감소하지 않는다.

## 12. AT-POL-001 — Policy SSOT

같은 semantic limit/default가 둘 이상의 Normative owner에서 값으로 중복 정의되지 않으며 Config/Policy ID를 통해 참조된다. config generation 변경이 deterministic digest를 만든다.

## 13. 기존 핵심 Acceptance 유지

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

## 14. Evidence

test ID, commit/build, config/schema/provider/plugin versions, seed/workload, result, metric, trace/artifact digest, platform, waiver를 보존한다.

## 15. 검증 기준

- 모든 v0.2 P0 FR/NFR에 Owner와 AT가 있다.
- 신규 AT가 `52-testing-verification.md` cross-layer suite에 포함된다.
- skip/waiver는 success가 아니며 owner/expiry/risk를 가진다.

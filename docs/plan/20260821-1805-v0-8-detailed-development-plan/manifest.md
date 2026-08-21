---
title: "DXBOT v0.8 문서 매니페스트와 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-ENG-052", "DXB-ENG-054"]
---

# DXBOT v0.8 문서 매니페스트와 검수 기록

## 1. Baseline

- Source planning baseline: `docs/plan/20260821-1628-v0-7-detailed-development-plan`
- Enhancement instruction: `docs/plan/20260821-1740-v0-8-enhancement-plan.md`
- Target: `docs/plan/20260821-1805-v0-8-detailed-development-plan`
- Theme: **Resolved Implementation Baseline + CLI Contract Closure**
- Active product Interface: **Headless Application Contract + CLI**
- Quality Tier: **Tier A — Common/Application Contract**
- Change type: **development-plan/documentation only**
- Rust implementation changes in this package: **none**

v0.7은 변경 역사와 source comparison이며 active normative source가 아니다. v0.8 package 안의 Canonical Owner가 현재 의미를 완전하게 소유한다.

## 2. Inventory

| Group | Count |
|---|---:|
| Governance | 6 |
| Architecture | 8 |
| Domains | 10 |
| Runtime | 9 |
| Interfaces | 2 |
| Engineering | 5 |
| Delivery | 5 |
| Plan documents | 45 |
| readme + manifest | 2 |
| **Total Markdown** | **47** |

- New Canonical document IDs: **0**
- Existing document IDs materialized/upgraded: **45 + index/manifest**
- Active TUI/Web Interface documents: **0**
- Package content SHA-256 excluding this manifest: `2343f22064c4d7c10b2087c47103e2c1c7ee52fa6590fc4f40d5cca05f708147`

## 3. v0.7 → v0.8 Resolution Matrix

| 범주 | 상태 | v0.8 처리 |
|---|---|---|
| Persistent Bot/Brain/Core/Conversation/Thread | Preserved | active Domain/Runtime docs 안에 직접 materialize |
| Project/Channel/Memory/Task/Process/Security/Resource | Preserved | owner별 self-contained 계약과 검증 기준 유지 |
| Headless Contract + CLI-only active scope | Preserved | BASE/ARC/IFC/DEL에서 직접 고정 |
| previous package normative inheritance | Superseded | 역사 참조로 격하, `생략=상속` 제거 |
| Draft P0 core documents | Changed | 구현 계약 결정 완료 의미의 Accepted로 승격 |
| P0 command 범위 | Clarified | IFC-041 matrix로 typed mapping 100% |
| selector/target resolution | Added closure | ID 우선, scoped exact, ambiguity fail |
| lost-response/idempotency | Added closure | durable receipt, digest binding, retention/reconcile |
| Runtime Host abstraction | Clarified | Linux user instance, XDG/UDS/lock/fencing/readiness |
| pagination | Added closure | snapshot/order/tie-breaker/cursor binding/partial |
| subscription | Added closure | at-least-once/gap/resync/terminal/last cursor |
| JSON/JSONL/exit/wait | Added closure | stable machine contract와 numeric registry |
| local security | Added closure | endpoint/peer/terminal/export threat model |
| public schema | Added closure | Rust SSOT와 generated drift Gate |
| roadmap | Changed | empty workspace→Backend→CLI integrated DAG/vertical slices |
| CI | Changed | active package/schema/matrix/dependency 실제 Gate |
| TUI/Web/BFF/frontend state | Removed from active scope | non-goal/future re-entry only |
| Plugin/provider lifecycle 전체 CLI | Removed from P0 | P1/P2 backlog |

## 4. P0 Document Status

- `DXB-BASE-000`: Normative Baseline
- `DXB-GOV-001~004`: Accepted/Accepted Template
- `DXB-ARC-010/011/014/015`: Accepted
- `DXB-RUN-030~035`: Accepted
- `DXB-IFC-040/041`: Accepted
- `DXB-ENG-050/052/053/054`: Accepted
- `DXB-DEL-060~063`: Accepted
- P0 implementation contract Open Question `OQ-080~089`: **Resolved**

Accepted는 문서 계약이 결정되었다는 의미이며 실제 Rust 구현 Acceptance가 통과했다는 뜻이 아니다.

## 5. New Acceptance / Risk

Acceptance:
- AT-BASE-001
- AT-CLI-008/009/010
- AT-APP-005/006/007
- AT-HOST-001
- AT-SEC-005
- AT-SCHEMA-001
- AT-CI-001

Risk:
- R-090~R-100

Owner/fixture/milestone 연결은 `DXB-DEL-061/062`가 소유한다.

## 6. Review Evidence

### Review 1 — Effective Baseline / Structural / Traceability

- 최초 실행: **PASS**
- 발견: 없음
- 수정 후 전체 재실행: **PASS**
- 검사 수: 578

### Review 2 — Cross-Layer Executability / Fault / Compatibility

- 최초 실행: **FAIL → 수정 완료**
- 발견: F1 — Local export threat contract에 validator가 요구하는 explicit `path traversal` 표기가 없어 계약 문구를 보강
- 수정 후 전체 재실행: **PASS**
- 검사 수: 172

### Review 3 — Adversarial Scope / Overengineering / Ambiguity

- 최초 실행: **FAIL → 수정 완료**
- 발견: F2 — generic mutation 금지 문구가 검증 가능한 explicit 표현이 아니어서 IFC-040을 명확화; F3 — 범용 RPC/IDL 선행 구현 금지 문구를 IFC-040에 명시; F4 — TUI/Web/BFF 비목표 문구를 DEL-060에 명시
- 수정 후 전체 재실행: **PASS**
- 검사 수: 63

## 7. Review Scope 결과

- Markdown inventory/ID/dependency DAG: PASS
- self-contained effective baseline/hidden inheritance: PASS
- P0 Command Matrix and operation mapping: PASS
- selector/receipt/instance/page/stream/machine/security/schema owner coverage: PASS
- P0 OQ-080~089 resolution: PASS
- active TUI/Web/BFF/framework residue: PASS
- lowercase kebab-case: PASS
- relative internal links in package: PASS

## 8. Evidence Boundary

실행한 것:
- local package generation
- deterministic inventory/frontmatter/ID/dependency/cycle/status/link/content checks
- Review 1/2/3 전체 package 재실행
- ZIP 생성과 package hash 계산

실행하지 않은 것:
- Rust build/test/clippy
- Runtime Host/CLI E2E
- Provider Conformance
- concurrency/fault/security executable fixture
- performance/RSS/soak
- repository CI workflow 실행

위 미실행 항목은 아직 Rust 구현과 CI validator가 존재하지 않는 문서 단계이므로 `Specified`이며 `Passed`가 아니다.

## 9. Final Result

- Effective Baseline / Structural / Traceability: **PASS**
- Cross-Layer Executability / Fault / Compatibility: **PASS**
- Adversarial Scope / Overengineering / Ambiguity: **PASS**
- unresolved validation finding after final rerun: **0**
- active Interface scope: **Headless Application Contract + CLI**
- active TUI/Web implementation: **0**
- package is ready to serve as the v0.8 implementation baseline.

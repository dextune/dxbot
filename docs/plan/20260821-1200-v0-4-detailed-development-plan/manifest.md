---
title: "DXBOT v0.4 문서 Manifest와 검증 이력"
document_id: "DXB-MANIFEST"
version: "0.4.0"
status: "Reviewed"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-INDEX", "DXB-ENG-052", "DXB-DEL-061", "DXB-DEL-062"]
---

# DXBOT v0.4 Manifest와 검증 이력

## 1. Source / Upgrade Target

Source package:
`docs/plan/20260821-1120-v0-3-detailed-development-plan`

Target package:
`docs/plan/20260821-1200-v0-4-detailed-development-plan`

Upgrade theme:
**Persistent Conversation + Thread + Live Control**

v0.3은 수정하지 않고 source baseline으로 보존한다.

## 2. Package Inventory

| Group | Count | 상태 |
|---|---:|---|
| `00-governance` | 6 | 3 upgraded / 3 inherited |
| `10-architecture` | 8 | 5 upgraded / 3 inherited |
| `20-domains` | 8 | 6 upgraded + 1 new / 1 inherited |
| `30-runtime` | 7 | 6 upgraded + 1 new |
| `40-interfaces` | 4 | all upgraded |
| `50-engineering` | 5 | 4 upgraded / 1 inherited |
| `60-delivery` | 5 | 4 upgraded / 1 inherited |
| Normative/plan docs | **43** | complete |
| `readme.md` + `manifest.md` | 2 | v0.4 index/evidence |
| Total Markdown | **45** | complete |

Inherited blob을 재사용한 문서도 v0.4 package의 유효한 계약이며 source v0.3 semantic을 유지한다.

## 3. 신규 Canonical Documents

- `20-domains/27-conversation-thread-model.md` — `DXB-DOM-027`
- `30-runtime/36-live-control-preemption.md` — `DXB-RUN-036`

## 4. Upgraded Documents

### Governance
- `00-normative-baseline.md`
- `02-glossary-domain-model.md`
- `03-architecture-decision-baseline.md`

### Architecture
- `10-system-architecture.md`
- `11-rust-workspace-modules.md`
- `13-harness-integration.md`
- `14-event-state-model.md`
- `15-storage-data-model.md`

### Domains
- `20-bot-identity-lifecycle.md`
- `21-brain-context.md`
- `22-memory-system.md`
- `23-goal-task-execution.md`
- `24-dynamic-core-scheduler.md`
- `26-control-plane.md`

### Runtime
- `30-concurrency-consistency.md`
- `31-resource-governance.md`
- `32-security-permissions-sandbox.md`
- `33-resilience-recovery.md`
- `34-observability-audit.md`
- `35-configuration-deployment.md`

### Interfaces
- `40-api-protocols.md`
- `41-cli.md`
- `42-tui.md`
- `43-web-control-center.md`

### Engineering
- `51-performance-memory-cache.md`
- `52-testing-verification.md`
- `53-versioning-migration.md`
- `54-repository-ci-release.md`

### Delivery
- `60-implementation-roadmap.md`
- `61-acceptance-traceability.md`
- `62-risk-register.md`
- `63-open-questions.md`

## 5. Inherited Unchanged Documents

- `00-governance/00-source-concept-original.md`
- `00-governance/01-document-governance.md`
- `00-governance/04-document-template.md`
- `10-architecture/12-shared-contracts-extension-model.md`
- `10-architecture/16-plugin-system.md`
- `10-architecture/17-extension-framework.md`
- `20-domains/25-multi-bot-network.md`
- `50-engineering/50-rust-engineering-rules.md`
- `60-delivery/64-external-reference-snapshot.md`

## 6. Canonical Ownership Check

| Meaning | Canonical Owner | Result |
|---|---|---|
| Main Conversation / Thread / lineage | DXB-DOM-027 | PASS |
| Thread-aware Context | DXB-DOM-021 | PASS |
| Memory scope/promotion | DXB-DOM-022 | PASS |
| Task/spec revision/Execution | DXB-DOM-023 | PASS |
| Core Lease/fairness | DXB-DOM-024 | PASS |
| User-facing Control Plane | DXB-DOM-026 | PASS |
| Live Control/Directive/preemption | DXB-RUN-036 | PASS |
| Provider Host/Lifecycle/SPI | DXB-ARC-017 | PASS |
| Side Effect safety | existing Task/Application/Host contract | PASS |

Session meanings are split into Interface Session, Provider Session, Thread and do not share one owner/type.

## 7. Review 1 — Structural / Canonical Ownership

### Scope
- 43 plan document paths + readme/manifest
- lowercase kebab-case naming
- document IDs/versions/depends_on
- Canonical Owner duplication
- Session/Thread terminology
- source v0.3 preservation

### Findings
**F1 — new `depends_on` edges could form reverse/cyclic architecture-domain-runtime dependencies.**

Correction:
- `DXB-DOM-027`을 BASE/GLOSSARY 기반 독립 Canonical Domain owner로 정리
- `DXB-DOM-023/024/026`의 `DXB-RUN-036` reverse dependency 제거
- `DXB-ARC-010/013/014/015`에서 후행 Domain/Runtime dependency 제거
- `DXB-RUN-030/031/033`에서 `DXB-RUN-036` reverse dependency 제거

Recheck: dependency direction/cycle issue **resolved**.

**F3 — post-compose inventory verification에서 상속 문서 `60-delivery/64-external-reference-snapshot.md`가 Manifest 집계에서 누락됨.**

Correction:
- Delivery count `4 → 5`
- plan document count `42 → 43`
- total Markdown count `44 → 45`
- inherited list에 `64-external-reference-snapshot.md` 추가

Recheck: GitHub commit diff 기준 실제 45개 Markdown과 Manifest/Readme 집계가 일치함.

### Result
**PASS after correction and inventory recheck**

## 8. Review 2 — Contract / Traceability

### Required Chain

```text
Main Conversation
→ Thread
→ Thread-local Memory / Thread-aware Context
→ Task Specification / Execution
→ Live Control Directive
→ Scheduler / Core Lease / Provider Host
→ Persistence / Recovery
→ Control API / CLI / TUI / Web
→ Acceptance / Risk / Open Question
```

### Cross-contract checks
- Interface Session ≠ Provider Session ≠ Thread: PASS
- Main Conversation ≠ Thread 0: PASS
- Conversation History ≠ Memory: PASS
- Thread→Bot Memory explicit promotion: PASS
- transcript size ≠ model Context size: PASS
- Execution immutable under redirect: PASS
- Work Queue ≠ Runtime Control Channel: PASS
- Provider Session loss independence: PASS
- Core Lease remains Scheduler-owned: PASS
- Side Effect Unknown/reconciliation preserved: PASS
- v0.3 Provider Host/SPI/Conformance retained: PASS

### Finding
**F2 — system architecture Mermaid had the Control Channel edge reversed.**

Correction:
`SUP → CTL`을 `CTL → SUP`로 수정하여 본문 Live Control Path와 일치시킴.

Recheck: diagram/text/runtime contract alignment **resolved**.

### Result
**PASS after correction**

## 9. Review 3 — Cross-Layer Executability

Scenario:

```text
Thread A/B/C parallel execution
→ A report request
→ B redirect
→ C suspend
→ B old Execution cooperative yield
→ B Task Specification revision + new Execution
→ Runtime crash
→ restart
→ A report state preserved
→ B redirect crash-window reconciled once
→ C remains Suspended with valid checkpoint
→ Main Conversation queries all Thread/Task/Execution/Core/control state
```

### Verification
- A report does not mutate Task specification: PASS
- B redirect does not mutate old Execution snapshot/provider binding: PASS
- B late old result is fenced from new revision: PASS
- C suspension differs Waiting and is durable: PASS
- crash between Directive commit/yield/new Execution is recoverable: PASS
- duplicate resume/new Execution prevention: PASS
- Side Effect Confirmed/Unknown semantics preserved: PASS
- Thread-local Memory/Context/Directive isolation: PASS
- Scheduler remains Core Lease authority: PASS
- Provider Session loss does not affect Thread identity: PASS
- Main Conversation/API can reconstruct state from Canonical + Projection sources: PASS

### Result
**PASS**

## 10. New Acceptance / Risk / Open Questions

Acceptance:
- AT-CONV-001
- AT-THREAD-001
- AT-CTX-002
- AT-MEM-004
- AT-CTRL-002
- AT-CTRL-003
- AT-CTRL-004
- AT-CTRL-005
- AT-SESSION-002

Risk:
- R-040 ~ R-049

P0 Open Questions:
- OQ-022 ~ OQ-030

각 ID는 `60-delivery` 및 `52-testing-verification.md`에 연결되어 있다.

## 11. Final Status

- Source v0.3 preserved: **PASS**
- Naming/layout: **PASS**
- Canonical ownership: **PASS**
- Dependency relationship: **PASS after correction**
- Package inventory: **PASS after correction**
- Contract traceability: **PASS after correction**
- Cross-layer executability: **PASS**
- Required three independent reviews: **3 / 3 PASS**

v0.4 문서 패키지는 구현 단계에서 Tier A 기준선으로 사용할 수 있는 상태다.

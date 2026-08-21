---
title: "DXBOT v0.2 문서 Manifest"
document_id: "DXB-MANIFEST"
version: "0.2.0"
status: "Catalog"
normative: false
priority: "N/A"
last_updated: "2026-08-21"
depends_on: ["DXB-INDEX"]
---

# DXBOT v0.2 문서 Manifest

- Package: `20260821-1014-v0-2-detailed-development-plan`
- 문서 수(Manifest 제외): 41
- 기본 naming: lowercase kebab-case
- Rust source naming: snake_case 예외
- Tool-mandated path: explicit allowlist
- Source concept는 원문 blob을 그대로 재사용한다.
- 의미 변경이 없는 문서는 v0.1 blob을 재사용하며 자체 frontmatter version도 유지한다.
- v0.2는 generated `all-in-one.md`를 Canonical/Necessary artifact로 요구하지 않는다.

## 1. Governance

| 파일 | 상태 |
|---|---|
| `00-governance/00-source-concept-original.md` | Inherited immutable |
| `00-governance/00-normative-baseline.md` | Updated v0.2 |
| `00-governance/01-document-governance.md` | Updated v0.2 |
| `00-governance/02-glossary-domain-model.md` | Updated v0.2 |
| `00-governance/03-architecture-decision-baseline.md` | Updated v0.2 |
| `00-governance/04-document-template.md` | Updated v0.2 |

## 2. Architecture

| 파일 | 상태 |
|---|---|
| `10-architecture/10-system-architecture.md` | Updated v0.2 |
| `10-architecture/11-rust-workspace-modules.md` | Updated v0.2 |
| `10-architecture/12-shared-contracts-extension-model.md` | Updated v0.2 |
| `10-architecture/13-harness-integration.md` | Updated v0.2 |
| `10-architecture/14-event-state-model.md` | Updated v0.2 |
| `10-architecture/15-storage-data-model.md` | Updated v0.2 |
| `10-architecture/16-plugin-system.md` | New v0.2 |

## 3. Domains

| 파일 | 상태 |
|---|---|
| `20-domains/20-bot-identity-lifecycle.md` | Updated v0.2 |
| `20-domains/21-brain-context.md` | Inherited |
| `20-domains/22-memory-system.md` | Updated v0.2 |
| `20-domains/23-goal-task-execution.md` | Updated v0.2 |
| `20-domains/24-dynamic-core-scheduler.md` | Updated v0.2 |
| `20-domains/25-multi-bot-network.md` | Updated v0.2 |
| `20-domains/26-control-plane.md` | Updated v0.2 |

## 4. Runtime

| 파일 | 상태 |
|---|---|
| `30-runtime/30-concurrency-consistency.md` | Updated v0.2 |
| `30-runtime/31-resource-governance.md` | Updated v0.2 |
| `30-runtime/32-security-permissions-sandbox.md` | Updated v0.2 |
| `30-runtime/33-resilience-recovery.md` | Updated v0.2 |
| `30-runtime/34-observability-audit.md` | Updated v0.2 |
| `30-runtime/35-configuration-deployment.md` | Updated v0.2 |

## 5. Interfaces

| 파일 | 상태 |
|---|---|
| `40-interfaces/40-api-protocols.md` | Updated v0.2 |
| `40-interfaces/41-cli.md` | Updated v0.2 |
| `40-interfaces/42-tui.md` | Updated v0.2 |
| `40-interfaces/43-web-control-center.md` | Updated v0.2 |

## 6. Engineering

| 파일 | 상태 |
|---|---|
| `50-engineering/50-rust-engineering-rules.md` | Updated v0.2 |
| `50-engineering/51-performance-memory-cache.md` | Updated v0.2 |
| `50-engineering/52-testing-verification.md` | Updated v0.2 |
| `50-engineering/53-versioning-migration.md` | Updated v0.2 |
| `50-engineering/54-repository-ci-release.md` | Updated v0.2 |

## 7. Delivery

| 파일 | 상태 |
|---|---|
| `60-delivery/60-implementation-roadmap.md` | Updated v0.2 |
| `60-delivery/61-acceptance-traceability.md` | Updated v0.2 |
| `60-delivery/62-risk-register.md` | Updated v0.2 |
| `60-delivery/63-open-questions.md` | Updated v0.2 |
| `60-delivery/64-external-reference-snapshot.md` | Inherited |

## 8. Package Root

| 파일 | 상태 |
|---|---|
| `readme.md` | Updated v0.2 |

## 9. Review Evidence

### Review 1 — Structural / Consistency — Passed

검사:
- 파일/디렉터리 naming 및 ASCII path
- document ID / `depends_on`
- Canonical/Lifecycle Owner
- Provider/Plugin 용어
- Manifest/Acceptance/Risk 연결

발견 및 수정:
- Plugin↔Security/Versioning의 순환 `depends_on` 제거
- Scheduler↔Resource Governance의 순환 `depends_on` 제거
- Rust Engineering↔Performance의 순환 `depends_on` 제거
- v0.1에 남아 있던 Event/State 문서를 v0.2로 승격해 Routine/Continuation/Side Effect 관계 보강
- repository naming 문서의 Unicode/invisible path guard 명시

### Review 2 — Cross-Layer Executability — Passed

검사 경로:
`Command → Application → Domain → Persistence → Scheduler → Provider → Recovery → Projection → Control API → Interface → Acceptance`

검증 시나리오:
- Bot persistence
- Multi-Bot delegation/restart
- Waiting Continuation recovery
- Side Effect crash/reconciliation
- Routine restart/missed occurrence
- Provider replacement/removal/multi-selection
- Plugin install/disable/upgrade/uninstall
- Long-term Memory pressure
- Repository/Policy governance

발견 및 수정:
- Multi-Bot delegated child와 Parent Continuation 연결 보강
- concurrency 문서에 Routine/Waiting/Provider drain/Plugin disable/Side Effect race 보강
- CLI/TUI/Web에 Routine/Provider/Plugin/Recovery 운영 표면 연결
- Plugin uninstall `block` data policy를 Storage 계약과 일치시킴

두 검수 후 새로 확인된 owner 없는 Canonical State 또는 silent fallback 경로는 없다.

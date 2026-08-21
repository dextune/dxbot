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
| `20-domains/25-multi-bot-network.md` | Inherited |
| `20-domains/26-control-plane.md` | Updated v0.2 |

## 4. Runtime

| 파일 | 상태 |
|---|---|
| `30-runtime/30-concurrency-consistency.md` | Inherited |
| `30-runtime/31-resource-governance.md` | Updated v0.2 |
| `30-runtime/32-security-permissions-sandbox.md` | Updated v0.2 |
| `30-runtime/33-resilience-recovery.md` | Updated v0.2 |
| `30-runtime/34-observability-audit.md` | Updated v0.2 |
| `30-runtime/35-configuration-deployment.md` | Updated v0.2 |

## 5. Interfaces

| 파일 | 상태 |
|---|---|
| `40-interfaces/40-api-protocols.md` | Updated v0.2 |
| `40-interfaces/41-cli.md` | Inherited |
| `40-interfaces/42-tui.md` | Inherited |
| `40-interfaces/43-web-control-center.md` | Inherited |

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

## 9. Review Requirement

문서 패키지는 commit 전/승격 전 다음 두 검수를 별도 수행한다.

1. Structural / Consistency Review
2. Cross-Layer Executability Review

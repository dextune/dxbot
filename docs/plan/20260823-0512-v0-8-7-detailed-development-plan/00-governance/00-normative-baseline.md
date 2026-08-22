---
title: "DXBOT v0.8.7 최상위 구현 기준선"
document_id: "DXB-BASE-000"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: []
---
# DXBOT v0.8.7 최상위 구현 기준선

v0.8.7은 기능 확장이 아니라 v0.8.6의 CLI 구현 전 계약을 crash/recovery/Owner/마일스톤 관점에서 수렴시킨 기준선이다.

## 제품 불변조건

`INV-001`~`INV-015`는 v0.8.6과 동일하다. Persistent Bot Identity·Memory·State, logical Brain, Dynamic Core Lease, typed delegation, Headless Contract, bounded resource, Provider/Interface 비소유 원칙을 유지한다.

## v0.8.7 비협상 교정

1. CLI local journal은 `Prepared → Dispatching → Observed → Terminal` 또는 provably-unsent `Abandoned`다. `Dispatching` durable fsync 이후에만 socket send를 시작한다.
2. Runtime은 authenticated Principal을 얻은 직후 `(PrincipalRef, IdempotencyKey)`와 `CommandId` binding을 먼저 조회한다. 기존 binding이 있으면 current selector/authorization을 재평가하지 않고 stored binding과 digest를 비교해 receipt를 반환한다.
3. binding이 없을 때만 selector resolution, authorization, policy generation resolution을 수행한다.
4. request/pre-accept error, Operation Receipt `Rejected`, committed Domain outcome `Rejected/Deferred`를 분리한다.
5. approval-required operation은 durable operation intent와 Receipt `Accepted`를 유지하고 ApprovalRef를 결박한다. approve/deny가 같은 operation을 재평가해 `Committed` 또는 `Rejected`로 종결한다.
6. `application-contract`는 public schema/metadata만 소유한다. Receipt lifecycle은 `application-operation`, persistence는 operation store가 소유한다.
7. P0 Application Command machine output은 공통 `out-operation-v1` envelope를 사용한다. Domain resource/result는 committed payload 또는 별도 Query로 조회한다.
8. P0 `target-terminal` wait는 제거한다. `applied`는 명시적 Directive/effect Owner가 있는 operation에만 허용한다.
9. create command는 전역 Instance revision CAS에 결박하지 않는다. uniqueness/idempotency와 owning aggregate revision만 사용한다.
10. Bot persistent identity 생성은 Provider readiness와 독립적이다. Provider availability는 activation/execution admission에서 판단한다.
11. full CLI surface는 한 번에 freeze하지 않는다. M2/M3/M4/M5 subset이 각 milestone evidence 후 단계적으로 freeze된다.
12. document validator PASS와 generated Rust contract PASS를 분리한다.

## Canonical Owner Registry

<!-- canonical-owner-registry:start -->
- `active-package` | `DXB-GOV-001` | `plan-governance` | `repository` | `DXB-INDEX`
- `product-invariants` | `DXB-BASE-000` | `baseline` | `repository` | `DXB-INDEX`
- `terminology` | `DXB-GOV-002` | `domain-language` | `repository` | `N/A`
- `application-operation-metadata` | `DXB-IFC-040` | `application-contract` | `contract-snapshot-store` | `application-contract`
- `operation-receipt` | `DXB-ARC-014` | `application-operation` | `operation-store` | `application-contract`
- `cli-projection-and-local-journal` | `DXB-IFC-041` | `cli` | `cli-state-store` | `application-contract`
- `cli-input-grammar` | `DXB-IFC-042` | `application-contract` | `contract-snapshot-store` | `application-contract`
- `bot-identity` | `DXB-DOM-020` | `domain-bot` | `bot-store` | `application-contract`
- `context-plan` | `DXB-DOM-021` | `application-context-plan` | `execution-store` | `application-contract`
- `memory-assertion` | `DXB-DOM-022` | `domain-memory` | `memory-store` | `application-contract`
- `task-execution-side-effect` | `DXB-DOM-023` | `domain-task` | `task-side-effect-store` | `application-contract`
- `core-lease` | `DXB-DOM-024` | `runtime-scheduler` | `transient-runtime-state` | `application-contract`
- `delegation` | `DXB-DOM-025` | `application-delegation` | `task-store` | `application-contract`
- `use-case-catalog` | `DXB-DOM-026` | `application` | `N/A` | `application-contract`
- `conversation-thread` | `DXB-DOM-027` | `domain-conversation` | `conversation-store` | `application-contract`
- `project-membership` | `DXB-DOM-028` | `domain-project` | `project-store` | `application-contract`
- `channel-membership` | `DXB-DOM-029` | `domain-channel` | `channel-store` | `application-contract`
- `storage-semantics` | `DXB-ARC-015` | `storage` | `canonical-storage` | `N/A`
- `storage-proof-profile` | `DXB-ARC-018` | `storage-spike` | `test-fixtures` | `N/A`
- `harness-capability` | `DXB-ARC-013` | `provider-host` | `provider-registry` | `capability-contract`
- `resource-admission` | `DXB-RUN-031` | `runtime-resource-governor` | `resource-ledger` | `application-contract`
- `principal-approval-authority` | `DXB-RUN-032` | `runtime-security` | `security-store` | `application-contract`
- `recovery` | `DXB-RUN-033` | `runtime-recovery` | `canonical-storage` | `application-contract`
- `audit` | `DXB-RUN-034` | `runtime-audit` | `audit-store` | `application-contract`
- `bootstrap-default-policy` | `DXB-RUN-035` | `runtime-bootstrap` | `instance-metadata-store` | `application-contract`
- `control-directive` | `DXB-RUN-036` | `application-control` | `directive-store` | `application-contract`
- `durable-process` | `DXB-RUN-038` | `runtime-process` | `process-store` | `application-contract`
<!-- canonical-owner-registry:end -->

## 구현 진입 판정

Workspace/M1A/M1B scaffolding은 GO다. M2/M3/M4/M5 command subset은 각 milestone Acceptance가 PASS한 뒤에만 freeze하며, 전체 63-operation freeze는 M6 이전에는 금지한다.

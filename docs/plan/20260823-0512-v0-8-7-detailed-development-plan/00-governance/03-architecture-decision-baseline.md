---
title: "v0.8.7 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---
# v0.8.7 아키텍처 결정 기준선

v0.8.6 ADR-0095~0114는 유지한다. 다음 교정을 추가한다.

| ADR | 결정 |
|---|---|
| ADR-0115 | durable `Dispatching` journal record를 fsync한 이후에만 network send를 시작한다. |
| ADR-0116 | authenticated Principal 이후 existing idempotency/Command binding lookup을 selector·authorization 재평가보다 먼저 수행한다. |
| ADR-0117 | PreAcceptError, Receipt Rejected, committed Domain Rejected/Deferred를 서로 다른 상태로 취급한다. |
| ADR-0118 | approval-required operation은 pending durable intent를 유지하고 Approval decision이 원 operation을 자동 재평가·종결한다. |
| ADR-0119 | application-contract는 Receipt schema, application-operation은 Receipt lifecycle, operation-store는 persistence를 소유한다. |
| ADR-0120 | Membership row + AuthorityBinding generation/revoke + required AuditIntent는 한 application Unit of Work에서 atomic commit한다. |
| ADR-0121 | P0 Application Command output은 `out-operation-v1`으로 수렴하고 `target-terminal` wait를 제거한다. |
| ADR-0122 | create command에서 global Instance revision CAS를 사용하지 않는다. |
| ADR-0123 | CLI contract freeze는 milestone별 subset으로 수행하며 M6 전 full 63-operation freeze를 금지한다. |
| ADR-0124 | Bot identity creation은 Provider readiness와 분리한다. |

구체 DB/Harness/threshold는 실행 증거 뒤 별도 freeze decision으로 둔다.

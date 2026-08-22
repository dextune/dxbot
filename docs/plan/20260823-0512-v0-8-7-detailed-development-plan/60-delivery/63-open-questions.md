---
title: "v0.8.7 결정 기록과 후속 질문"
document_id: "DXB-DEL-063"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-GOV-003", "DXB-DEL-060", "DXB-IFC-040", "DXB-IFC-041", "DXB-ARC-018"]
---
# v0.8.7 결정 기록과 후속 질문

## Resolved

- journal은 durable Dispatching 이후에만 send한다.
- existing idempotency/Command binding lookup은 selector/auth 재평가보다 먼저다.
- PreAcceptError, Receipt Rejected, Task/Delegation Domain rejection을 분리한다.
- Approval decision이 original pending operation을 자동 continuation한다.
- Receipt runtime Owner를 application-operation으로 분리한다.
- Membership/AuthorityBinding/AuditIntent는 atomic Unit of Work다.
- Application Command output은 out-operation-v1로 수렴하고 target-terminal wait를 제거한다.
- create command의 global Instance revision CAS를 제거한다.
- Channel initial-members batch를 제거하고 member set을 재사용한다.
- Bot identity는 Provider readiness와 독립 생성한다.
- M2/M3/M4/M5 milestone별 subset만 freeze한다.
- document validator Acceptance와 Rust generated Acceptance를 분리한다.

## Deferred implementation choices

concrete storage/Harness, numeric retention/bytes/compatibility, stronger same-UID isolation, TUI/Web/Plugin lifecycle CLI는 기존처럼 후속 evidence+ADR 대상이다.

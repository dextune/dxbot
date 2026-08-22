---
title: "DXBOT v0.8.6 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
package_path: "docs/plan/20260822-2027-v0-8-6-detailed-development-plan"
review_revision: 6
adversarial_review_rounds: 5
final_rechecks: 2
parent_plan_commit: "d07f6041d54dd3a2602446336080257a1520b9ae"
---
# DXBOT 상세 개발기획 v0.8.6

v0.8.6은 기능을 늘린 계획이 아니다. v0.8.5의 CLI 구현 전 적대적 검토에서 드러난 계약 모순과 무소유 상태를 제거하고, 기존 Persistent Bot Runtime 방향을 구현 가능한 의미로 닫은 교정 버전이다.

## 현재 구현 진입 판정

| 범위 | 판정 | 이유 |
|---|---|---|
| Workspace bootstrap | GO | 책임·의존성 경계가 정리됨 |
| M1A Storage Proof | GO | 검증할 durability/idempotency/snapshot 의미가 닫힘 |
| M1B contract source 구현 | GO | operation/error/exit/wait metadata 규격이 닫힘 |
| 전체 CLI parser/schema freeze | BLOCKED | M1A, generated metadata, crash fixture가 아직 실행되지 않음 |
| Release-ready 주장 | BLOCKED | Rust E2E, real Harness, security/performance evidence가 없음 |

## v0.8.6 핵심 교정

- client 계산 가능한 `RequestDigest`와 server 계산 `ResolvedBindingDigest` 분리
- Local Journal, Operation Receipt, Directive, Target Lifecycle 상태 분리
- operation별 wait predicate와 stable error/exit/output schema 정의
- Approval, AuthorityBinding, ActionGrant, SideEffect Ledger의 Canonical Owner와 상태기계 확정
- 안전한 default Brain/Permission/Resource/Provider policy semantic 확정
- epoch-bearing IdempotencyKey와 bounded acceptance horizon 확정
- 62개 lexical CLI path를 63개 typed operation mode로 분해하고 실제 입력 DSL 정의
- Dynamic Core P0 병렬성을 서로 다른 Execution 간 병렬성으로 제한
- Membership CAS, delegated sender binding, local journal abandonment 의미 확정
- validator의 자동 검증 범위와 semantic review 범위를 분리
- 5개 적대적 review evidence와 2개 최종 재검수 evidence를 별도 기록

## Active implementation path

```text
v0.8.6 semantic closure
→ M1A Storage Proof
→ application-contract metadata source
→ Kernel/Domain/Application minimum
→ Instance/Principal/Default Policy bootstrap
→ Bot-only CLI vertical slice
→ Multi-Bot membership/delegation
→ Recovery/Streaming/Export/real Harness
→ Compatibility/Security/Performance freeze
```

## Inventory

Governance 6, Architecture 9, Domains 10, Runtime 9, Interfaces 3, Engineering 5, Delivery 5: plan documents 47. `readme.md`와 `manifest.md`를 포함해 Markdown 49개다.

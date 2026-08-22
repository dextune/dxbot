---
title: "DXBOT v0.8.7 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
package_path: "docs/plan/20260823-0512-v0-8-7-detailed-development-plan"
review_revision: 7
adversarial_review_rounds: 5
final_rechecks: 2
parent_plan_commit: "e20fcf70dcaf22aa7b1988db03bb7297cf280729"
---
# DXBOT 상세 개발기획 v0.8.7

v0.8.7은 v0.8.6 적대적 재검토 결과를 기능 확장 없이 CLI 구현 가능한 상태로 수렴시킨 교정 package다.

## 핵심 수렴

- local journal `Dispatching` durability 이후 send
- existing idempotency/Command binding lookup 선행
- PreAcceptError / Receipt Rejected / Domain Rejected·Deferred 분리
- Approval decision이 original pending operation을 continuation
- Receipt schema Owner와 runtime lifecycle Owner 분리
- Membership+AuthorityBinding+AuditIntent atomic Unit of Work
- Application Command output을 `out-operation-v1`로 통일
- P0 `target-terminal` 제거, `applied` 범위 축소
- create의 global Instance revision CAS 제거
- Bot identity와 Provider readiness 분리
- M2/M3/M4/M5 milestone별 subset freeze
- document validator Acceptance와 Rust generated contract Acceptance 분리

## 구현 진입 판정

Workspace/M1A/M1B scaffolding은 GO다. M2 이후 public CLI surface는 milestone evidence에 따라 부분 freeze한다. 전체 63-operation freeze는 M6 전 BLOCKED다.

## Inventory

Governance 6, Architecture 9, Domains 10, Runtime 9, Interfaces 3, Engineering 5, Delivery 5: plan documents 47. `readme.md`와 `manifest.md` 포함 Markdown 49개다.

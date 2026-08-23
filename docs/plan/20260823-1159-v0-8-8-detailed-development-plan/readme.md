---
title: "DXBOT v0.8.8 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.8"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-005", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
package_path: "docs/plan/20260823-1159-v0-8-8-detailed-development-plan"
review_revision: 8
adversarial_review_rounds: 5
final_rechecks: 2
parent_plan_commit: "c7323ff8e47aac1d8cd40988a2322e1397cd1239"
---

# DXBOT 상세 개발기획 v0.8.8

v0.8.8은 기능 확장이 아니라 v0.8.7 적대적 검토에서 확인된 계약 회귀와 Gate 불일치를 수렴하는 버전이다.

핵심 교정은 `00-governance/05-v0-8-8-contract-closure.md`가 소유한다. v0.8.7의 binding-first lookup, durable Dispatching, rejection 3분법, Approval continuation, Receipt owner 분리, atomic membership/security/audit, `out-operation-v1`, no `target-terminal`, create global CAS 제거, Bot identity/Provider readiness 분리, milestone subset freeze 방향은 유지한다.

추가로 common Command envelope, local/wire option 경계, RequestDigest canonicalization 책임, versioned CLI Local Journal, Bot/Project/Task exposed lifecycle, P0 Common Extension→P1 Plugin dependency 방향, milestone/Acceptance 수렴 조건을 명시적으로 폐쇄한다.

## 구현 진입 판정

- Workspace scaffolding: GO
- M1A storage/failpoint spike: GO
- M1B contract source prototype: 조건부 GO; public freeze 금지
- M2/M3 subset freeze: Acceptance evidence 전까지 BLOCKED
- 전체 operation/schema freeze: M6까지 BLOCKED

새 CLI command, 범용 RPC/IDL/Workflow engine, Plugin/TUI/Web/distributed scope는 추가하지 않는다.

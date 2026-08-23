---
title: "DXBOT v0.8.9 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-005", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
package_path: "docs/plan/20260823-1218-v0-8-9-detailed-development-plan"
review_revision: 9
adversarial_review_rounds: 5
final_rechecks: 2
parent_plan_commit: "e7438f99d6d72aa03fb7fde10668a8bdbcbb22e1"
---
# DXBOT 상세 개발기획 v0.8.9

v0.8.9는 기능 확장이 아니라 v0.8.8의 CLI 구현 전 관계성 결함을 수렴하는 버전이다.

핵심 교정은 다음 다섯 가지다.

1. active package 안에서 제품 불변조건, active ADR, security/recovery fail-closed 의미를 완결한다.
2. 하나의 CLI input metadata에서 `CliInput`과 `CommandPayload`를 생성하고 local/source field의 wire 누출을 금지한다.
3. Instance-global CommandId, principal-bound IdempotencyKey, compaction tombstone, single-writer journal takeover, no implicit cancel을 닫는다.
4. Acceptance의 최초 milestone을 단일화하고 누적 Gate와 command freeze 관계를 기계 검증한다.
5. current value를 실제 변이하는 validator negative fixture로 false-green을 차단한다.

## 구현 진입 판정

- Workspace scaffolding: GO
- M1A storage/failpoint spike: GO
- M1B contract source prototype: GO, public freeze 금지
- M2/M3/M4/M5 subset freeze: 누적 executable Acceptance PASS 전까지 BLOCKED
- 전체 63-operation/schema freeze: M6까지 BLOCKED

새 CLI command, generic RPC/IDL/Workflow engine, DB/Harness 제품 고정, Plugin/TUI/Web/distributed scope는 추가하지 않는다.

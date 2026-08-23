---
title: "DXBOT v0.8.10 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-005", "DXB-IFC-043", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063"]
package_path: "docs/plan/20260823-1310-v0-8-10-detailed-development-plan"
review_revision: 10
adversarial_review_rounds: 5
final_rechecks: 2
parent_plan_commit: "4f42cfa9e413aa306dc9bcfc7538f00e6763b3e6"
---
# DXBOT 상세 개발기획 v0.8.10

v0.8.10은 기능 확장이 아니라 v0.8.9를 실제 사용자 관점에서 다시 검토해, 안전하지만 사람이 사용할 수 없던 CLI 입력과 복구 표면을 수렴한 버전이다.

핵심 교정은 다음과 같다.

1. first-run Instance 선택과 Runtime bootstrap 경로를 닫는다.
2. 사용자는 Domain selector를 입력하고 CLI가 bounded preflight로 canonical ID와 CAS를 materialize한다.
3. `conversation send`는 Bot Main Conversation, `channel send`는 Channel을 직접 대상으로 삼는다.
4. help, human/json/jsonl, non-TTY, actionable failure, journal 재진입과 `--all` partial/resume 의미를 닫는다.
5. 사용자 여정 10개를 별도 Canonical Owner와 Acceptance로 추적한다.

## 사용자 관점 판정

- 첫 실행과 Runtime discovery: 설계 GO
- Bot 생성부터 첫 대화·Task 결과까지: 설계 GO
- 일상 mutation과 conflict 처리: 설계 GO
- 자동화·machine output: 설계 GO
- approval/provider/ambiguity/recovery: 설계 GO
- multi-instance와 pagination/output: 설계 GO

위 GO는 문서 계약 완결 판정이다. Rust/Runtime/CLI 실행 Acceptance는 구현 전까지 `Blocked`이며 실제 제품 사용성 PASS를 주장하지 않는다.

새 CLI command, wizard, auto-start, auto mutation retry, generic RPC/IDL/Workflow engine, DB/Harness 제품 고정, TUI/Web/Plugin/distributed scope는 추가하지 않는다.

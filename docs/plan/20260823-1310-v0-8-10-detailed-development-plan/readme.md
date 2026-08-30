---
title: "DXBOT v0.8.10 상세 개발기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-30"
depends_on: ["DXB-BASE-000", "DXB-GOV-005", "DXB-IFC-043", "DXB-DEL-060", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-DEL-066", "DXB-DEL-067", "DXB-DEL-068"]
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

위 GO는 문서 계약 완결 판정이다. 실제 제품 판정은 최신 executable evidence와 post-closeout 적대적 재감사를 함께 적용한다.

새 CLI command, wizard, auto-start, auto mutation retry, generic RPC/IDL/Workflow engine, DB 제품 고정, TUI/Web/dynamic Plugin/distributed scope는 추가하지 않는다. Harness 제품과 모델 결정은 이후 `MiniMax-M3` 및 공식 DeepSeek Harness로 고정됐으며 본 package의 후속 문서가 이를 추적한다.

## CLI 개발 종료 Gate

기존 Acceptance의 component/projection PASS와 실제 `dxb` 제품 완료를 구분한다. 기능 확장 없이 CLI를 종료하기 위한 blocking finding, execution spine, 63-operation executability, binary user journey, crash/recovery 및 최종 2회 재검수의 역사적 closeout evidence는 [CLI 개발 완료 적대적 종료 플랜](60-delivery/66-cli-completion-adversarial-plan.md)을 따른다.

## Post-closeout 재감사와 실운영 준비

GitHub `main`을 CLI closeout 이후 다시 적대적으로 검토한 결과, execution/security/recovery spine은 유지되지만 version live compatibility, `runtime start --ready-at`, human mutation/result actionability, human failure next action, unknown-command suggestion의 사용자 표면 계약이 다시 열렸다.

또한 CLI가 Task/Process를 durable하게 생성하는 것과 실제 Brain/Context Plan→Execution→Resource Governor→Dynamic Core Lease→real Provider→Result/Evidence→Memory/Recovery를 수행하는 것은 별도 완료 조건이다.

현재 reopened CLI finding과 production Provider/Execution/Memory/Multi-Bot/운영 준비의 전체 순서 및 `DXBOT Operational Ready` Gate는 [CLI 이후 DXBOT 실운영 준비 적대적 개발 플랜](60-delivery/67-operational-runtime-readiness-adversarial-plan.md)을 따른다.

`DXB-DEL-066`의 과거 실행 PASS만으로 현재 `CLI Contract Complete` 또는 전체 `DXBOT Operational Ready`를 자동 선언하지 않는다.

## Harness 모듈화 후속 개발

`DXB-DEL-067`의 local P0 direct Provider evidence를 범용 Harness framework 완료로 확대 해석하지 않는다. 고정된 `MiniMax-M3`와 공식 DeepSeek Harness의 pinned ACP v1 subprocess adapter를 전제로, 개선사항 1~9, H0~H11 gate, fault/resource/security matrix, adapter-removal build와 독립 재검수를 [MiniMax M3 기반 공식 DeepSeek Harness 모듈화 후속 개발 플랜](60-delivery/68-modular-harness-adoption-plan.md)이 소유한다.

해당 checklist가 executable evidence로 닫히기 전 `Harness Modularization Complete`는 BLOCKED다.

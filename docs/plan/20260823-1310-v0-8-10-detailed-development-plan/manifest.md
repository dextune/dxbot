---
title: "DXBOT v0.8.10 문서 매니페스트와 사용자 실사용 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000", "DXB-GOV-005", "DXB-IFC-043", "DXB-DEL-061", "DXB-DEL-062", "DXB-ENG-052", "DXB-ENG-054"]
review_revision: 10
adversarial_review_rounds: 5
final_rechecks: 2
---
# DXBOT v0.8.10 문서 매니페스트와 사용자 실사용 검수 기록

<!-- manifest-machine:start -->
active_package_path: docs/plan/20260823-1310-v0-8-10-detailed-development-plan
plan_version: 0.8.10
review_revision: 10
adversarial_review_rounds: 5
final_rechecks: 2
markdown_count: 51
parent_plan_commit: 4f42cfa9e413aa306dc9bcfc7538f00e6763b3e6
source_baseline_commit: 5b55b67fbaaf0f3192c126865e8b665ff26fc5aa
<!-- manifest-machine:end -->

## v0.8.10 사용자 적대적 검토 증거

<!-- review-evidence:start -->
- `R1-ONBOARDING` | install 이후 help/version/first runtime start/instance selection | 첫 실행 막힘과 bootstrap paradox 제거 | `DXB-RUN-035/DXB-IFC-043` | PASS
- `R2-CORE-JOURNEY` | Bot create→Main Conversation send→Task submit/result | internal ID와 mandatory revision 복사 제거 | `DXB-DOM-020/027/DXB-IFC-042/043` | PASS
- `R3-AUTOMATION` | non-TTY, machine format, stdout/stderr, exit/action schema | script가 human prompt와 렌더링에 오염되지 않음 | `DXB-IFC-040/041/043` | PASS
- `R4-FAILURE-RECOVERY` | runtime/provider unavailable, conflict, approval, interrupted submission | 사용자가 다음 안전 행동을 결정 가능 | `DXB-RUN-033/DXB-IFC-043` | PASS
- `R5-SAFETY-SCALE` | multi-instance, ambiguity, pagination, output, sensitive input | silent fallback/overwrite/unbounded UX 제거 | `DXB-RUN-032/035/DXB-IFC-041/043` | PASS
<!-- review-evidence:end -->

<!-- final-recheck:start -->
- `RC1-STRUCTURAL` | package path, 51 documents, frontmatter, ID/DAG, owner, registry, manifest, naming | PASS
- `RC2-CROSS-LAYER` | user selector→preflight→CommandPayload→journal→binding→Receipt/Domain→recovery→human/machine projection | PASS
<!-- final-recheck:end -->

`PASS`는 사용자 관점 문서 관계성 검수 결과다. 실제 CLI 실행 사용성은 Rust/Runtime/CLI fixture가 `Passed`가 되기 전까지 주장하지 않는다.

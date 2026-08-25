---
title: "문서 패키지 Manifest"
document_id: "DXB-ADP-MANIFEST"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-INDEX"]
target_owners: ["docs/plan"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 문서 패키지 Manifest

## 1. 패키지 정보

- 패키지: `20260825-0930-grok-architecture-adoption-plan`
- 상태: `Reference Snapshot`
- 규범성: `false`
- 목적: Grok Bot 재구성본에서 차용 가능한 runtime 운영 패턴을 DXBOT 기존 아키텍처에 맞게 보완
- DXBOT baseline: `e43739614631c95752482c2ad2ec53cb7ebd251f`
- Grok baseline: `a9f633e09d49a85829b8236331b9e21f7e612634`

## 2. 파일 목록

| 순서 | 파일 | Document ID | 목적 |
|---:|---|---|---|
| 1 | `readme.md` | `DXB-ADP-INDEX` | 전체 결론과 읽기 순서 |
| 2 | `00-scope-and-decision-principles.md` | `DXB-ADP-000` | 범위와 차용 판단 기준 |
| 3 | `10-gap-assessment.md` | `DXB-ADP-010` | DXBOT 현재 상태 대비 gap |
| 4 | `20-extension-lifecycle-and-runtime-composition.md` | `DXB-ADP-020` | Extension/runtime 조립 |
| 5 | `21-local-execution-isolation-and-sandbox-lifecycle.md` | `DXB-ADP-021` | 실행 격리와 sandbox |
| 6 | `22-observability-diagnostics-and-state-backstop.md` | `DXB-ADP-022` | 진단·health·quarantine |
| 7 | `23-human-readable-projections-and-recovery-evidence.md` | `DXB-ADP-023` | read-only projection |
| 8 | `24-multi-bot-messaging-and-wake-semantics.md` | `DXB-ADP-024` | Message delivery와 fresh wake |
| 9 | `25-provider-adapter-conformance-and-usage-accounting.md` | `DXB-ADP-025` | Provider SPI 이질성 증명 |
| 10 | `26-build-provenance-and-artifact-verification.md` | `DXB-ADP-026` | 외부 artifact 검증 |
| 11 | `30-non-adoption-boundaries.md` | `DXB-ADP-030` | 비차용 경계 |
| 12 | `40-integrated-roadmap.md` | `DXB-ADP-040` | 단계별 적용 계획 |
| 13 | `50-acceptance-and-test-plan.md` | `DXB-ADP-050` | test/fault/resource gate |
| 14 | `60-risk-register.md` | `DXB-ADP-060` | 위험과 rollback |
| 15 | `90-source-map.md` | `DXB-ADP-090` | pinned source traceability |
| 16 | `99-validation-report.md` | `DXB-ADP-099` | 두 차례 최종 재검수 |
| 17 | `manifest.md` | `DXB-ADP-MANIFEST` | 패키지 manifest |
| 18 | `manifest.json` | - | machine-readable manifest |
| 19 | `sha256sums.txt` | - | 파일 무결성 checksum |

## 3. 적용 방식

`docs/plan/20260825-0930-grok-architecture-adoption-plan/` 아래에 독립 Reference Snapshot으로 배치한다. 활성 plan으로 승격하거나 Canonical Owner 문서에 반영하기 전에는 비규범적 참고 패키지로 유지한다.

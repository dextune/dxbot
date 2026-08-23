---
title: "Repository·Actual CI·Release 운영 v0.8.9"
document_id: "DXB-ENG-054"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-GOV-001", "DXB-ENG-052", "DXB-ENG-053"]
---
# Repository·Actual CI·Release 운영 v0.8.9

workflow는 active plan validator와 negative self-test를 PR/main에서 read-only로 실행한다. active package는 Accepted `DXB-INDEX` 중 최고 semantic version 하나로 결정한다.

validator는 기존 frontmatter/ID/DAG/manifest/owner/link/exit/command/input/risk 검사에 다음을 추가한다.

- `INV-001~015` exact coverage
- `ADR-0095~0132` exact contiguous coverage
- known local field의 `@local` 강제와 ContentSource `@oneof` 강제
- Acceptance의 단일 최초 milestone
- milestone exit registry와 Acceptance schedule exact relation
- command milestone이 Acceptance 최초 milestone보다 빠른 경우 거부
- current manifest 값을 regex로 변이하는 negative fixture
- local-field leak, invariant 누락, milestone ordering negative fixture

CI가 실행되지 않았거나 실패한 상태에서 Manifest에 Actual CI PASS를 기록하지 않는다. Manifest의 review PASS는 문서 검수 결과이고 Acceptance의 executable/PASS 상태는 실제 artifact, command, fixture hash로만 승격한다.

Validator는 Rust state machine, storage atomicity, real Harness, 모든 자연어 contradiction을 증명하지 않는다.

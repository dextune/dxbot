---
title: "Repository·Actual CI·Release 운영 v0.8.7"
document_id: "DXB-ENG-054"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-GOV-001", "DXB-ENG-052", "DXB-ENG-053"]
---
# Repository·Actual CI·Release 운영 v0.8.7

현재 workflow는 active plan validator와 negative self-test를 PR/main에서 read-only로 실행한다.

v0.8.7 validator는 기존 frontmatter/ID/DAG/manifest/owner/registry/exit/Acceptance/link 검사에 더해 다음을 검사한다.

- Field DSL 기본 문법과 정의되지 않은 `*?`/`+?` 조합 거부
- command kind와 wait set 관계
- `target-terminal` 금지
- Application Command output=`out-operation-v1`
- `applied` wait는 허용 operation allowlist만
- input target/input schema/Acceptance exact relation
- `@local` field가 mutation semantic input으로 오용되지 않는 marker 검사
- milestone freeze registry의 command key 존재와 중복 없음

Validator는 여전히 Rust state machine, storage atomicity, real Harness, 모든 자연어 contradiction을 증명하지 않는다.

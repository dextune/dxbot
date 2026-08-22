---
title: "Repository·Actual CI·Release 운영 v0.8.5"
document_id: "DXB-ENG-054"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-GOV-001", "DXB-ENG-052", "DXB-ENG-053"]
---
# Repository·Actual CI·Release 운영 v0.8.5

## Actual workflow

`.github/workflows/validate-active-plan.yml`은 PR/main에서 read-only로 다음을 실행한다.

```text
python3 scripts/plan-validator.py --active
python3 scripts/plan-validator.py --self-test
```

과거 v0.1 경로/hash를 수정하고 main에 자동 push하는 one-shot workflow는 제거한다.

## Validator

- highest Accepted `DXB-INDEX` package 하나 선택
- frontmatter, lowercase kebab-case, unique ID
- local dependency existence/cycle
- manifest path/version/count
- internal relative links
- command registry와 input registry exact equality
- command Acceptance ID 존재
- source provenance `normative:false`
- active TUI/Web/BFF implementation residue
- built-in duplicate-ID/cycle/missing-input/manifest-drift negative fixture

Rust workspace가 없으면 code architecture/build Gate를 `not executable`로 보고한다. 존재하면 이후 cargo metadata/schema/test Gate를 추가하되 문서 PASS로 대체하지 않는다.

## Release evidence

active package path/version, source baseline commit, validator output, schema hash, build artifact digest, M1A Storage Proof, Harness canary, security/performance results, Review 1/2/3와 미수행 Gate를 기록한다.

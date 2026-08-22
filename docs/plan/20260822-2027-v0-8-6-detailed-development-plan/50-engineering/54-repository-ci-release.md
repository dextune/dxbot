---
title: "Repository·Actual CI·Release 운영 v0.8.6"
document_id: "DXB-ENG-054"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-GOV-001", "DXB-ENG-052", "DXB-ENG-053"]
---
# Repository·Actual CI·Release 운영 v0.8.6

## Actual workflow

`.github/workflows/validate-active-plan.yml`은 PR/main에서 read-only로 다음을 실행한다.

```text
python3 scripts/plan-validator.py --active
python3 scripts/plan-validator.py --self-test
```

trigger는 `docs/plan/**`, `AGENTS.md`, `docs/agent/**`, validator, workflow를 포함한다.

## Validator가 실제 검사하는 것

- highest Accepted active package 하나
- required frontmatter/status/priority/date
- package/version/review metadata와 manifest count
- lowercase kebab-case, unique ID, dependency existence/cycle
- review evidence 5개와 final recheck 2개
- Canonical Owner registry key uniqueness와 owner document existence
- command registry 9-column schema, input registry 5-column schema
- command key/input schema/Acceptance exact relation
- exit code `0,2..18` exact set
- Acceptance table status/evidence/blocker marker
- internal relative links
- duplicate ID/cycle/missing input/malformed registry/manifest/review/exit negative fixture

## Validator가 검사하지 않는 것

Rust architecture, state machine correctness, crash atomicity, authorization, memory bound, Harness behavior와 모든 자연어 contradiction은 executable fixture와 review evidence가 소유한다.

## Release evidence

active package/version, validator output, generated schema hash, build artifact digest, M1A Storage Proof, Contract/Digest fixture, Harness canary, security/audit/performance results, 5회 review와 2회 recheck, 미수행 Gate를 기록한다.

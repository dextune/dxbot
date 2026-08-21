---
title: "Repository·실제 CI·Release 운영"
document_id: "DXB-ENG-054"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-001", "DXB-ENG-052", "DXB-ENG-053"]
---

# Repository·실제 CI·Release 운영

## 1. 목적

현재 active 개발기획 package와 generated contract/architecture를 실제 PR Gate에서 검증하고, 과거 특정 v0.1 hash만 확인하는 one-shot workflow를 release evidence로 오인하지 않게 한다.

## 2. Active Plan Discovery

CI는 hardcoded directory가 아니라 `docs/plan/*-v*-detailed-development-plan/readme.md`의 frontmatter를 읽어:
1. status가 `Accepted`
2. semantic plan version이 가장 높음
3. path timestamp/manifest가 일치함
을 만족하는 package 하나를 active plan으로 선택한다.

동일 최고 버전 또는 Accepted package가 여러 개면 fail한다. enhancement plan 단일 파일은 active detailed package로 선택하지 않는다.

## 3. Plan Validator

Reference logical command:

```text
cargo xtask plan verify --active
```

Rust workspace가 아직 없을 때는 동일 규칙의 bootstrap script를 사용할 수 있으나 최종 owner는 repository-owned deterministic validator다.

검사:
- Markdown inventory/count와 lowercase kebab-case
- frontmatter parse, unique document_id
- depends_on 존재성/cycle
- Accepted/P0 status rule
- hidden previous-version normative inheritance
- Canonical owner/duplicate semantic marker
- TUI/Web/BFF active implementation reference
- P0 Command Matrix completeness/duplicate Application operation alias
- Acceptance/Risk/OQ/ADR/owner orphan
- manifest resolution/hash drift
- internal relative link

## 4. Contract / Generated Gate

```text
cargo xtask contract generate --check
cargo test -p <contract-owner> contract_golden
```

논리적으로 다음 drift를 검출한다.
- public schema snapshot
- request/response/event/error golden
- operation registry
- CLI command metadata/help
- exit registry
- feature/version compatibility matrix

generated output을 수정해 source mismatch를 숨기지 않는다.

## 5. Architecture Gate

- CLI→Domain/Runtime/Storage/concrete Provider forbidden dependency
- Host Lifecycle Client→Domain/Task/Memory/Scheduler/Provider dependency
- Control endpoint→Store direct mutation bypass
- public DTO=Domain/Persistence type sharing
- Provider Host bypass/Dependency Firewall
- unbounded channel/cache static policy 후보
- TUI/Web active crate/package/artifact

가능한 범위에서 cargo metadata/module graph와 compile-fail/architecture tests를 사용한다.

## 6. Contract / Runtime / CLI CI

구현이 존재하는 단계부터:
- format/lint/build/test
- deterministic Reference Provider
- Runtime absent→start→ControlReady smoke
- concurrent start/stale endpoint fixture
- selector/receipt/page/subscription contract suite
- CLI JSON/JSONL/exit/wait/timeout golden
- SIGINT/broken pipe/slow consumer
- endpoint/terminal/export security
- compatibility matrix
- large output/RSS/soak
- Backend identity/memory/task/process/security/resource regression

문서 package merge 시 Rust 구현이 아직 없으면 미실행 Gate를 PASS로 기록하지 않고 `not yet executable`로 구분한다.

## 7. Stale Workflow 정리

현재 v0.1 특정 파일/hash만 확인하는 workflow가 있다면 새 active validator와 병행 기간을 최소화하고, 새 validator가 main에서 재현 가능해진 commit에서 제거/대체한다. stale workflow의 green을 current plan release evidence로 사용하지 않는다.

## 8. Release Evidence

- active plan path/version/package hash
- generated schema/source hash
- compatibility matrix
- forbidden dependency result
- Runtime Host/CLI build artifact digest
- acceptance/fault/security/performance results
- Review 1/2/3 evidence
- Critical Risk residual/owner
- 미수행 Gate와 이유

## 9. Change Safety

- direct main 변경은 repository policy/사용자 요청을 따르되 force push/history rewrite 금지
- logical change 단위 commit
- unrelated file 수정/정리 금지
- secret/local runtime data/artifact commit 금지
- generated source/output을 같은 change set에서 갱신

## 10. 검증 기준

### AT-CI-001
- active v0.8 package 자동 탐색
- intentionally broken ID/DAG/matrix/schema/dependency fixture에서 CI fail
- self-written manifest assertion만으로 PASS 불가
- stale v0.1-only workflow가 current release evidence가 아님
- local/PR에서 동일 failure 재현 가능

---
title: "Repository·CI·Release 운영"
document_id: "DXB-ENG-054"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ENG-050", "DXB-ENG-052", "DXB-ENG-053"]
---

# Repository·CI·Release 운영

## 1. 목적

Repository layout/naming, architecture dependency, removal cleanliness, 문서 추적성을 CI에서 자동 검증한다. 본 문서가 repository naming/layout의 Canonical Owner다.

## 2. Normative Layout

기능이 존재할 경우 기본 소유 위치:

```text
crates/       # core/internal Rust crates
providers/    # replaceable provider implementations
plugins/      # first-party/reference plugin packages
apps/         # deployable applications
docs/         # normative/design/operations docs
tests/        # cross-crate integration/e2e
fixtures/     # versioned test fixtures
migrations/   # persistent schema migrations
benchmarks/   # benchmark code/manifests
examples/     # public API examples
scripts/      # repository automation
schemas/      # public/persistence schemas
generated/    # generated artifacts with declared source
```

실제 기능이 없는데 빈 분류 디렉터리를 미리 만들 필요는 없다.

## 3. Naming Rule

### 기본
- 사용자 정의 directory: lowercase kebab-case
- Markdown/YAML/JSON/schema/script 등 사용자 정의 file stem: lowercase kebab-case
- Rust package name: kebab-case
- Rust module/source `*.rs`: `snake_case.rs`

### Tool-Mandated Allowlist
`Cargo.toml`, `Cargo.lock`, `.github` 등 toolchain/platform이 고정 이름을 요구하는 항목만 명시 allowlist로 관리한다. 관례만을 이유로 사용자 정의 파일명을 임의 예외 처리하지 않는다.

### 금지
- 대소문자만 다른 경로
- whitespace/trailing dot
- Unicode confusable 또는 invisible character로 ASCII path를 위장
- kebab-case Rust source를 유지하기 위한 불필요한 `#[path]`
- 임의 `common`, `utils`, `helpers`, `misc` ownership bucket

## 4. Ownership

- crate/module/doc에 domain/capability owner를 연결
- Provider 구현은 `providers/` 또는 명확한 provider crate owner
- Plugin package는 `plugins/` 또는 별도 plugin release root
- generated는 source-of-truth와 regenerate command 명시
- migration/fixture/schema는 변경 feature와 같은 PR에서 관리

## 5. Fast CI Gate

- format/compile/lint
- naming/layout validator
- Unicode/invisible path guard
- duplicate document_id/depends_on/link
- crate dependency allowlist
- Domain→Provider/Plugin/UI 금지
- Provider→Provider 금지
- unbounded channel / unsafe / production unwrap allowlist
- schema/API/config diff
- generated drift
- requirement/acceptance/risk link

## 6. Full CI Gate

- unit/property
- storage/recovery
- Waiting Continuation / Side Effect crash
- Routine restart
- Fake Harness E2E
- Provider replacement/removal/multi-selection
- Plugin lifecycle/security(P1+)
- migration fixtures
- performance smoke
- minimal/provider-free build
- release packaging

## 7. Removal Cleanliness Gate

Provider/Plugin/Optional Feature 제거 PR은 Cargo dependency/feature, registry entry, config schema, docs/reference, API capability catalog, migration/data cleanup, generated client, fixture, package/SBOM의 orphan을 검사한다. 관련되지 않은 core acceptance가 통과해야 한다.

## 8. 문서 2회 Review Gate

Review 1: structural/naming/ID/depends_on/owner/terminology/manifest/acceptance/risk.

Review 2: cross-layer scenario 정상/교체/제거/복수/crash/cancel/timeout/migration/disable.

두 review evidence가 없으면 Normative 문서 버전을 Accepted로 승격하지 않는다.

## 9. Branch / PR / Release

작은 의미 단위, schema/API/event/config/performance/security/removal 영향, tests, migration/rollback을 PR에 기록한다. Release는 full CI, migration restore, Provider canary, SBOM, checksum/signing, install/upgrade/rollback smoke를 거친다.

## 10. 검증 기준

- AT-REPO-001이 naming/layout 위반을 차단한다.
- Rust `.rs` snake_case와 non-Rust kebab-case가 CI에서 구분된다.
- Unicode/invisible path가 거부된다.
- Provider 제거 후 orphan dependency/registry/config가 0이다.
- docs links/traceability/manifest가 clean하다.
- CI exception은 owner/reason/expiry가 없으면 허용되지 않는다.

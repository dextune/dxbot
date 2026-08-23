---
title: "테스트·검증·5회 적대적 Review와 2회 재검수 전략 v0.8.9"
document_id: "DXB-ENG-052"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ENG-050", "DXB-ENG-051", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ARC-018", "DXB-RUN-033"]
---
# 테스트·검증·5회 적대적 Review와 2회 재검수 전략 v0.8.9

## 필수 deterministic fixture

### Contract/Projection

- active package의 `INV-001~015`와 `ADR-0095~0132` 완전성
- command/input/Acceptance/milestone registry exact relation
- `ready_at`, `timeout`, `all`, `output`, `confirmation`의 `@local` 강제
- ContentSource text/file/stdin → bounded bytes, ArtifactRef → typed ref+digest
- generated CommandPayload에 path/fd/source variant/local field 0
- parser/help/schema/error/exit/output generated exact diff

### Submission/Recovery

- `Prepared → Dispatching` fsync-before-send crash matrix
- command별 single writer, concurrent observer, writer-exit takeover
- Dispatching crash 후 lookup-required와 uncertain eviction 0
- SIGINT, broken pipe, local timeout 뒤 implicit Runtime cancel 0
- same CommandId same/different Principal/key/digest
- same key same/different CommandId/digest
- cross-principal mismatch non-creation/non-disclosure
- full Receipt → tombstone compaction 전후 retry
- acceptance horizon 경계에서 expired key 신규 mutation 오해석 0
- selector/auth 변경 전 binding-first lookup

### State/Security/Domain

- PreAcceptError vs Receipt Rejected vs Task/Delegation Rejected/Deferred
- Approval approve/deny/revoke 후 original operation continuation/recovery
- Membership + AuthorityBinding + AuditIntent atomicity
- Bot create without production Provider, activate/submit provider-unavailable
- endpoint generation/owner/symlink fail-closed
- safe output no-follow/no-replace와 terminal control-sequence escaping

### Delivery/Evidence

- Acceptance 하나당 최초 milestone 하나
- command milestone보다 늦은 Acceptance를 validator가 거부
- milestone exit registry와 Acceptance registry exact schedule
- M2/M3/M4/M5 누적 Gate 없이 subset freeze 불가
- negative fixture가 current value를 실제 변이하는지 검증

## 5회 Review 목적

1. Baseline/Canonical Owner self-contained closure
2. CLI parser/local/wire/digest projection closure
3. crash/retry/recovery/concurrency/security closure
4. command/Acceptance/milestone cumulative Gate closure
5. validator/CI/evidence false-green closure

## 변경 후 별도 재검수

- Structural: package path, frontmatter, ID/DAG, owner, registry, naming, link, scope, manifest
- Cross-Layer Executability: `CliInput → CommandPayload → Journal → Control → Binding → Receipt/Approval → Domain/Storage → Recovery → Projection → CLI`

문서 validator PASS는 generated Rust contract, storage atomicity, authorization correctness, real Harness 실행 PASS를 대체하지 않는다.

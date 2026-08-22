---
title: "테스트·검증·5회 적대적 Review와 2회 재검수 전략 v0.8.6"
document_id: "DXB-ENG-052"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ENG-050", "DXB-ENG-051", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ARC-018", "DXB-RUN-033"]
---
# 테스트·검증·5회 적대적 Review와 2회 재검수 전략 v0.8.6

## Deterministic fixtures

- active package discovery, review metadata, evidence count
- Canonical Owner registry uniqueness
- command/input registry key/schema/Acceptance exact relation
- exit registry exact code set
- RequestDigest/ResolvedBindingDigest canonicalization
- epoch-bearing IdempotencyKey horizon and future skew
- PreparedUnsent/SentUnknown/Abandoned crash windows
- Receipt/Directive/Target wait predicate matrix
- Approval/AuthorityBinding/ActionGrant state and concurrent decision
- Side Effect Unknown/reconcile
- bootstrap UID→Principal/default policy/owner binding atomicity
- storage commit/outbox/receipt/audit-intent crash matrix
- membership CAS and delegated sender resolution
- graceful shutdown vs Host Action
- descriptor-relative atomic no-replace export
- Reference Provider and real Harness Host-path canary

## Acceptance additions

- `AT-CONTRACT-001`: digest, receipt, wait, output, error, exit, operation/input metadata가 exact generated snapshot으로 일치한다.
- `AT-IDEMP-001`: horizon 안 duplicate effect 0, horizon 밖 key는 신규 mutation으로 해석되지 않는다.
- `AT-JOURNAL-001`: every local journal crash/abandon/capacity path가 blind replay 또는 uncertain eviction을 만들지 않는다.
- `AT-SECSTATE-001`: Approval/AuthorityBinding/ActionGrant lifecycle과 CAS/revoke가 deterministic하다.
- `AT-AUDIT-001`: required AuditIntent가 mutation과 원자 결박되고 audit capacity failure가 fail closed한다.

## Five adversarial reviews

1. Governance/self-contained/evidence truth
2. Canonical Owner/state ownership
3. CLI parse-to-output cross-layer contract
4. Crash/concurrency/resource/security
5. Implementation flow/modularity/scope

각 pass는 발견을 수정한 뒤 active package 전체를 처음부터 다시 검사한다.

## Final recheck 1 — Structural

49 Markdown, IDs, dependency DAG, frontmatter, owner registry, command/input/exit/Acceptance/Risk/Manifest/Workflow 관계를 검사한다.

## Final recheck 2 — Cross-Layer Executability

```text
CLI parse → local durable prepare → peer auth → principal/selector resolve
→ Application operation → Domain transition → atomic storage/audit
→ Runtime/Provider Host → receipt/query/event → safe writer/recovery
```

각 단계에 crash, stale revision, revoke, disk pressure, unavailable Provider, slow consumer, endpoint loss, host escalation, export race를 삽입한다.

문서 validator PASS는 Rust fixture PASS를 대체하지 않는다.

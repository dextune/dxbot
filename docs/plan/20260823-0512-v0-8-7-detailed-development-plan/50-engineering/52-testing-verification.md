---
title: "테스트·검증·5회 적대적 Review와 2회 재검수 전략 v0.8.7"
document_id: "DXB-ENG-052"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ENG-050", "DXB-ENG-051", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-ARC-018", "DXB-RUN-033"]
---
# 테스트·검증·5회 적대적 Review와 2회 재검수 전략 v0.8.7

필수 deterministic fixture:

- Prepared→Dispatching fsync-before-send crash matrix
- Dispatching crash 후 lookup-required/uncertain eviction 0
- idempotency/Command binding lookup before selector re-resolution
- same CommandId/different key, same key/different CommandId/digest conflict
- file/stdin materialized digest와 @local exclusion
- PreAcceptError vs Receipt Rejected vs Task Rejected/Deferred
- Approval approve/deny/revoke 후 original operation continuation/recovery
- Membership+AuthorityBinding+AuditIntent atomicity
- operation output/wait relation과 milestone freeze subset
- Bot create without production Provider, activate/submit provider-unavailable

5개 review 목적은 Crash/Submission, State/Approval, Owner/Atomicity, CLI Contract, Roadmap/Evidence로 고정한다. 변경 후 Structural과 Cross-Layer Executability 2회 재검수를 별도로 수행한다.

문서 validator PASS는 generated Rust contract, crash atomicity, security correctness PASS를 대체하지 않는다.

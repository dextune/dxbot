---
title: "Rust 구현 규칙 v0.8.7"
document_id: "DXB-ENG-050"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-011", "DXB-ARC-018", "DXB-IFC-040", "DXB-IFC-041", "DXB-IFC-042", "DXB-RUN-030"]
---
# Rust 구현 규칙 v0.8.7

CLI→Domain/Storage/Runtime internal/concrete Provider, Provider→Application internal을 금지한다.

- `CommandId`, `IdempotencyKey`, `ClientRequestId`는 distinct newtype이다.
- materialized file/stdin bytes 또는 ArtifactRef가 RequestDigest에 들어간다. path/local confirmation/profile/output path는 들어가지 않는다.
- CLI journal `Dispatching` durability 이전 send 금지.
- server binding lookup은 absent일 때만 selector/auth resolve한다.
- application-contract는 schema/metadata, application-operation은 Receipt lifecycle을 소유한다.
- Membership+AuthorityBinding+AuditIntent는 application Unit of Work로 commit한다.
- Bot creation은 Provider readiness와 분리한다.
- all task/channel/cache/buffer는 owner/cancel/join/item+byte cap을 가진다.
- Reference Provider는 explicit deterministic test policy에서만 사용하고 production fallback을 금지한다.

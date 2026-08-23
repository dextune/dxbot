---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-BASE-000"]
---
# 용어집과 도메인 모델

## 상태·Owner 구분

| 용어 | 의미 | Owner |
|---|---|---|
| PreAcceptError | durable operation binding 생성 전의 parse, validation, authentication, key-scope 실패 | Control/Application boundary |
| Operation Receipt | logical operation binding과 commit/recovery 상태 | `application-operation` / `DXB-ARC-014` |
| Domain Outcome | Task, Delegation, Memory 등 canonical aggregate의 결과 상태 | 각 Domain owner |
| Approval | 특정 pending operation/action/target/policy generation의 승인 결정 | `DXB-RUN-032` |
| Local Journal | CLI 송신 전후의 비규범 복구 메타데이터 | `DXB-IFC-041` |
| CliInput | argv, local source handle, rendering/wait/output control을 포함한 CLI 전용 projection | `DXB-IFC-042` |
| CommandPayload | local field와 source handle을 제거하고 materialized semantic만 가진 wire projection | `DXB-IFC-040/042` |

Local Journal은 `Prepared → Dispatching → Observed → Terminal` 또는 `Prepared → Abandoned`다. `Prepared`만 provably-unsent다. `Dispatching` 이상은 Runtime lookup 전 삭제·재송신하지 않는다.

Operation Receipt는 `Accepted → Committed | Rejected | Superseded | RecoveryRequired`다. Approval을 기다리는 operation은 `Accepted`를 유지한다.

`Receipt Rejected ≠ Task Rejected/Deferred`. Task가 생성된 뒤 Domain owner가 `Rejected/Deferred`를 commit했다면 operation 자체는 `Committed`다.

`RequestDigest`는 raw typed selector, materialized payload bytes 또는 ArtifactRef, semantic option, schema/canonicalization version을 결박한다. file path, file descriptor, source variant, profile, `ready_at`, local timeout, `--all`, confirmation, output path, rendering은 digest 대상이 아니다.

`CommandId`, `IdempotencyKey`, `ClientRequestId`는 서로 다른 identity다.

- `CommandId`: Runtime Instance 전역 logical submission identity.
- `IdempotencyKey`: authenticated Principal과 issuance epoch에 결박된 retry identity.
- `ClientRequestId`: 개별 transport 시도 correlation identity.

same CommandId는 same Principal, IdempotencyKey, RequestDigest에만 재사용할 수 있다. same IdempotencyKey도 same CommandId와 RequestDigest에만 결박된다. cross-principal mismatch는 receipt 존재를 노출하지 않는 PreAccept conflict이며 신규 operation을 만들지 않는다.

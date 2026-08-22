---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.8.7"
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
| PreAcceptError | durable operation binding 생성 전의 parser/validation/authentication 실패 | Control/Application boundary |
| Operation Receipt | logical operation binding과 commit/recovery 상태 | `application-operation` / `DXB-ARC-014` |
| Domain Outcome | Task/Delegation/Memory 등 canonical aggregate의 결과 상태 | 각 Domain owner |
| Approval | 특정 pending operation/action digest의 승인 결정 | `DXB-RUN-032` |
| Local Journal | CLI 송신 전후의 비규범 복구 메타데이터 | `DXB-IFC-041` |

Local Journal은 `Prepared → Dispatching → Observed → Terminal` 또는 `Prepared → Abandoned`다. `Prepared`만 provably-unsent다. `Dispatching` 이상은 Runtime lookup 전 삭제·재송신하지 않는다.

Operation Receipt는 `Accepted → Committed | Rejected | Superseded | RecoveryRequired`다. Approval을 기다리는 operation은 `Accepted`를 유지한다.

`Receipt Rejected ≠ Task Rejected/Deferred`. Task가 생성된 뒤 Domain owner가 Rejected/Deferred를 commit했다면 operation 자체는 `Committed`다.

`RequestDigest`는 materialized typed payload bytes/ArtifactRef와 semantic option을 결박한다. file path, local confirmation, output path, profile은 digest 대상이 아니다.

`CommandId`, `IdempotencyKey`, `ClientRequestId`는 서로 다른 identity다. same CommandId는 same IdempotencyKey/RequestDigest에만 재사용할 수 있고, same IdempotencyKey도 same CommandId/RequestDigest에만 결박된다.

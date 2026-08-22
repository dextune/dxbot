---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-014", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-029"]
---
# 동시성·상태 소유권·일관성

## Submission race

CLI는 command file을 exclusive create해 `Prepared`를 fsync한다. materialized payload digest를 확정한 뒤 `Dispatching` record를 append+fsync하고 parent durability가 확보된 다음에만 socket send를 시작한다. crash 후 `Prepared`만 provably-unsent다. `Dispatching` 이상은 lookup-required다.

## Server binding race

authenticate Principal → existing Command/Idempotency binding lookup → existing이면 stored digest 비교/return → absent이면 selector/auth resolve → commit 순서를 강제한다.

CommandId와 IdempotencyKey 어느 한쪽이 기존 다른 counterpart에 결박돼 있으면 conflict다. completion/cancel/membership replace는 revision/generation winner 하나만 commit한다.

Export는 descriptor-relative no-follow/atomic no-replace를 사용하고 stale HostGeneration host-stop을 fence한다.

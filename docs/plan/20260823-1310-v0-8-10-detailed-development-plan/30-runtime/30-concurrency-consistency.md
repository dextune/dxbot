---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-014", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-029"]
---
# 동시성·상태 소유권·일관성

## CLI submission race

command file은 descriptor-relative exclusive create로 만들고 `Prepared` record와 parent directory durability를 확보한다. materialized `CommandPayload`와 RequestDigest를 확정한 뒤 `Dispatching` record를 append+fsync하고서만 socket send를 시작한다.

command별 append lock은 single writer를 강제한다. 최초 creator는 immutable CommandId/key/digest header를 소유한다. 동일 header를 가진 후속 process는 writer lock을 얻기 전 observation-only다. 이전 writer가 종료되어 OS lock이 해제된 뒤에만 takeover할 수 있으며 PID/mtime 추측으로 lock을 강제 회수하지 않는다. header 불일치는 local conflict다.

`Prepared`만 provably-unsent다. `Dispatching` 이상은 lookup-required이며, Runtime binding 조회 전 자동 replay, abandon, delete, capacity eviction을 금지한다.

## Server binding race

```text
authenticate Principal
→ validate key principal/epoch
→ global CommandId lookup + principal IdempotencyKey lookup
→ existing이면 stored binding/digest compare and return
→ 둘 다 absent이면 selector/auth/policy resolve and atomic commit
```

같은 CommandId의 cross-principal 경쟁, same key/different CommandId, same CommandId/different key/digest는 winner 하나만 유지하고 loser는 신규 operation을 만들지 않는다.

completion, cancel, membership replace는 revision/generation winner 하나만 commit한다. create command는 owning namespace uniqueness와 idempotency를 사용하며 global Instance revision CAS로 직렬화하지 않는다.

## Local process termination

SIGINT, SIGTERM, broken pipe, pager 종료, local wait timeout은 CLI의 관찰·렌더링을 종료할 수 있다. 이미 `Dispatching` 이상인 Runtime operation에는 별도 typed cancel/control command 없이 cancellation을 전파하지 않는다. journal은 마지막 durable 상태를 유지한다.

Export는 descriptor-relative no-follow/atomic no-replace를 사용하고 stale HostGeneration host-stop을 fence한다.

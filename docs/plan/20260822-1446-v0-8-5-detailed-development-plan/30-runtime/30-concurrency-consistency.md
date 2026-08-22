---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-014", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-029"]
---
# 동시성·상태 소유권·일관성

## Ownership keys

| 상태 | Owner / key |
|---|---|
| Domain aggregate | Domain ID + revision |
| Operation Receipt | Principal + CommandId/IdempotencyKey + RequestDigest |
| Runtime Instance | InstanceId + HostGeneration |
| Query Snapshot | SnapshotId + watermark + schema |
| CLI submission file | InstanceId + CommandId; non-canonical |
| export temp/publication | opened parent directory FD + destination name |

## Client submission race

각 mutation은 per-command file을 exclusive create하고 content+parent directory를 fsync한 뒤 송신한다. 여러 CLI process가 하나의 JSON map을 atomic replace하는 방식은 금지한다. 같은 CommandId file이 있으면 digest를 비교하고 conflict 또는 resume한다.

## Runtime/command race

same key+same digest는 기존 receipt, different binding은 conflict다. selector resolution 뒤 commit 직전에 current authorization/revision을 재검증한다. completion/cancel/membership replace는 revision/generation으로 winner 하나만 commit한다.

## Export race

path string 검사와 publication 사이의 TOCTOU를 허용하지 않는다. opened parent directory 기준 no-follow 탐색, exclusive temp, atomic no-replace publication을 사용한다. destination가 검사 후 생성되어도 overwrite하지 않는다.

## Runtime stop race

graceful control stop과 host-level escalation은 mode가 다르다. stale HostGeneration을 가진 escalation은 current process를 종료하지 못한다.

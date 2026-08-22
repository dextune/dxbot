---
title: "P0 Storage 실행 프로파일"
document_id: "DXB-ARC-018"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-ARC-015", "DXB-RUN-031"]
---
# P0 Storage 실행 프로파일

## 1. 목적

DB 제품을 선행 고정하지 않으면서 CLI 이전에 반드시 증명해야 할 durability, snapshot, recovery, disk-bound semantic을 정의한다.

## 2. Reference semantic

- 하나의 Runtime Instance에는 active canonical writer가 하나다.
- concurrent reader는 허용하되 writer fencing은 `InstanceId + HostGeneration`을 검증한다.
- aggregate/journal/outbox/receipt binding은 atomic commit boundary를 공유한다.
- snapshot은 long-lived connection/transaction이 아니라 durable watermark와 retained versions에 결박된다.
- compaction은 active snapshot, nonterminal receipt, process, side-effect recovery reference를 삭제하지 않는다.
- disk pressure는 신규 semantic work/snapshot을 admission reject할 수 있으나 committed state를 임의 삭제하지 않는다.

## 3. Snapshot 구현 프로파일

첫 page에서 다음을 고정한다.

```text
SnapshotId
ProjectionWatermark
normalized query/filter/sort/scope/principal/schema digest
last sort key + canonical ID tie-breaker
expiry and retained-byte budget
```

구현은 다음 중 하나를 선택하고 ADR/evidence를 남긴다.

1. versioned projection row/tombstone retention
2. bounded materialized `(sort-key, id, revision)` keyset + revision-addressable value

full result payload를 메모리에 고정하거나 CLI 수명 동안 DB read transaction을 유지하는 방식은 P0 reference가 아니다.

## 4. Receipt recovery

모든 nonterminal receipt는 owner kind, deadline/lease, last progress, reconciliation policy를 가진다. startup에서 owner가 없으면 무기한 Pending으로 두지 않고 `RecoveryRequired` 또는 safe resumed state로 결정한다. capacity 부족 시 새 mutation을 commit 전에 거부한다.

## 5. M1A executable spike

후보 storage adapter는 다음을 실제 process crash/failpoint로 통과해야 한다.

- commit 전/후 crash와 response drop
- same key same/different digest
- outbox/receipt atomicity
- 100 concurrent readers + one writer
- concurrent insert/update/delete 중 snapshot duplicate/missing 0
- snapshot expiry/compaction
- WAL/history/temp disk ceiling
- startup orphan receipt reconcile
- disk-full에서 committed data 보존과 신규 admission 거부
- old fixture open/migration rehearsal

M1A PASS 전 public persistence trait, Command DTO, CLI parser를 freeze하지 않는다.

# Persistence and Recovery Guide

## Applies When

persistent state/schema, transaction, outbox, receipt/idempotency, migration, snapshot, crash recovery를 변경할 때 적용한다.

## Core Rules

Canonical state와 함께 commit돼야 하는 binding/audit/outbox는 한 crash-safe transaction 또는 증명된 atomic protocol을 사용한다. recovery 판정은 transient memory가 아니라 durable state와 Stable Contract에서 수행한다.

schema 변경은 forward migration, restart safety, partial resume, backup/recovery, irreversible step, rollback limitation, old fixture load를 검토한다. projection/index/cache는 rebuild 가능해야 한다.

## Decision Rules

atomicity가 여러 Owner를 가로지르면 Application Unit of Work가 coordination을 소유하되 각 Domain state를 별도 authoritative copy로 만들지 않는다.

## Forbidden Patterns

partial commit을 외부에 success로 노출, old receipt reinterpret, migration 중 silent data loss, CLI가 canonical data migration 직접 수행, crash window를 mock만으로 대체하는 것을 금지한다.

## Verification

commit 직전/직후 crash, response loss, duplicate delivery, migration interruption/restart, disk-full, snapshot/compaction, old fixture, orphan recovery를 실제 failpoint로 검증한다.

## Related Guides

[../quality/concurrency-fault-testing.md](../quality/concurrency-fault-testing.md)

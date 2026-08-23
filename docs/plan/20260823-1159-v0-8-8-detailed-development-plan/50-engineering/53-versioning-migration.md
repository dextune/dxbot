---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-IFC-040", "DXB-ARC-015", "DXB-ENG-052"]
---
# 버전·호환성·Migration

## 1. 버전 축

```text
runtime_version / client_version
protocol_major.minor / schema_major.minor
operation_schema_version / public_event_schema_version
data_schema_version / provider-capability version
request_digest_canonicalization_version
idempotency_key_format_version
```

하나의 제품 버전 숫자로 lockstep하지 않는다.

## 2. Pre-release compatibility policy

P0 구현과 M6 freeze 전에는 숫자 기반 N-minor 호환성 창을 규범화하지 않는다. 같은 major의 additive change도 generated schema diff와 semantic fixture를 통과한 경우에만 compatible로 판정한다.

첫 stable release에서 maintenance capacity, receipt/idempotency horizon, migration fixture, client/runtime matrix evidence를 근거로 compatibility window를 ADR로 freeze한다.

## 3. Command/Receipt/Idempotency

- RequestDigest canonicalization version을 Receipt와 함께 기록한다.
- selector/action/target/idempotency semantic 변경은 major 또는 explicit migration이다.
- IdempotencyKey format version과 issuance epoch를 검증한다.
- receipt lookup retention과 key acceptance horizon은 별도 policy지만 expired key를 신규 mutation으로 취급하지 않는 invariant는 유지한다.
- old Receipt를 reinterpret하지 않는다.

## 4. Cursor/JSON/Exit

cursor는 Instance, query, scope, principal visibility, snapshot, schema에 결박된다. ordering/filter semantic이 바뀌면 explicit Query restart다.

machine field 제거/의미 변경과 numeric exit code 의미 재사용은 breaking change다. additive optional field는 fixture가 허용한 같은 major minor change일 수 있다.

## 5. Data migration

persistent schema 변경은 forward migration, restart safety, partial resume, backup/recovery, irreversible step, rollback limitation, old fixture load를 Gate로 둔다. CLI가 data migration을 직접 수행하지 않는다.

## 6. Snapshot policy

accepted contract release는 generated schema/golden/source hash를 commit한다. release snapshot을 수정하지 않고 새 version을 만든다. generator semantic drift는 CI failure다.

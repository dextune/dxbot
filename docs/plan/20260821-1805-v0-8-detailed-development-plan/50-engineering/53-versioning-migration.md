---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-ARC-015", "DXB-ENG-052"]
---

# 버전·호환성·Migration

## 1. 목적

Persistent Domain data, Runtime binary, public protocol/schema, CLI, cursor, receipt의 호환성 단위를 분리하고 silent downgrade나 identity rewrite를 막는다.

## 2. 버전 축

```text
runtime_version
client_version
protocol_major.minor
schema_major.minor
operation_schema_version
public_event_schema_version
data_schema_version
provider/capability contract version
```

하나의 제품 버전 숫자로 lockstep하지 않는다.

## 3. P0 Compatibility Window

Reference release line에서 다음을 지원한다.

- 동일 protocol/schema major 안의 additive minor 변경은 backward-compatible할 수 있다.
- Runtime은 current minor와 직전 **2개 minor**의 negotiated public schema를 지원한다. 구현 전 minor가 2개 미만이면 존재하는 범위만 지원한다.
- CLI는 current Runtime과 직전 **1개 Runtime minor**의 advertised compatible range를 대상으로 contract fixture를 유지한다.
- 이전 major는 기본적으로 incompatible이며 explicit migration/bridge ADR 없이는 연결하지 않는다.
- unknown optional read/event field는 동일 major에서 무시 가능하다.
- unknown required mutation field/operation/receipt state는 explicit incompatible error다.

숫자 window 변경은 compatibility/maintenance 비용 evidence와 ADR을 요구한다.

## 4. Domain Data 비회귀

v0.8 문서 고도화 자체는 BotId, ConversationId, ThreadId, ProjectId, ChannelId, TaskId, ExecutionId, MemoryId, ProcessId, Directive, Side Effect, ActionGrant data migration을 요구하지 않는다.

실제 구현에서 persistent schema가 바뀌면:
- forward migration
- crash/restart safety
- partial migration resume
- backup/recovery
- irreversible step
- rollback/downgrade limitation
- old fixture load
을 별도 Gate로 둔다. CLI upgrade가 data migration을 직접 수행하지 않는다.

## 5. Command / Receipt Compatibility

- RequestDigest canonicalization/version은 receipt와 함께 기록한다.
- selector/action/target/idempotency binding의 semantic 변경은 schema major다.
- old receipt는 compatibility window에서 OperationId로 조회 가능해야 한다.
- new Runtime이 old key binding을 reinterpret하지 않는다.
- terminal receipt retention 30일은 compatible client recovery window보다 짧아지지 않는다.
- expired receipt는 `operation-expired`; automatic new mutation fallback 금지.

## 6. Cursor Compatibility

cursor는 instance, query, scope, principal visibility, snapshot, schema에 결박된다. schema major/InstanceId를 넘어 portable하지 않다. additive minor에서 ordering/filter semantic이 같을 때만 continuation을 허용한다. 그 외는 explicit restart Query다.

## 7. JSON/JSONL / Exit Compatibility

- machine field 제거/의미 변경은 major breaking change다.
- additive optional field는 minor다.
- terminal status와 exit class mapping은 committed golden fixture다.
- numeric exit code를 재사용해 다른 의미를 부여하지 않는다.
- human output/help 변경은 machine compatibility를 의미하지 않는다.

## 8. Schema Snapshot Policy

각 accepted contract release는 generated schema/golden/source hash를 commit한다. release된 snapshot은 수정하지 않고 새 version을 생성한다. drift는 CI failure다. generator 변경이 semantic output을 바꾸면 schema diff와 compatibility 판정을 요구한다.

## 9. Runtime/CLI Upgrade

- old CLI ↔ current compatible Runtime
- current CLI ↔ previous compatible Runtime
- incompatible pair explicit error
- host service upgrade와 Control readiness를 구분
- rolling multi-host/HA는 P0 비범위
- Runtime restart 후 Domain identity 유지

## 10. Supersession / Future Interface

TUI/Web 문서는 active package에 없으며 data migration 대상이 아니다. 새 Interface는 Headless Contract를 재사용하고 자체 duplicate schema/Authorization state를 만들지 않는 별도 version plan을 요구한다.

## 11. 검증 기준

- protocol/schema/runtime/client/data version 혼용 0.
- compatible window fixture가 양방향 semantic을 검증.
- incompatible mutation/cursor explicit failure.
- receipt recovery가 supported window에서 유지.
- v0.8 plan 도입만으로 Backend ID rewrite/data migration 0.

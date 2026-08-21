---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-ARC-016", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-036", "DXB-IFC-040"]
---

# 버전·호환성·Migration

## 1. 목적

Persistent Bot/Memory에 Main Conversation/Thread/Directive/Suspension을 추가하면서 v0.3 data/API/Provider compatibility를 보호한다. v0.3 Capability/Provider SDK/Plugin versioning discipline을 그대로 유지한다.

## 2. 독립 버전 대상

기존 Runtime, Event, DB/Snapshot/Memory, API, Config/Profile, Artifact, Capability Contract, Provider SDK/package/config, Conformance, Plugin, Projection에 추가:
- Conversation/Thread event/schema
- Conversation Message schema
- Thread lineage schema
- Thread Memory scope schema
- Task Specification revision/control epoch
- Control Directive schema
- Suspension/checkpoint schema

## 3. v0.3 → v0.4 Data Migration 원칙

- 기존 Bot마다 Main Conversation exactly one을 idempotent하게 생성/연결
- legacy conversation/message data의 provenance 보존
- legacy `Session`을 Thread로 자동 승격하지 않음
- Provider Session ID를 ThreadId로 변환하지 않음
- Thread가 명확하지 않은 legacy Task/Message는 explicit `legacy-unassigned`/migration mapping 정책을 사용
- 기존 Canonical Memory를 임의 Thread-local로 강등하지 않음
- 새 scope metadata가 없는 기존 Memory는 기존 Bot-global semantic을 보존하는 것이 기본 안전 방향
- migration은 restart-safe/idempotent이며 partial marker를 가진다

정확한 physical schema와 legacy mapping은 storage migration ADR/fixture에서 확정한다.

## 4. Event / API Compatibility

v0.4에서 새 Conversation/Thread/Control event/API resource를 additive하게 추가하는 것을 우선한다. 기존 `Session` field가 의미 혼합을 일으키면 deprecate→replacement field→migration window를 제공한다.

redirect/suspend 등의 new command가 기존 cancel/retry semantics를 재정의하지 않는다.

## 5. Provider / Harness Compatibility

Provider Session resume, cancellation, yield, native steering은 optional negotiated features다. 기존 Provider가 이를 지원하지 않는다는 이유만으로 Capability semantic breaking으로 처리하지 않는다. Common Runtime은 unsupported safe fallback/awaiting-safe-point semantics를 가진다.

Provider Session token schema 변경이 Thread/Conversation migration을 요구해서는 안 된다.

## 6. Suspension / Directive Schema

Continuation/Side Effect처럼 P0 persistent schema로 version decoder/migration fixture를 둔다. old Directive schema를 decode할 수 없을 때 running Task를 임의 mutation하지 않고 explicit recovery-required/quarantine를 사용한다.

## 7. Rollback

binary/config/schema/data/external Side Effect/Provider/Plugin과 함께 Conversation/Thread migration을 구분한다.

rollback 시 이미 생성된 Thread/Conversation history를 유실하지 않는다. v0.3 binary가 새 v0.4 records를 이해하지 못하면 downgrade-blocked를 명시한다.

## 8. 검증 기준

- v0.3 fixture DB/Event/Task/Memory를 v0.4로 migration 후 동일 BotId/Memory semantics 유지.
- Main Conversation duplicate 생성 0.
- legacy Session/Provider Session이 ThreadId로 오인 변환되지 않음.
- partial migration crash 후 idempotent restart.
- incompatible Directive/Suspension schema가 silent resume하지 않음.
- 기존 Provider removal/Contract compatibility gate 유지.

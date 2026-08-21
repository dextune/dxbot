---
title: "버전·호환성·Migration"
document_id: "DXB-ENG-053"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-015", "DXB-IFC-040"]
---


# 버전·호환성·Migration

## 1. 목적

Persistent Bot과 장기 Memory가 코드 업그레이드로 손상되지 않도록 데이터, Event, API, Config, Plugin/Harness의 버전과 Migration 규칙을 정의한다.

## 2. 책임 범위

- 버전 대상과 호환성
- schema/event up/down migration
- rolling/stop-the-world upgrade
- preview dependency pinning
- import/export compatibility
- rollback와 data backup
- deprecation

## 3. 버전 대상

각각 독립 버전:
- Runtime binary
- Domain event schemas
- DB schema
- Snapshot schemas
- Memory record/revision schema
- Control API
- Config/profile schema
- Artifact manifest
- Harness Adapter protocol
- Provider/plugin manifest
- Bot export bundle
- Projection/index generation

하나의 global version 숫자로 모든 호환성을 표현하지 않는다.

## 4. 호환성 규칙

- Reader는 지원 범위의 older schema를 읽음
- Writer는 현재 schema만 씀
- unknown major/critical variant는 fail/quarantine
- additive optional field를 우선
- semantic change는 field 재사용 금지, 새 field/version
- Event는 immutable; 과거 row를 임의 재작성하지 않음
- migration 결과와 source digest
- Projection/Index는 rebuild 가능하므로 별도 generation
- API는 client/server capability negotiation

## 5. Event Migration

선호 순서:
1. 여러 version decoder + current domain representation
2. 필요 시 explicit transformation during load
3. offline canonical migration은 저장 비용/지원 범위 때문에 필요한 경우
4. 원본 backup과 migration ledger

Upcaster chain이 무한히 길어지지 않게 지원 window와 compaction checkpoint를 관리한다. Event 의미를 잃는 migration은 허용하지 않는다.

## 6. DB Migration

단계:
1. preflight와 free space
2. backup/snapshot
3. application compatibility 확인
4. additive schema
5. backfill
6. dual-read 또는 verification
7. switch current
8. old schema cleanup은 후속 release
9. integrity/replay test

단일 노드 P0는 maintenance mode를 허용한다. P2 rolling upgrade는 expand/contract 패턴을 사용한다.

## 7. Snapshot/Projection

- Snapshot은 invalid이면 Event replay로 폐기 가능
- projection/index는 generation을 바꾸고 shadow rebuild
- 전환 전에 count/digest/invariant 비교
- rollback은 이전 generation으로 pointer 전환
- stale projection을 current로 오인하지 않음
- migration 중 query에 rebuilding/watermark 표시

## 8. API와 Client

- supported version range
- command/event type capability catalog
- deprecation response headers/metadata
- unknown enum safe handling
- generated client golden tests
- old client의 destructive command는 stricter compatibility
- event cursor에 schema/projection generation
- frontend/CLI가 server mismatch를 명확히 표시

## 9. Config Migration

- deprecated key와 replacement
- 자동 변환은 diff/backup 제공
- unknown key silent ignore 금지
- security default가 바뀌면 명시적 release note와 safe migration
- Bot profile override가 새 default를 의도치 않게 상쇄하지 않는지 검사
- secret ref format migration
- last-known-good config

## 10. Harness/Plugin 호환성

DeepSeek Harness 같은 preview upstream:
- exact version/digest pin
- Adapter protocol fingerprint
- compatibility matrix
- canary profile
- golden/conformance trajectory
- raw upstream event quarantine
- automatic latest upgrade 금지
- Adapter rollback
- resume token/version 호환성 확인

Plugin:
- manifest API range
- capability contract version
- config schema
- migration hook의 권한 제한
- unload/rollback
- signed digest(P2)

## 11. Export/Import

Bundle manifest:
- format version
- source runtime version
- Bot identity/Memory/Goal selection
- event/snapshot inclusion
- artifact digests
- encryption/key references
- required capabilities
- excluded secrets/permissions
- provenance
- checksum/signature 후보

Import는 dry-run과 conflict report를 제공한다. 원본 BotId 보존/새 ID 발급 정책을 명시한다.

## 12. Rollback

Rollback 가능 여부:
- binary only
- config
- schema additive
- data transformed
- external side effect
- Harness/provider
를 구분한다.

irreversible migration은:
- 별도 backup
- 검증 query
- maintenance window
- owner approval
- abort point
- recovery procedure
가 필요하다.

## 13. 예외상황

- unknown Event가 특정 Bot에만 존재: Bot quarantine, 전체 Runtime read 가능
- migration crash: ledger marker로 재개/rollback
- free space 부족: 시작 전 중단
- old binary가 new schema 접근: compatibility guard
- mixed node version: supported matrix 밖이면 ownership/admission 차단
- sidecar resume token incompatible: 새 Execution attempt
- export artifact missing: partial bundle reject 또는 명시적 degraded import
- deprecated API consumer 미확인: telemetry 기반 deprecation 연장/차단 결정

## 14. 확장성

multi-node rolling upgrade에서도 aggregate writer version을 명확히 관리한다. shard별 migration 상태와 routing gate를 둔다. 장기 archival Event는 decoder artifact와 schema catalog를 함께 보존한다.

## 15. 구현 우선순위

- **P0:** DB/Event/API/Config version, migration ledger, backup, startup guard
- **P1:** compatibility matrix, export/import, canary Harness, projection shadow rebuild
- **P2:** rolling upgrade, multi-node mixed version, plugin signing
- **P3:** automated migration planning

## 16. 검증 기준

- 과거 fixture DB/Event를 현재 Runtime이 복원한다.
- migration 중 kill 후 재시작이 데이터 중복/손실 없이 완료된다.
- unknown major event가 silent ignore되지 않는다.
- downgrade 불가능한 변경이 배포 전에 표시된다.
- preview Harness upgrade가 conformance/canary 없이 활성화되지 않는다.
- export/import 후 Identity/Memory digest와 provenance가 검증된다.

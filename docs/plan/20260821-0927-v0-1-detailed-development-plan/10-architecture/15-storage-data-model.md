---
title: "저장소와 데이터 모델"
document_id: "DXB-ARC-015"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-011"]
---


# 저장소와 데이터 모델

## 1. 목적

Persistent Bot의 정체성과 기억을 안전하게 보존하면서 단일 노드 P0 구현의 단순성, 향후 다중 노드 확장, 메모리·I/O 효율을 균형 있게 달성한다.

## 2. 책임 범위

- 저장 계층 분리
- 논리 테이블/컬렉션
- 트랜잭션, WAL/Checkpoint, 인덱스
- Artifact와 검색 인덱스
- 백업, 정합성, 데이터 수명

특정 SQL 문법과 Migration 파일은 구현 단계에서 정의한다.

## 3. 저장 계층

| 계층 | Canonical 여부 | 용도 |
|---|---|---|
| Domain Journal | Canonical | 상태 변경 사실과 순서 |
| Current State Tables | Canonical Materialization | 빠른 Aggregate load, expected revision |
| Memory Records/Revisions | Canonical | 기억 내용, 출처, 정책, 변경 이력 |
| Outbox/Inbox | Canonical Delivery State | 내구성 비동기 전달 |
| Artifact Store | Canonical Content | 큰 파일·trace·결과 |
| Read Projections | Derived | Control/UI 조회 |
| Search Index | Derived | lexical/vector/metadata recall |
| Cache | Ephemeral | hot read 최적화 |

동일 필드가 여러 저장소에 있더라도 Canonical Owner를 명시한다. Derived 저장소는 rebuild 절차가 필수다.

## 4. 초기 저장 전략

P0는 Embedded transactional store를 기본으로 한다.
- 한 Runtime 인스턴스가 writer 권한을 가진다.
- crash-safe journal과 transaction을 사용한다.
- blocking I/O는 async executor와 분리한다.
- DB connection/statement/cache 상한을 둔다.
- 개발 편의를 위해 임의 JSON 파일을 Canonical Store로 사용하지 않는다.

다중 노드 Store는 P2로 미루되 Store Port가 DB의 세부 타입을 Domain에 노출하지 않도록 한다.

## 5. 논리 데이터 모델

### 5.1 Core Tables
- `bots`: current identity, lifecycle, revision, timestamps
- `bot_events`: aggregate event envelope
- `goals`: current goal state
- `tasks`: task header, owner, status, priority, revision
- `task_edges`: dependency/delegation graph
- `executions`: attempts, outcome, provider, trace reference
- `core_leases`: admission, owner, expiry, terminal state
- `messages`: durable Bot envelope
- `inbox_receipts`: consumer deduplication
- `outbox`: publish state and retry metadata

### 5.2 Memory
- `memory_items`: stable identity, class, scope, current revision
- `memory_revisions`: immutable content/provenance/policy
- `memory_links`: typed relation and weight
- `memory_tombstones`: deletion/retention enforcement
- `memory_index_queue`: derived index update work
- `context_checkpoints`: selected memory IDs/digests, not duplicate content

### 5.3 Operation
- `artifacts`: metadata, digest, location, encryption, ref count/retention
- `projection_checkpoints`
- `idempotency_records`
- `schema_migrations`
- `runtime_locks`
- `audit_records` 또는 별도 append store

## 6. 데이터 표현 원칙

- Hot lookup 필드는 정규화된 column으로 저장한다.
- 변화가 잦거나 드문 event-specific payload는 versioned structured payload로 저장한다.
- 한 레코드에 전체 Bot/Memory graph를 직렬화하지 않는다.
- 문자열 enum은 migration 가독성을 위해 허용하되 unknown 처리 계약을 둔다.
- 큰 텍스트, model delta, tool output, binary는 Artifact로 분리한다.
- digest를 통해 content identity와 corruption을 검증한다.
- timestamp는 UTC 기준 저장, UI에서 locale 변환한다.
- secret 원문은 저장하지 않고 Secret reference와 audit만 저장한다.

## 7. 트랜잭션 경계

### Command Commit
- current revision read/lock
- domain event append
- current state update
- idempotency result
- outbox insert
를 하나의 transaction으로 처리한다.

### Memory Commit
- current memory revision 확인
- dedup/conflict 검증
- new immutable revision insert
- item current pointer update
- relation update
- index queue insert
를 하나의 transaction으로 처리한다.

Artifact 본문 저장이 별도 시스템이면:
1. temp upload + digest
2. DB transaction에 pending reference
3. commit 후 finalize
4. 실패 시 orphan sweeper
패턴을 사용한다.

## 8. 읽기와 캐시

- Aggregate load는 current state + 필요한 recent events만 읽는다.
- Projection query는 cursor 기반 pagination을 사용한다.
- offset pagination은 작은 관리 화면에만 제한한다.
- cache key에 tenant/bot/policy/version을 포함한다.
- negative cache와 stale cache도 TTL/크기 상한을 둔다.
- cache invalidation은 Event revision 기반으로 하고 시간만 의존하지 않는다.
- Memory content를 중복 캐시할 때 Arc/immutable blob reference를 우선한다.

## 9. 예외상황

- DB locked/busy: bounded retry와 jitter, command deadline 초과 시 실패
- 디스크 full: 신규 write admission 중지, read-only degraded mode, 명확한 alert
- checksum mismatch: record quarantine, backup/replay 시도, silent repair 금지
- migration 중 crash: transactional migration 또는 단계별 marker로 재개
- Artifact 존재하지 않음: dangling reference 표시와 repair queue
- Index 지연: canonical lookup은 가능하며 검색 결과에 watermark 표시
- tombstone된 Memory가 cache/index에 잔존: query-time policy filter로 즉시 차단 후 async 제거
- outbox poison message: quarantine와 수동 재처리, 전체 큐 정지 금지

## 10. 백업과 복구

- backup에는 DB snapshot, Artifact manifest, encryption metadata, schema version이 포함된다.
- 복구는 새 위치에서 checksum과 replay를 검증한 후 원본을 대체한다.
- backup 중 write consistency를 보장한다.
- 복구 리허설을 자동화하고 RPO/RTO는 배포 등급별로 정의한다.
- Derived Index/Projection은 백업 필수가 아니며 재구축 시간을 측정한다.

## 11. 확장성

- BotId 또는 TenantId+BotId를 shard key 후보로 유지한다.
- Artifact는 content-addressed backend로 분리 가능하다.
- Search Index는 local에서 external로 교체 가능하다.
- multi-writer 전환 전 Aggregate expected revision과 idempotency contract를 그대로 보존한다.
- 장기 Journal은 hot/cold tier로 이동하되 audit/replay pointer를 유지한다.

## 12. 구현 우선순위

- **P0:** embedded DB, schema migration, Bot/Task/Event/Memory, outbox, backup/restore smoke
- **P1:** Artifact store, search index queue, snapshots, repair/doctor
- **P2:** external DB, remote artifact, archival, shard-ready keys

## 13. 검증 기준

- 전원 차단/kill injection 후 committed Event가 유실되거나 중복되지 않는다.
- 동일 Memory의 concurrent update가 revision conflict를 정확히 반환한다.
- Projection/Index를 삭제하고 canonical data에서 재구축한다.
- 100k Memory/Task 기준 query latency와 RSS가 예산 내다.
- disk full, DB busy, artifact orphan 시나리오가 자동화되어 있다.
- backup 복원 후 Bot Identity, Memory digest, Task graph가 원본과 일치한다.

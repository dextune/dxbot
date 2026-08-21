---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 핵심 제품 용어

| 용어 | 정의 | Canonical Owner |
|---|---|---|
| Bot | Session과 독립된 Persistent Identity·Memory·State 엔티티 | `DXB-DOM-020` |
| Brain | 하나의 Bot에서 Identity·Memory·Goal·실행 능력을 연결하는 logical semantics | `DXB-DOM-021` |
| Core Lease | Scheduler가 임시로 부여하는 bounded 실행 권한 | `DXB-DOM-024` |
| Conversation | Bot 또는 Channel의 persistent message parent | `DXB-DOM-027` |
| Thread | Conversation 안의 독립 lineage/context branch | `DXB-DOM-027` |
| Task | 수행할 Canonical work intent | `DXB-DOM-023` |
| Execution | immutable Task revision을 실제 수행하는 attempt | `DXB-DOM-023` |
| Durable Process | 여러 aggregate의 장기 진행·waiting·replay를 조정하는 owner | `DXB-RUN-038` |
| Memory Scope | Bot/Project/Channel/Thread 지식 경계와 provenance | `DXB-DOM-022` |

## 2. v0.8 Contract 용어

| 용어 | 정의 | 수명/Owner |
|---|---|---|
| Runtime Instance | 하나의 `InstanceId`와 persistent data root에 결박된 Runtime 배포 단위 | persistent / `DXB-RUN-035` |
| HostGeneration | 한 Runtime Instance의 현재 process incarnation을 fence하는 단조 증가 generation | process / `DXB-RUN-035` |
| ResourceSelector | ID 또는 명시적 scope 안의 exact name/alias로 대상을 요청하는 입력 | request / `DXB-IFC-040` |
| Resolved ResourceRef | kind, Canonical ID, scope, revision/generation을 가진 확정 대상 | response/command / `DXB-IFC-040` |
| CommandId | logical mutation을 최초 전송 전에 식별하는 stable ID | receipt retention / `DXB-IFC-040` |
| IdempotencyKey | 동일 logical mutation 재전송을 기존 outcome에 결박하는 key | receipt retention / `DXB-IFC-040` |
| RequestDigest | principal/action/target/payload/schema를 정규화해 key mismatch를 검출하는 digest | receipt / `DXB-IFC-040` |
| Operation Receipt | mutation의 durable 접수·진행·결과·복구 상태 | durable / `DXB-IFC-040`, storage `DXB-ARC-015` |
| Query Snapshot | paged Query가 일관되게 읽는 revision/watermark 경계 | cursor retention / `DXB-IFC-040` |
| Page Cursor | query/filter/sort/scope/principal/snapshot/schema에 결박된 opaque continuation | bounded / `DXB-IFC-040` |
| Partial Machine Stream | 일부 record가 이미 출력되었으나 정상 완료가 아닌 stream | invocation / `DXB-IFC-041` |
| Terminal Record | JSONL stream의 complete/partial/gap/error와 last safe cursor를 고정하는 마지막 record | stream / `DXB-IFC-040/041` |
| Contract Schema Source | public DTO/error/exit/help schema를 생성하는 Rust source | build / `DXB-ENG-053/054` |

## 3. 반드시 구분할 개념

```text
CLI Process ≠ Runtime Host Process
CLI Session ≠ Conversation ≠ Thread
Host Lifecycle Operation ≠ Application Command
Operation Receipt ≠ Authorization Grant
ResourceSelector ≠ Resolved ResourceRef
Command ≠ Query ≠ Subscription ≠ Domain Event
Page Cursor ≠ Event Cursor
Observed Cursor ≠ durable business state
Protocol Version ≠ Schema Version ≠ Runtime Version ≠ Data Schema Version
Public DTO ≠ Domain Struct ≠ Persistence Row
Role ≠ Authority ≠ Authorization Decision ≠ Local Confirmation
Knowledge Memory ≠ Runtime Memory
```

## 4. Selector와 Scope

Canonical ID가 최종 식별자다. name/alias는 explicit scope 안에서 exact match가 하나일 때만 resolve된다. profile/current context는 scope 입력 편의일 뿐 authority나 Canonical truth가 아니다. fuzzy match와 implicit last-used target은 P0 mutation에 사용하지 않는다.

## 5. Operation 상태

`Accepted → Pending/AwaitingSafePoint → Committed` 또는 `Rejected | Superseded | RecoveryRequired`로 표현한다. `Accepted`, `Pending`, `Committed`는 서로 다른 성공 조건이며 CLI가 하나의 `ok`로 뭉개지 않는다.

## 6. 검증 기준

- 위 구분을 깨는 shared type/ID/boolean shortcut 0
- selector가 authority를 생성하는 경로 0
- receipt가 expected revision/authorization을 대체하는 경로 0

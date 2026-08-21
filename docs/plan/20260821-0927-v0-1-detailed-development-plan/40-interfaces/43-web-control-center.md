---
title: "Web Control Center"
document_id: "DXB-IFC-043"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P2"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-IFC-040", "DXB-RUN-032"]
---


# Web Control Center

## 1. 목적

단순 채팅 UI가 아니라 다수 Bot, Core, Task, Memory, 통신, 자원, 권한을 통합 관찰·운영하는 Control Center를 제공한다.

## 2. 책임 범위

- 정보 아키텍처와 화면군
- Web BFF/read composition
- real-time state
- 보안/세션/approval
- graph/trace visualization
- 대규모 목록과 성능
- 배포/버전 분리

Web UI는 Runtime과 완전히 분리되고 Control API만 사용한다.

## 3. 화면 구조

### Overview
- Bot 상태와 health
- Task/Core queue
- resource/cost
- incidents/approvals
- provider/index/projection health

### Bot Detail
- identity/role/profile
- goals/tasks
- memory overview
- communication
- resource/permission
- activity timeline

### Execution
- Task DAG
- Core lanes
- model/tool trajectory
- result/artifact
- retry/cancel/approval
- correlation trace

### Memory
- search/filter
- revision/provenance
- relations/conflicts
- compaction/index
- sensitivity/retention/forget

### Network
- Bot topology
- message/delegation flow
- failed/expired messages
- trust/capability

### Administration
- providers/capabilities
- resource policy
- users/roles
- config/version/migration
- audit/backup/maintenance

## 4. Frontend 경계

- generated API client 또는 shared schema
- client-side store는 UI projection
- Domain 상태기계 재구현 금지
- command mutation은 server response/event로 확정
- BFF는 read aggregation만 수행하며 write rule을 만들지 않음
- Web-specific formatting을 API core에 넣지 않음
- offline-first Bot state editing은 P0/P1에서 지원하지 않음

## 5. 실시간 동기화

- initial snapshot + event stream
- per-view subscription filter
- sequence/cursor
- reconnect/resync
- optimistic pending state
- server watermark/stale 표시
- high-frequency model delta는 opt-in/coalesced
- browser hidden 상태에서 stream 감쇠
- multiple tabs는 독립 client이나 command idempotency 사용

## 6. Control UX 원칙

- Bot과 Core를 시각적으로 구분
- Session 종료와 Bot 삭제를 혼동시키지 않음
- destructive action은 scope/impact/dry-run
- stale data에서 mutation 시 expected revision
- long-running operation의 진행/취소/결과
- partial/degraded 상태를 단순 성공/실패로 숨기지 않음
- actual usage와 estimate 구분
- 권한 때문에 숨겨진 데이터와 실제 0건 구분
- trace 원문 접근은 별도 권한

## 7. Graph 시각화

Bot Network, Task DAG, Memory relations는 다음을 지킨다.
- server-side pagination/neighborhood query
- depth/node budget
- lazy expansion
- cycle/edge type 구분
- layout computation worker/off-main-thread
- 전체 graph 자동 load 금지
- stable IDs와 selected neighborhood cache
- large graph fallback table

## 8. Web 보안

- secure session/token
- CSRF, origin, content security policy
- XSS-safe rendering; model/tool output를 raw HTML로 삽입 금지
- Artifact download content disposition/type
- authorization은 server에서 재검증
- sensitive content screen/clipboard/export controls
- approval request digest와 reauthentication
- admin action audit reason
- rate limit
- multi-tenant isolation
- browser storage에 secret/trace 원문 최소화

## 9. 접근성과 반응형

- keyboard navigation
- screen reader labels
- 색상 외 상태 표현
- reduced motion
- large text/zoom
- desktop control center 우선이나 좁은 viewport에서는 핵심 monitoring/approval 제공
- dense graph는 대체 table
- locale/timezone 명시

## 10. 성능·메모리

- route-level code split
- virtualized lists/tables
- normalized client cache
- content/artifact lazy load
- stream event coalescing
- bounded client history
- browser memory watermark
- worker for graph/large parsing
- response pagination/compression
- cache key에 permission/user/scope 포함
- stale cache에서 민감 데이터 재노출 방지

## 11. 예외상황

- API unavailable: 명확한 read-only/offline banner, destructive action 비활성
- event gap: resync
- auth expiry: sensitive state clear 후 재인증
- permission revoked: cache invalidation
- partial bulk: item result
- projection stale: watermark 표시
- browser tab sleep: cursor resume
- upload interruption: resumable 또는 temp cleanup
- incompatible frontend/backend: hard compatibility screen
- malformed model output: escaped text/controlled renderer

## 12. 확장성

Frontend는 별도 배포 가능하며 API version contract로 독립 릴리스한다. organization/multi-tenant view는 scope를 확장하되 Bot 내부 의미를 바꾸지 않는다. plugin UI는 sandboxed extension surface와 signed manifest가 준비된 뒤 P3에서 검토한다.

## 13. 구현 우선순위

- **P2:** overview, Bot/Task/Core, approval, health, basic Memory/Network
- **P3:** advanced graph/trajectory, policy analytics, plugin UI, mobile operations

## 14. 검증 기준

- Web이 DB 또는 Runtime 내부 Port에 직접 접근하지 않는다.
- Core가 별도 Bot처럼 생성/삭제되는 UX가 없다.
- 10k row/large graph에서 client memory와 interaction latency가 예산 내다.
- XSS payload가 model/tool output에서 실행되지 않는다.
- permission revoke 후 cached sensitive data가 재표시되지 않는다.
- frontend/backend version mismatch가 silent corruption 없이 차단된다.

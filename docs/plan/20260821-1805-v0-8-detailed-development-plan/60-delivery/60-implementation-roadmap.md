---
title: "통합 구현 로드맵과 단계별 Gate"
document_id: "DXB-DEL-060"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-010", "DXB-IFC-040", "DXB-IFC-041", "DXB-ENG-052"]
---

# 통합 구현 로드맵과 단계별 Gate

## 1. 목적

Rust workspace가 비어 있는 상태부터 Persistent Backend와 Headless Application Contract, Runtime Host, CLI를 하나의 실행 가능한 dependency DAG로 연결한다. 존재하지 않는 fixture를 전제하거나 CLI command를 먼저 만든 뒤 Backend shortcut으로 맞추는 방식을 금지한다.

## 2. 통합 Dependency DAG

```text
Workspace / Kernel
→ Domain identity/state minimum
→ Persistence / Journal / Recovery minimum
→ Provider Host + deterministic Reference Provider
→ Bot Main Conversation / Task minimum
→ Application Contract Kernel
→ Runtime Instance / Host / Control Endpoint / Client
→ Bot-only CLI Vertical Slice
→ Project / Channel Vertical Slice
→ Durable Process / Recovery / Streaming / Automation
→ Compatibility / Security / Performance / Release Freeze
```

각 milestone은 이전 단계의 executable evidence를 입력으로 사용하며 문서만으로 구현 완료를 선언하지 않는다.

## 3. M0 — Effective Baseline / Scope Freeze

### 작업
- v0.8 active 47-file package와 Canonical owner map 승인
- hidden inheritance resolution과 P0/P1/P2 scope freeze
- P0 Command Matrix 100% mapping
- P0 Accepted document set
- active plan validator bootstrap
- TUI/Web/BFF/remote/HA active reference 제거

### Exit
- AT-BASE-001
- AT-CLI-008 문서/validator 조건
- unresolved inheritance 0
- dependency cycle/duplicate owner 0
- active TUI/Web artifact/milestone 0

## 4. M1 — Workspace / Domain / Persistence / Contract Kernel

### 구현
- Rust workspace와 logical dependency boundary
- kernel ID/revision/time/unit/error values
- Bot/Main Conversation/Thread/Task 최소 Aggregate
- journal/state/outbox/idempotency crash-safe persistence
- deterministic Reference Provider와 Provider Host 최소 경로
- Application Command/Query/Subscription public type source
- ResourceSelector/ResolvedResourceRef
- Operation Receipt/RequestDigest/key binding
- snapshot Page/Cursor와 public stream terminal/error schema
- schema/golden generator

### Vertical executable slice

```text
CreateBot command
→ canonical Bot/Main Conversation commit
→ response drop failpoint
→ same operation retry/query
→ same identity/outcome 회수
```

### Exit
- AT-APP-001/002/003/005
- AT-SCHEMA-001 최소 schema
- same key/different digest conflict
- direct Store/Scheduler/Provider interface path 0
- OQ-082~087 resolved decisions 반영

## 5. M2 — Runtime Instance / Host / Local Security

### 구현
- Linux user-scoped service adapter
- XDG config/data/state/runtime paths
- InstanceId/data root metadata
- single-instance lock/HostGeneration fencing
- stale lock/socket/descriptor recovery
- readiness `ProcessStarted→StorageRecovered→RuntimeReady→ControlReady`
- authenticated UDS Control Endpoint/Client
- peer credential/principal mapping
- terminal sanitizer와 safe file writer foundation

### Vertical executable slice

```text
runtime absent
→ concurrent runtime start
→ one ControlReady instance
→ status/version Query
→ graceful stop/restart
→ same Bot identity
```

### Exit
- AT-HOST-001
- endpoint portion of AT-SEC-005
- unsafe runtime directory/instance mismatch fail closed
- Host Lifecycle Client Domain shortcut 0

## 6. M3 — Bot-Only CLI Vertical Slice

### 구현 흐름

```text
install/config
→ runtime start
→ bot create
→ conversation show/send/history
→ thread create/branch
→ task submit/show/watch/control/result
→ operation show/reconcile
→ memory get/search/history/propose/promote
→ Runtime restart
→ same identity/result/receipt 복구
```

### 필수 CLI 계약
- P0 global options/exit registry
- deterministic selector/ambiguity
- JSON/JSONL stdout/stderr
- accepted/committed/terminal wait
- local timeout/SIGINT independence
- page/stream bounded writer

### Exit
- AT-CLI-001~006
- AT-CLI-009/010
- response-loss/SIGINT/stale selector fault injection
- Backend shortcut/client policy duplicate 0

## 7. M4 — Project / Channel Vertical Slice

### 구현 흐름

```text
project create
→ channel create
→ membership/Role/Authority
→ channel message/thread
→ task/delegation/collaboration process
→ scoped memory proposal/promotion
→ result/evidence
```

### Gate
- Project/Channel은 Brain/Task/Memory assertion owner가 아님
- Membership/Role/Authority/Authorization 분리
- ambiguous participant/resource selector 0
- Private→Shared flow/declassification
- fan-out/queue/bytes/task bounded
- participant revoke/failure/restart recovery

### Exit
- Backend Project/Channel/Memory/Security regression
- CLI P0 Project/Channel matrix rows E2E
- AT-SEC-005 information-flow portion

## 8. M5 — Recovery / Streaming / Automation

### 구현
- snapshot page + `--all`
- cursor integrity/expiry/reauth/restart
- subscription at-least-once/gap/resync
- JSONL header/item/event/terminal
- partial output/last safe cursor/exit 13
- operation/side-effect reconcile
- slow/broken pipe/subscriber cleanup
- safe export/temp/atomic rename/cleanup
- Runtime restart/endpoint recreation/resync

### Exit
- AT-APP-006/007
- AT-CLI-010 full suite
- AT-SEC-005 terminal/export suite
- large output/resource/subscriber leak 0
- partial stream complete 오판 0

## 9. M6 — Freeze / CI / Release Planning

### 구현·검증
- P0 normative docs/contracts Accepted 상태와 generated version freeze
- compatibility current + previous windows
- actual active plan/schema/architecture CI
- full Backend regression
- Runtime Host + CLI packaging/install smoke
- security/performance/soak evidence
- Review 1/2/3 전체 package와 구현 evidence
- P1/P2 backlog 분리

### Exit
- AT-SCHEMA-001/AT-CI-001
- Critical Risk mitigation evidence
- active TUI/Web/Plugin ecosystem artifact 0
- implementation-ready baseline과 executable P0 release evidence 승인

## 10. Milestone Dependency와 병렬화

- M0은 모든 구현의 전제다.
- M1 Domain/Persistence와 Contract schema는 Vertical Slice 범위에서 함께 진행할 수 있으나 public schema를 실제 use-case 없이 전량 일반화하지 않는다.
- M2 Host/Endpoint는 M1의 최소 status/version/receipt contract를 사용한다.
- M3가 common contract를 실사용해 보정하기 전 M4 command surface를 대량 구현하지 않는다.
- M5 page/stream foundation 일부는 M1에 만들 수 있으나 full automation freeze는 M3/M4 representative workloads 뒤다.
- M6는 모든 이전 Gate의 executable evidence를 요구한다.

## 11. 비목표

TUI, Web Control Center, BFF는 active 구현 범위가 아니다.

- TUI/Web/BFF/frontend state
- distributed Runtime/consensus/HA
- multi-tenant IAM 전체
- generic RPC/IDL/workflow framework
- all command tree simultaneous P0
- Provider/Plugin lifecycle CLI 전체
- specific allocator/Graph DB 필수화
- speculative crate/module explosion

## 12. 검증 기준

- 각 milestone 선행 Gate와 executable vertical slice 존재.
- 미구현 Backend fixture를 전제로 한 단계 0.
- P0 command가 one Application operation에 연결.
- Runtime memory/security/recovery correctness보다 CLI 장식 기능이 앞서지 않음.
- M0~M6가 Acceptance/Risk/Resolved Decision에 추적됨.

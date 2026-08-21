---
title: "동시성·상태 소유권·일관성"
document_id: "DXB-RUN-030"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-010", "DXB-ARC-014", "DXB-ARC-017", "DXB-DOM-024", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029"]
---

# 동시성·상태 소유권·일관성

## 1. 목적

다수 Bot/Project/Channel/Thread/Core/Provider와 동시에 발생하는 Membership·Role·Authority·Memory·Supervisor 변경이 data race, scope leak, stale authority, duplicate promotion, unbounded fan-out을 만들지 않도록 race winner와 fencing 규칙을 고정한다.

## 2. Single Writer / Immutable Read

- Bot/Project/Channel/Conversation/Thread/Task/Memory Canonical write는 owner Command + expected revision/generation을 사용한다.
- Execution/Core는 immutable/versioned snapshot을 읽고 Result/Proposal을 반환한다.
- running Execution의 Context Plan/Task spec/Provider binding/selected Memory revision을 mutation하지 않는다.
- Runtime routing/presence/control signal은 durable Canonical State를 대체하지 않는다.
- Derived cache/index candidate는 current authorization authority가 아니다.

## 3. Ownership Matrix

| 상태 | Owner | 동시성 방식 |
|---|---|---|
| Bot state | Bot/Application coordinator | revisioned single write |
| Project/Membership | Project Domain | revision/generation + idempotency |
| Channel/Membership/Role/Authority | Channel Domain | revision/generation + expected version |
| Main/Channel Conversation + Thread | Conversation/Thread owner | revision + idempotency |
| Scoped Memory | Memory subsystem | ScopeRef + immutable revision/conflict |
| Task/spec/SupervisorRef/Continuation | Task owner | expected revision + UoW |
| Control Directive | Control/Task command owner | DirectiveId + target revision/generation |
| Execution snapshot | Execution owner | immutable + fencing |
| Core Lease | Scheduler | lease/fencing |
| Channel Presence Runtime | Orchestration/Projection | derived/rebuildable |
| Provider lifecycle/activity | Provider Host/Lifecycle | generation/activity guard |

## 4. Queue / Channel

다음은 모두 item+byte cap, overflow/backpressure, close/send failure semantic을 가진다.
- Scheduler Work Queue
- Runtime Control Channel
- Channel inbound event queue
- participant routing/fan-out queue
- Bot Network delegation queue
- history/Memory retrieval buffer
- Provider Host buffer/stream
- event subscriber buffer

한 Channel 또는 Bot의 flood가 unrelated Bot/Channel의 admission을 무기한 점유하지 않는다.

## 5. v0.4 Race 유지

redirect vs complete/cancel, suspend vs complete/Side Effect, duplicate resume, reprioritize vs admission, Thread archive vs Task, Thread branch vs update, simultaneous Supervisor commands, Provider drain/lifecycle, Side Effect Unknown, Lease expiry의 기존 semantic을 모두 유지한다.

## 6. v0.5 신규 Race

### Membership Revoke vs Execution
- revoke commit 후 **신규** Channel/Project Memory/history/artifact read/write/control은 거부한다.
- 이미 생성된 immutable Execution Context를 rewrite하지 않는다.
- high-risk Side Effect가 아직 commit/external effect 전이면 policy에 따라 current authorization을 다시 확인한다.

### Role/Authority Change vs Manager Control
- routing 시점 snapshot이 과거 generation이어도 mutation commit 직전 current Authority generation을 확인한다.
- stale generation의 redirect/suspend/delegate command는 conflict/rejected다.

### Channel Archive vs Running Task
- archive가 Task를 암묵 cancel하지 않는다.
- new normal routing/admission은 archive policy에 따라 차단할 수 있다.
- 기존 Task는 explicit control 또는 continuation policy로 처리한다.

### Thread Memory Promotion vs Source Update
- PromotionProposal은 source Memory revision/digest를 pin한다.
- source가 변경되면 stale proposal은 재평가/conflict다.

### Simultaneous Promotion
- source digest + target Scope + normalized content/relationship key를 이용한 dedup과 expected revision/conflict policy를 사용한다.

### Channel Routing vs Membership Change
- Coordinator는 bounded routing snapshot을 만들 수 있다.
- 실제 dispatch/control/write 전 current membership/authority를 재검증한다.
- revoked participant를 stale routing list만으로 깨우지 않는다.

### Manager Redirect vs Research Completion
- Task/spec/Supervisor/Authority revision과 existing v0.4 Execution fencing이 semantic winner를 결정한다.
- late research result가 redirected revision을 덮지 않는다.

### Shared Memory Update vs Context Creation
- Context Plan은 선택된 immutable Memory revisions를 pin한다.
- concurrent new Memory가 old Execution prompt를 mutation하지 않는다.

## 7. Scope Isolation

- Project A/B, Channel A/B, Thread A/B ScopeRef는 별도 canonical authorization을 가진다.
- Bot-global Memory는 Shared Scope로 자동 복제되지 않는다.
- final canonical authorization은 index/cache 이후에 수행한다.

## 8. Lock / Async

- async mutex guard를 외부 I/O await 동안 보유하지 않는다.
- routing/authorization snapshot을 mutable global map lock에 장시간 묶지 않는다.
- child task는 owner/cancellation/join path를 가진다.
- safe-point/control signal은 짧은 critical section을 사용하고 atomics 도입은 model-test 근거를 가진다.

## 9. 검증 기준

- 신규 race 각각 deterministic barrier/fake clock fixture가 있다.
- revoke 이후 stale cache/index/routing에서 정보/권한 누출 0.
- promotion duplicate가 Canonical Memory explosion을 만들지 않는다.
- Channel archive가 running Task를 자동 terminal로 만들지 않는다.
- shutdown 후 queue/task/permit/activity/presence leak 0.

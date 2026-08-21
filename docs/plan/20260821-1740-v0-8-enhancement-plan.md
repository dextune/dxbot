---
title: "DXBOT v0.8 개발기획 고도화 플랜"
document_id: "DXB-ENH-008"
version: "0.1.0"
status: "Draft Enhancement Plan"
normative: false
priority: "P0"
last_updated: "2026-08-21"
depends_on:
  - "DXB-BASE-000"
  - "DXB-IFC-040"
  - "DXB-IFC-041"
  - "DXB-ENG-052"
  - "DXB-ENG-054"
  - "DXB-DEL-060"
  - "DXB-DEL-061"
  - "DXB-MANIFEST"
---

# DXBOT v0.8 개발기획 고도화 플랜

## 1. 목적

본 문서는 `docs/plan/20260821-1628-v0-7-detailed-development-plan`에 대한 3회 전체 적대적 리뷰 결과를 바탕으로, v0.7을 **CLI까지 임의 판단 없이 개발 가능한 구현 기준선**으로 고도화하기 위한 v0.8 문서 작업 계획을 정의한다.

v0.8의 목표는 새로운 제품 기능을 늘리는 것이 아니다. v0.7이 이미 확정한 다음 방향을 유지하면서 구현 계약의 공백을 닫는 것이다.

> **Persistent DXBOT Runtime → Headless Application Contract → CLI Reference Interface**

핵심 문제는 Backend 개념의 부족이 아니다. 현재 문서는 금지 경계와 장기 원칙은 강하지만, 실제 CLI 개발에 필요한 대상 식별, 명령 결과, 재시도, Runtime instance, cursor, partial output, local security, schema source와 실제 CI가 충분히 닫혀 있지 않다.

v0.8은 이를 해결하는 **Resolved Implementation Baseline + CLI Contract Closure** 버전으로 정의한다.

---

## 2. 기준선과 리뷰 결론

### 2.1 기준선

- Source baseline: `docs/plan/20260821-1628-v0-7-detailed-development-plan`
- Backend semantic baseline: v0.6에서 v0.7이 상속한 모든 활성 Canonical Owner
- Active product Interface: Headless Application Contract + CLI
- TUI/Web: v0.8에서도 active implementation/release scope가 아님
- Change classification: Tier A — Common/Application Contract 및 구현 기준선
- 본 작업의 직접 산출물: 개발기획 문서
- 본 작업 자체는 Rust 구현 완료를 의미하지 않음

### 2.2 적대적 리뷰의 최종 판정

v0.7은 다음 면에서 충분히 강하다.

- Persistent Bot, Single Brain, Dynamic Core Lease, Memory, Durable Process 의미
- CLI→Domain/Store/Scheduler/Provider 직접 접근 금지
- Runtime Host와 CLI process lifetime 분리
- Command/Query/Subscription 분리
- bounded paging/streaming과 Runtime Memory 연계
- Common Authorization/Resource policy의 client-side 중복 금지
- TUI/Web/BFF/frontend state 선행 구현 금지

그러나 다음 이유로 전체 CLI 구현 기준선으로는 부족하다.

1. 이전 버전 상속으로 유효 요구가 여러 패키지에 분산된다.
2. 구현을 따라야 할 핵심 문서가 `Draft` 상태다.
3. 미구현 Backend부터 CLI까지 하나의 실행 가능한 Dependency DAG가 없다.
4. P0 CLI command 범위와 각 명령의 Contract mapping이 고정되지 않았다.
5. Resource selector와 ambiguous target 처리 규칙이 없다.
6. response-loss 이후 Operation Receipt를 CLI에서 복구하는 계약이 불완전하다.
7. Runtime instance, endpoint, data/config/runtime directory와 single-instance 의미가 없다.
8. pagination snapshot/order/cursor와 stream partial/terminal 의미가 불완전하다.
9. accepted/pending/committed, wait/timeout, machine error의 CLI 의미가 닫혀 있지 않다.
10. local IPC, terminal control sequence, file export 공격면이 빠져 있다.
11. public DTO/schema의 단일 생성 원본이 없다.
12. 현재 실제 CI가 v0.7 문서와 선언한 Gate를 검증하지 않는다.

---

## 3. v0.8 목표 상태

v0.8 완료 시 다음이 참이어야 한다.

1. v0.8 active package 하나만 읽어 현재 구현 계약을 해석할 수 있다.
2. 이전 버전은 역사·변경 근거이며 활성 요구를 조립해야 하는 Normative source가 아니다.
3. 모든 P0 CLI command가 하나의 Application operation에 매핑된다.
4. 각 command/query/subscription의 입력·출력·revision·idempotency·wait·stream·error가 구현 가능 수준으로 고정된다.
5. lost response와 CLI crash 뒤에도 기존 Runtime receipt에서 mutation 결과를 복구할 수 있다.
6. Resource ID/name/scope selector가 deterministic하고 ambiguous mutation을 거부한다.
7. Runtime Host instance와 bootstrap lifecycle이 platform reference 기준으로 닫힌다.
8. pagination과 subscription이 누락·중복·gap·partial output을 명시적으로 표현한다.
9. JSON/JSONL/stdout/stderr/exit class가 automation contract로 동작한다.
10. local endpoint, terminal, export file의 P0 threat model과 Acceptance가 존재한다.
11. public contract schema가 하나의 source에서 생성·검증된다.
12. 문서·schema·dependency·scope Gate가 실제 CI에서 실행된다.
13. Bot-only Vertical Slice를 먼저 검증하고, 그 위에 Project/Channel 및 Recovery slice를 확장한다.
14. TUI/Web/Plugin ecosystem/분산 HA를 이유로 v0.8 공통 계약을 과도하게 일반화하지 않는다.

---

## 4. 비목표

v0.8 문서 고도화에서 다음을 새로 설계하거나 선행 구현하지 않는다.

- TUI 또는 Web Control Center
- BFF, view/pane/route schema, frontend state store
- 다중 사용자·다중 tenant IAM 전체
- 분산 Runtime, consensus, remote HA
- 범용 Workflow DSL
- 범용 RPC/IDL framework 자체 개발
- 여러 transport를 위한 추상화 계층 폭증
- 전체 command tree의 동시 P0 구현
- Provider/Plugin lifecycle 관리 기능 전체를 CLI P0로 승격
- 특정 allocator나 Graph DB의 필수화
- CLI 편의를 위한 Domain/Runtime 정책 복제
- 과거 문서 내용을 그대로 반복하는 새 Canonical Owner 문서

새 Canonical 문서 추가는 기본값 `0`으로 한다. 기존 문서가 의미를 소유할 수 없는 경우에만 Review 1에서 추가 필요성을 증명한다.

---

## 5. v0.8 설계 원칙

### 5.1 Self-contained Effective Baseline

v0.8은 strict-superset 참조 상속을 구현자가 직접 조립하는 방식에서 벗어난다.

- v0.8 active 문서는 현재 유효한 계약을 문서 안에 직접 포함한다.
- 이전 패키지는 역사 및 변경 근거로 참조할 수 있으나 active semantic source가 아니다.
- 동일 `document_id`의 현재 의미는 v0.8 문서 하나가 소유한다.
- `manifest.md`는 이전 요구의 보존·변경·supersession 결과를 resolution matrix로 기록한다.
- `생략 = 상속`을 v0.8 active 구현 규칙으로 사용하지 않는다.

문서 중복을 늘리라는 의미가 아니다. 각 규칙은 하나의 Canonical Owner에만 완전하게 남기고 다른 문서는 ID/상대 링크로 참조한다.

### 5.2 Vertical Slice Before Surface Expansion

전체 Contract와 command tree를 한 번에 freeze하지 않는다.

```text
Contract Kernel
→ Runtime Instance / Host / Control Boundary
→ Bot-only Vertical Slice
→ 실제 CLI E2E를 통한 공통 계약 보정
→ Project/Channel Vertical Slice
→ Recovery/Streaming/Automation
→ Compatibility Freeze
```

CLI command를 먼저 다량 생성한 뒤 Backend shortcut을 만드는 것도 금지하고, 모든 public DTO를 실제 use-case 없이 미리 일반화하는 것도 금지한다.

### 5.3 Typed and Narrow Contracts

- generic `control <json>` mutation 금지
- generic `process control`이 typed operation을 우회하지 않게 함
- CLI alias는 Application operation을 복제하지 않음
- 동일 use-case는 하나의 typed Command/Query/Subscription으로 귀결
- Runtime/Domain/Provider 내부 타입은 public DTO로 재사용하지 않음

### 5.4 Local-first, Future-compatible

v0.8은 하나의 reference deployment/platform에서 완전한 동작을 먼저 닫는다. 미래 remote/TUI/Web 가능성은 Headless Contract의 책임 경계로 보존하되, 현재 사용하지 않는 다중 transport·frontend abstraction을 추가하지 않는다.

---

## 6. P0 고도화 작업

## 6.1 P0-01 — Effective Baseline 단일화

### 문제

v0.7은 v0.6/v0.5/v0.4의 반복하지 않은 요구를 계속 상속한다. 구현자와 리뷰어가 서로 다른 문서 조합을 읽어도 모두 준수했다고 주장할 수 있다.

### 작업

- v0.8의 모든 active Canonical 문서를 현재 계약으로 materialize
- 이전 버전 normative inheritance 문구 제거 또는 역사 참조로 격하
- `readme.md`에 active document set과 owner map 명시
- `manifest.md`에 아래 resolution 기록
  - preserved
  - clarified
  - changed
  - superseded
  - removed from active scope
- 동일 규칙의 중복 원본 제거
- 모든 active `depends_on`을 v0.8 package 내부 ID로 해소

### 완료 조건

- v0.8 package 밖의 문장을 읽지 않아도 P0 구현 계약을 판정 가능
- unresolved inheritance 0
- 동일 semantic Canonical Owner 중복 0
- dependency cycle 0

---

## 6.2 P0-02 — 구현 승인 상태와 Freeze Gate

### 문제

현재 핵심 Contract와 roadmap 문서가 `Draft`이며, 문서 거버넌스상 구현 강제력이 확정되지 않았다.

### 작업

v0.8에서 최소 다음 문서를 P0 Freeze 대상으로 지정한다.

- `DXB-BASE-000`
- `DXB-GOV-001~004`
- `DXB-ARC-010/011/014/015`
- `DXB-RUN-030~035`
- `DXB-IFC-040/041`
- `DXB-ENG-050/052/053/054`
- `DXB-DEL-060~063`
- `readme.md`
- `manifest.md`

P0 미결정 사항이 남은 문서는 `Accepted`로 승격하지 않는다. P1/P2 문서는 `Draft`로 유지할 수 있으나 P0 Release Gate를 암묵 차단하지 않게 한다.

### 완료 조건

- 구현 착수 기준 문서 상태가 명시됨
- P0 Open Question 미해결 문서의 허위 Accepted 0
- Accepted 문서가 owner/test/acceptance를 가짐

---

## 6.3 P0-03 — Backend부터 CLI까지 통합 Dependency DAG

### 문제

v0.7 M1~M5는 v0.6 Backend fixture가 이미 존재하는 것처럼 보이지만 현재 저장소에는 Rust Runtime 구현이 없다.

### 작업

`DXB-DEL-060`에 다음 dependency를 하나의 roadmap으로 통합한다.

```text
Workspace / Kernel
→ Domain identity/state minimum
→ Persistence / Journal / Recovery minimum
→ Provider Host + deterministic Reference Provider
→ Bot Main Conversation / Task minimum
→ Application Contract Kernel
→ Runtime Host composition / Control Endpoint / Client
→ CLI Bot-only Vertical Slice
→ Project/Channel Vertical Slice
→ Durable Process / Recovery / Streaming
→ CLI hardening / release
```

기존 Backend 세부 Gate를 복사하지 않고 milestone prerequisite와 exit evidence만 연결한다.

### 완료 조건

- 각 v0.8 milestone의 선행 Backend Gate가 명확
- 존재하지 않는 fixture를 전제로 한 단계 0
- 각 milestone이 최소 하나의 실행 가능한 Vertical Slice를 생성

---

## 6.4 P0-04 — CLI P0 Command Matrix Freeze

### 문제

현재 command tree는 넓지만 어떤 명령이 P0인지와 실제 Application operation mapping이 없다.

### 작업

`DXB-IFC-041`이 다음 열을 가진 Command Matrix를 Canonical하게 소유한다.

| 열 | 의미 |
|---|---|
| CLI path | 확정 command/subcommand |
| Priority | P0/P1/P2 |
| Application operation | 단일 typed owner |
| Kind | Command/Query/Subscription/Host Lifecycle |
| Target selector | 필요한 ResourceRef/scope |
| Required revision | expected revision/generation |
| Idempotency | key/receipt 필요 여부 |
| Wait behavior | immediate/accepted/committed/follow |
| Input source | argv/stdin/file/structured |
| Human output | 핵심 표현 |
| Machine output | schema/version |
| Exit/error class | stable class |
| Security | permission/approval/confirmation |
| Resource | page/chunk/byte bound |
| Acceptance | 연결된 AT ID |

### 권장 P0 기본 범위

- `runtime`: start/status/stop/doctor
- `version`
- `bot`: create/list/show/activate/deactivate/archive/restore
- `conversation`: show/send/history
- `thread`: create/list/show/history/branch
- `task`: submit/list/show/watch/cancel/suspend/resume/redirect/result
- `memory`: get/search/history/propose/promote
- `project`: create/list/show/archive/restore
- `channel`: create/list/show/join/leave/members/send/history
- `process`: show/watch
- `operation`: show/reconcile
- `approval`: list/show/approve/deny — 실제 P0 high-risk flow에 필요한 최소 범위
- `provider`: list/show — read-only 진단 범위
- `reconcile`: doctor가 해결하지 못하는 P0 복구 경로만

### P1/P2 기본 분류

- Goal/Routine 전체 관리
- Core watch 및 상세 운영
- Provider deprecate/drain/detach
- Plugin install/upgrade/rollback/uninstall
- Capability test
- Memory compact/index 운영
- 범용 trace export
- 광범위한 repair mutation

### 완료 조건

- P0 command coverage 100%
- P1/P2를 포함한 active CLI 후보의 분류 누락 0
- matrix에 없는 P0 CLI command 0
- 하나의 Application operation을 중복 구현하는 alias 0
- generic JSON mutation escape hatch 0
- P1/P2는 폐기 항목이 아니라 명시적 후속 CLI backlog이며 P0 Release Gate와 분리됨

---

## 6.5 P0-05 — Resource Addressing과 Selector 규칙

### 문제

Bot/Project/Channel/Thread/Task를 ID, 이름, alias, profile context 중 무엇으로 선택하는지 정의되지 않았다.

### 작업

`DXB-IFC-040/041`에 다음을 고정한다.

- Canonical ID가 최종 식별자
- name/alias selector는 명시한 scope 안에서 유일할 때만 resolve
- fuzzy match를 P0 mutation selector로 사용하지 않음
- ambiguous selector는 후보 목록을 포함한 explicit error
- destructive/high-risk mutation은 resolved ID와 expected revision을 요구
- implicit “last used” target을 mutation authority로 사용하지 않음
- profile/current context는 입력 편의일 뿐 Canonical state가 아님
- machine output은 resolved ResourceRef를 반환
- Project/Channel/Bot scope precedence를 하나의 규칙으로 고정
- selector resolution 후 authorization과 revision을 Runtime이 재검증

### 완료 조건

- 동일 이름 fixture에서 잘못된 대상 mutation 0
- scope 없는 ambiguous name의 silent selection 0
- client local cache가 target truth가 되는 경로 0

---

## 6.6 P0-06 — Operation Receipt와 Idempotency Closure

### 문제

response-loss 재시도 원칙은 있으나 CLI에서 Command 결과를 복구하는 구체 surface와 key binding이 없다.

### 작업

`DXB-IFC-040`이 `OperationReceipt` 또는 동등한 typed 계약을 소유한다.

최소 의미:

```text
CommandId / OperationId
IdempotencyKey
PrincipalRef
Action
Target ResourceRef
RequestDigest
Protocol/Schema Version
SubmittedAt
State
Canonical Outcome Ref?
Result/Error Class?
Retry/Reconciliation Disposition
Receipt Revision
Retention/Expiry Policy Ref
```

최소 상태 의미:

```text
Accepted
Pending / AwaitingSafePoint
Committed
Rejected
Superseded
RecoveryRequired
```

규칙:

- 동일 key + 동일 request digest는 기존 receipt/outcome을 반환
- 동일 key + 다른 principal/action/target/payload/schema digest는 conflict
- receipt가 authorization 또는 expected revision을 대체하지 않음
- receipt retention/GC가 policy로 명시됨
- CLI는 `operation show/reconcile`로 uncertain mutation을 조회
- CLI는 network transmission 전에 CommandId를 생성
- CLI local journal을 둘 경우 ID/digest/profile/timestamp만 bounded하게 보존하며 Runtime receipt가 source of truth
- local journal에 secret/full payload/authority state를 저장하지 않음
- accepted와 committed를 동일 success 의미로 뭉개지 않음

### 완료 조건

- commit-after-response-loss duplicate effect 0
- CLI restart 뒤 uncertain operation 조회 가능
- key-payload mismatch가 explicit conflict
- receipt expiry 이후의 복구 의미 명시

---

## 6.7 P0-07 — Runtime Instance와 Host Lifecycle 모델

### 문제

Runtime Host와 CLI process 분리는 정의됐지만 Runtime instance granularity, endpoint/data ownership, concurrent start가 닫히지 않았다.

### 작업

v0.8 M2 이전 ADR에서 최소 다음을 확정한다.

- P0 reference platform 또는 platform tier
- per-user / system / workspace Runtime 중 기본 instance 모델
- InstanceId와 workspace/data directory 관계
- config/data/runtime/socket/log directory ownership
- endpoint discovery
- single-instance lock와 fencing
- concurrent `runtime start` winner
- stale PID/socket/lock 복구
- start vs start, start vs stop, stop vs restart race
- readiness 단계와 timeout
- graceful shutdown → escalation 조건
- service manager kill과 Domain cancel/reconcile 구분
- instance mismatch 시 CLI explicit error
- endpoint path permission과 peer identity

### 완료 조건

- concurrent start에서 active Runtime 1개
- stale endpoint가 다른 data directory에 연결되는 경로 0
- process handle이 Task/Execution control handle로 사용되는 경로 0
- Runtime restart 뒤 Domain identity 유지

---

## 6.8 P0-08 — Pagination / Snapshot / Cursor 계약

### 문제

bounded pagination은 정의됐지만 ordering, snapshot consistency, cursor binding, partial page 의미가 없다.

### 작업

`DXB-IFC-040`이 다음을 고정한다.

- 모든 paged Query의 stable primary sort와 tie-breaker
- cursor가 query kind/filter/sort/scope/snapshot-or-watermark/schema에 결박됨
- snapshot-consistent와 live/eventual Query를 명시적으로 구분
- authorization/policy 변화 시 cursor continuation 규칙
- stale/expired/foreign cursor explicit error
- page 중복·누락 허용 여부
- total count는 optional이며 full materialization을 요구하지 않음
- `--all`은 page-by-page incremental writer
- page 실패 시 이미 출력된 record의 유효 범위와 last cursor
- large export의 temporary file/atomic rename/partial cleanup

### 완료 조건

- concurrent insert/delete 중 silent duplicate/missing을 current snapshot으로 오판하지 않음
- 다른 filter/scope에서 cursor 재사용 0
- `--all` CLI RSS가 전체 dataset에 비례해 증가하지 않음

---

## 6.9 P0-09 — Subscription / Partial / Terminal 계약

### 문제

resume/gap 원칙은 있으나 acknowledgment, partial machine stream, terminal record와 broken pipe 이후 상태가 부족하다.

### 작업

- event identity/cursor/watermark 의미
- delivered vs acknowledged/observed cursor 구분 여부
- retention window와 gap detection
- reconnect reauthorization/version negotiation
- resume 실패 시 explicit resync
- slow consumer disconnect/backpressure policy
- JSONL partial output의 terminal status 전달 방식
- 중간 오류 시 stdout에 이미 기록된 item의 의미
- last safe cursor 노출
- SIGINT/broken pipe는 local observation 종료
- target Task/Process lifecycle은 explicit Command만 변경
- server/client subscriber cleanup

### 완료 조건

- silent event loss 0
- partial JSONL을 complete success로 오판하는 경로 0
- reconnect loop 후 subscriber/task retained leak 0
- broken pipe가 Runtime Task failure를 생성하지 않음

---

## 6.10 P0-10 — Machine Output, Wait, Timeout, Error 계약

### 문제

JSON/JSONL과 exit class는 선언됐으나 accepted/pending/committed, local timeout, partial success의 automation 의미가 닫히지 않았다.

### 작업

`DXB-IFC-041`에 전역 option과 결과 의미를 고정한다.

- `--json`
- `--jsonl`
- `--wait`
- `--follow`
- `--timeout`
- `--request-id`
- `--non-interactive`
- `--yes`
- `--output <file>` 적용 범위

규칙:

- stdout=result, stderr=diagnostic/progress
- machine stdout contamination 0
- host lifecycle error와 Application error 구분
- local wait timeout은 Runtime mutation rollback을 의미하지 않음
- accepted/pending/committed 각각의 exit/result 의미
- partial output의 exit class와 structured terminal metadata
- approval-required와 local confirmation 분리
- human string parsing을 automation contract로 사용하지 않음
- exact numeric exit registry는 하나의 source에서 관리
- unknown/internal/recovery-required를 transient retry로 자동 추측하지 않음

### 완료 조건

- script가 operation state와 local wait state를 구분 가능
- timeout 뒤 duplicate retry 0
- JSON/JSONL schema와 exit class의 compatibility fixture 존재

---

## 6.11 P0-11 — Local CLI Threat Model

### 문제

secret redaction은 강하지만 local endpoint, terminal escape, file output 공격면이 빠져 있다.

### 작업

`DXB-RUN-032`, `DXB-IFC-041`, `DXB-ENG-052`에 다음을 추가한다.

#### Endpoint / Bootstrap

- user-owned endpoint directory
- socket/named pipe owner·permission
- peer credential 또는 동등한 principal binding
- world-writable endpoint 금지
- stale endpoint/symlink replacement 방지
- Host Lifecycle Adapter의 shell string composition 금지 또는 strict argument API
- endpoint/profile 변경으로 privilege escalation 금지

#### Terminal

- untrusted Model/Tool/Message/Artifact text의 ANSI/OSC control sequence 처리
- machine output에서 raw terminal control 의미 제거
- clickable link/clipboard escape의 기본 비활성 또는 sanitization
- `--raw`가 필요하면 명시적·비기본·위험 표시

#### File / Export

- path traversal 거부
- symlink overwrite와 device/special file 처리
- 기존 파일 overwrite 정책
- temporary file + atomic rename 가능한 범위
- partial failure cleanup
- sensitive output file permission
- secret/raw sensitive data의 default redaction

### 완료 조건

- local endpoint hijack fixture 실패
- ANSI/OSC terminal injection fixture 무해화
- symlink/path traversal export 거부
- partial sensitive file 잔존 0 또는 명시적 안전 정책

---

## 6.12 P0-12 — Public Schema SSOT와 실제 CI

### 문제

Contract schema의 생성 원본이 없고 현재 workflow는 과거 v0.1 문서만 고정 hash로 검증한다.

### 권장 기본 방향

P0에서는 별도 범용 IDL 플랫폼을 만들지 않는다.

- Rust public contract module/crate를 schema source로 사용
- Domain/Persistence type과 분리
- deterministic serialization/schema snapshot 생성
- committed golden fixture 또는 generated artifact drift 검사
- 두 번째 언어/remote transport가 실제로 필요해질 때 IDL 승격 검토

정확한 crate 수는 `DXB-ARC-011`의 “logical boundary first” 원칙을 따른다.

### 실제 CI 작업

현재 one-shot v0.1 workflow를 active gate로 오인하지 않도록 정리하고 다음을 실행한다.

- current active plan package 자동 탐색
- Markdown inventory/count
- `document_id` 중복
- `depends_on` 존재성/cycle
- active TUI/Web reference
- Accepted/P0 status rule
- Requirement/Acceptance/Risk/OQ orphan
- manifest resolution drift
- schema snapshot drift
- CLI forbidden dependency
- Host Lifecycle Client forbidden dependency
- public DTO = Domain/Persistence type 공유
- P0 Command Matrix completeness
- generated help/schema/exit registry drift
- relevant Rust build/test가 생긴 뒤 Contract/CLI E2E 추가

### 완료 조건

- 문서 PASS가 self-written manifest assertion에만 의존하지 않음
- PR에서 gate failure를 재현 가능
- stale v0.1-only workflow가 현재 release evidence로 남지 않음

---

## 7. 문서별 변경 지도

## 7.1 `readme.md`

- v0.8 theme를 `Resolved Implementation Baseline + CLI Contract Closure`로 변경
- 이전 버전 normative inheritance 제거
- active owner map과 P0 command scope 요약
- integrated milestone DAG
- 완료 정의 갱신

## 7.2 `manifest.md`

- v0.7→v0.8 resolution matrix
- active/self-contained inventory
- P0/P1/P2 document status
- Command Matrix completeness evidence
- actual CI evidence link/결과
- 3회 전체 Review evidence
- 발견·수정·재실행 기록

## 7.3 Governance

### `DXB-BASE-000`

- self-contained current baseline
- previous packages historical reference
- Operation Receipt, selector, Runtime instance, pagination, local security 원칙 추가
- overengineering 금지 원칙 추가

### `DXB-GOV-001`

- Effective Baseline resolution 절차
- P0 Accepted Gate
- 3회 전체 Review를 v0.8 package-specific 추가 요구로 정의
- generated evidence와 Canonical document 구분

### `DXB-GOV-002`

신규 용어:

- Operation Receipt
- Request Digest
- Resource Selector / Resolved ResourceRef
- Runtime Instance
- Query Snapshot / Page Cursor
- Partial Machine Stream / Terminal Status
- Contract Schema Source

### `DXB-GOV-003`

ADR 후보 추가:

- Runtime instance/platform/endpoint
- receipt lifecycle/key binding
- selector resolution
- pagination snapshot/cursor
- stream terminal/partial
- machine wait/exit
- local endpoint/terminal/export security
- schema SSOT/compatibility

### `DXB-GOV-004`

표준 템플릿에 다음 항목 추가:

- Resource selector
- operation receipt/idempotency
- page/stream consistency
- machine/partial result
- local attack surface
- P0/P1/P2 scope
- real CI evidence

## 7.4 Architecture

### `DXB-ARC-010`

- full implementation dependency path
- Runtime Instance boundary
- Vertical Slice architecture
- Operation Receipt와 Event/Projection 연결

### `DXB-ARC-011`

- contract schema source logical module
- CLI local journal의 허용 최소 surface
- no crate explosion rule 재강조
- Runtime instance/endpoint adapter boundary

### `DXB-ARC-012`

- Application Contract와 Capability Contract 구분 유지
- public schema source가 Provider schema로 오염되지 않게 함

### `DXB-ARC-014`

- Operation Receipt state/commit/retry
- cursor/snapshot event relation
- partial stream terminal 의미

### `DXB-ARC-015`

- receipt는 Application/Command owner journal에 저장
- CLI local journal은 non-canonical bounded reference
- Runtime instance metadata와 Domain identity 분리
- cursor/subscriber state persistence 경계

## 7.5 Domain

### `DXB-DOM-026`

- 각 Control use-case의 typed operation owner
- message/task/process control alias 중복 제거
- selector와 effective authority 반환 의미

### `DXB-DOM-027`

- Conversation/Thread selector와 parent scope
- `conversation send`/`thread send`가 하나의 canonical send use-case를 재사용하도록 명시

다른 Domain 문서는 의미를 바꾸지 않고 v0.8 self-contained baseline 및 새 public reference와의 관계만 검증한다.

## 7.6 Runtime

### `DXB-RUN-030`

- concurrent Runtime start/stop
- duplicate key + different digest
- cursor snapshot과 concurrent mutation
- output file partial/rename race

### `DXB-RUN-031`

- page/snapshot/receipt/client journal byte accounting
- resync burst와 export temporary storage quota

### `DXB-RUN-032`

- local endpoint/peer/bootstrap/terminal/file threat model
- selector가 authority를 생성하지 않음

### `DXB-RUN-033`

- stale endpoint/lock recovery
- receipt expiry/reconciliation
- partial export cleanup
- cursor snapshot recovery

### `DXB-RUN-034`

- operation state, selector resolution, instance identity, cursor/partial metrics
- high-cardinality/secret 제한 유지

### `DXB-RUN-035`

- P0 reference platform tier
- Runtime instance/data/config/runtime directory
- endpoint/lock/readiness/profile precedence

`DXB-RUN-036~038`은 explicit operation/receipt/cursor와의 관계를 재검증하되 owner 의미를 복제하지 않는다.

## 7.7 Interface

### `DXB-IFC-040`

가장 큰 변경 대상이다.

- Operation Receipt
- Command lifecycle와 key binding
- Resource selector
- structured error
- snapshot pagination
- subscription partial/terminal/resume
- schema source/compatibility
- Runtime instance/endpoint discovery safe summary

### `DXB-IFC-041`

- P0/P1/P2 Command Matrix
- global options
- wait/timeout/follow/request-id
- input source 규칙
- target resolution
- machine partial/error
- file export
- terminal sanitization
- operation show/reconcile

## 7.8 Engineering

### `DXB-ENG-050`

- schema source와 generated drift
- local journal bounds
- terminal/file safe writer
- no generic RPC escape hatch

### `DXB-ENG-051`

- snapshot pagination
- operation receipt retention
- local journal
- partial export
- terminal sanitization overhead
- concurrent start/endpoint recovery benchmark

### `DXB-ENG-052`

- 신규 Acceptance fixture
- three-pass review evidence
- Runtime instance/concurrent start
- key-digest mismatch
- selector ambiguity
- page consistency
- partial JSONL
- endpoint/terminal/file security

### `DXB-ENG-053`

- actual compatibility support window
- schema snapshot policy
- command/receipt compatibility
- cursor compatibility

### `DXB-ENG-054`

- real v0.8-aware CI
- P0 command matrix gate
- stale v0.1 workflow 정리
- build/package evidence와 docs assertion 분리

## 7.9 Delivery

### `DXB-DEL-060`

- integrated Backend→CLI milestone DAG
- Vertical Slice 중심 순서
- P0/P1/P2 scope

### `DXB-DEL-061`

아래 신규 Acceptance를 정식 매핑한다.

### `DXB-DEL-062`

아래 신규 Risk를 추가하고 phase blocker를 연결한다.

### `DXB-DEL-063`

아래 Open Question을 milestone 이전에 닫는다.

---

## 8. 제안 신규 Acceptance

ID는 v0.8 작성 시 기존 전체 inventory와 충돌 여부를 검사한 뒤 확정한다.

| 제안 ID | 의미 |
|---|---|
| AT-BASE-001 | Self-contained Effective Baseline |
| AT-CLI-008 | P0 Command Matrix Completeness |
| AT-APP-005 | Operation Receipt / Idempotency Key Binding |
| AT-CLI-009 | Resource Selector Ambiguity Safety |
| AT-HOST-001 | Runtime Instance / Concurrent Start |
| AT-APP-006 | Pagination Snapshot / Ordering / Cursor Binding |
| AT-APP-007 | Subscription Partial / Terminal / Resume |
| AT-CLI-010 | Machine Wait / Timeout / Partial Semantics |
| AT-SEC-005 | Local Endpoint / Terminal / Export Safety |
| AT-SCHEMA-001 | Public Contract Schema SSOT / Drift |
| AT-CI-001 | Actual Plan/Contract/Architecture Gate Execution |

각 Acceptance는 deterministic fixture와 명확한 pass/fail 조건을 가져야 한다. “문서에 적혀 있음”을 실행 증거로 취급하지 않는다.

---

## 9. 제안 신규 Risk

| 제안 ID | 위험 | 영향 |
|---|---|---|
| R-090 | 숨은 inheritance로 구현자별 계약 분기 | Critical |
| R-091 | P0 command 범위 불명확으로 scope 폭증 | High |
| R-092 | uncertain mutation receipt를 복구할 수 없음 | Critical |
| R-093 | ambiguous selector가 잘못된 resource를 변경 | Critical |
| R-094 | duplicate Runtime instance/endpoint/data split-brain | Critical |
| R-095 | pagination 중 누락·중복을 complete snapshot으로 오판 | High |
| R-096 | partial JSONL/timeout을 success로 오판 | Critical |
| R-097 | local endpoint/terminal/export 공격 | Critical |
| R-098 | client/server public schema drift | Critical |
| R-099 | 선언된 Gate가 실제 CI에서 실행되지 않음 | Critical |
| R-100 | Contract-all-at-once가 사용되지 않는 추상화 폭증 | High |

---

## 10. v0.8에서 닫아야 할 Open Question

| 제안 ID | 질문 | Gate |
|---|---|---|
| OQ-080 | P0 reference platform과 Runtime instance granularity | M2 |
| OQ-081 | Runtime endpoint/data/config/runtime directory와 lock | M2 |
| OQ-082 | Operation Receipt retention/key binding/local journal | M1 |
| OQ-083 | Resource selector와 implicit context 허용 범위 | M1/M3 |
| OQ-084 | pagination snapshot/live model과 cursor binding | M1 |
| OQ-085 | JSONL partial/terminal/resume 형식 | M1/M5 |
| OQ-086 | accepted/pending/committed 및 wait/timeout exit 의미 | M1/M3 |
| OQ-087 | public schema source와 compatibility support window | M1 |
| OQ-088 | terminal escape와 file export P0 안전 정책 | M2/M5 |
| OQ-089 | P0 command matrix 최종 범위 | M0 |

Open Question이 남았다는 이유로 unsafe default나 generic fallback을 구현하지 않는다. 해당 Gate를 차단한다.

---

## 11. v0.8 문서 고도화 실행 순서

## M0 — Effective Baseline / Scope Freeze

작업:

- v0.7 active 문서 전체 inventory
- previous inheritance resolution
- P0/P1/P2 command 초안
- P0 document freeze set
- actual docs CI 최소 구현 계획
- TUI/Web active reference 재검증

Exit:

- AT-BASE-001 문서 조건 충족
- P0 Command Matrix 초안 100%
- unresolved inheritance 0
- active TUI/Web artifact 0

## M1 — Contract Kernel Closure

작업:

- Operation Receipt
- key/digest binding
- structured error
- Resource selector
- pagination snapshot/cursor
- subscription partial/terminal
- machine wait/timeout
- schema SSOT

Exit:

- AT-APP-005~007
- AT-CLI-008/009/010 문서 계약
- OQ-082~087 종료
- generic mutation fallback 0

## M2 — Runtime Instance / Local Security

작업:

- reference platform tier
- Runtime instance/directories/endpoint/lock/readiness
- concurrent start/stop/restart
- peer authentication/permission
- terminal/file threat model

Exit:

- AT-HOST-001
- AT-SEC-005
- OQ-080/081/088 종료
- duplicate Runtime instance 0

## M3 — Bot-only Vertical Slice Plan

문서상 완결할 흐름:

```text
install/config
→ runtime start
→ bot create
→ conversation send
→ task submit
→ task watch
→ operation receipt 확인
→ memory inspect
→ Runtime restart
→ same identity/result 복구
```

Exit:

- 각 단계가 Command Matrix와 Contract schema에 연결
- response-loss/SIGINT/stale selector fault 삽입 가능
- Backend shortcut 0

## M4 — Project / Channel Vertical Slice Plan

```text
project create
→ channel create
→ membership/authority
→ channel send
→ task/delegation/process
→ scoped memory promotion
→ result/evidence
```

Exit:

- Role/Authority/Authorization 분리
- ambiguous participant/resource selector 0
- Private→Shared flow 비회귀
- bounded collaboration/resource 비회귀

## M5 — Recovery / Streaming / Automation Plan

작업:

- page/`--all`
- JSONL
- cursor resume/resync
- partial output
- file export
- operation reconcile
- slow/broken pipe
- Runtime restart/endpoint recreation

Exit:

- AT-APP-006/007
- AT-CLI-010
- large-output/partial semantics 완결
- resource/subscriber leak contract 0

## M6 — Freeze / CI / Release Planning

작업:

- P0 documents Accepted
- compatibility matrix 확정
- actual CI Gate
- packaging/help/completion의 P0/P1 범위
- full Backend regression map
- 3회 전체 Review

Exit:

- AT-SCHEMA-001/AT-CI-001
- Critical Risk evidence 연결
- v0.8 implementation-ready baseline 승인

---

## 12. 3회 전체 반복 검수

v0.8 문서 작성 완료 후 표본 검사가 아니라 **전체 active package를 세 번 독립적으로 반복 검수**한다. 각 검수에서 발견한 문제를 수정한 뒤 해당 검수를 처음부터 다시 수행한다.

## Review 1 — Effective Baseline / Structural / Traceability

검사:

- 모든 active file/path/ID/inventory
- self-contained effective semantic
- previous version hidden inheritance
- `depends_on` 존재성/cycle
- Canonical Owner 중복
- P0/P1/P2 scope
- Accepted status
- Command Matrix completeness
- Requirement→ADR→Owner→Acceptance→Risk→OQ
- TUI/Web active residue
- manifest resolution matrix

실패 예:

- 현재 요구를 알기 위해 v0.6 문장을 직접 찾아야 함
- matrix에 없는 P0 command
- 같은 operation의 여러 canonical CLI path
- 문서 상태와 Release Gate 불일치

## Review 2 — Cross-Layer Executability / Fault / Compatibility

전체 추적:

```text
Shell
→ CLI parser/selector
→ Host Lifecycle or Control Client
→ Application Contract
→ Application
→ Domain/Persistence
→ Runtime/Scheduler/Provider Host
→ Result/Event/Receipt
→ Pagination/Subscription
→ Machine/Human Output
```

삽입 fault:

- Runtime absent
- concurrent start
- stale endpoint
- ambiguous name
- stale revision
- permission deny/approval required
- commit 후 response loss
- same key/different digest
- CLI crash/SIGINT
- Runtime restart
- cursor expiry/gap
- concurrent list mutation
- slow/broken pipe
- partial JSONL
- export path/symlink
- version mismatch
- memory pressure

실패 예:

- owner 없는 상태
- duplicate semantic effect
- identity drift
- silent data loss
- unbounded allocation
- secret/terminal/file safety bypass

## Review 3 — Adversarial Scope / Overengineering / Implementation Ambiguity

질문:

- 실제 P0 use-case가 없는 abstraction이 추가됐는가
- 미래 TUI/Web/remote를 이유로 generic framework를 만들었는가
- 하나의 logical module을 근거 없이 여러 crate로 분리했는가
- command alias가 Backend use-case를 복제하는가
- exact enum/transport/library를 증거 없이 과고정했는가
- 반대로 구현자가 정책을 발명해야 하는 공백이 남았는가
- Provider/Plugin/Core/Routine 관리가 P0에 불필요하게 들어왔는가
- 문서 수와 규칙 중복이 늘었는가
- Runtime Memory/Concurrency/Security correctness보다 CLI 장식 기능이 앞섰는가

통과 기준:

- P0 Vertical Slice에 직접 필요하지 않은 신규 framework 0
- 새로운 Canonical Owner는 증명된 경우만 존재
- 구현자 재량이 허용되는 부분과 금지되는 부분이 분리됨
- command surface가 최소이면서 representative Backend operation을 완결함

---

## 13. 완료 정의

v0.8 고도화 문서 패키지는 다음을 모두 만족해야 완료다.

1. v0.8 active package가 self-contained implementation baseline이다.
2. 이전 버전 hidden normative inheritance가 없다.
3. P0 command matrix가 완결되고 P1/P2가 분리된다.
4. Operation Receipt와 uncertain mutation recovery가 CLI까지 연결된다.
5. Resource selector가 deterministic하며 ambiguity를 거부한다.
6. Runtime instance/endpoint/lock/readiness/concurrent start가 닫힌다.
7. pagination snapshot/order/cursor가 닫힌다.
8. subscription partial/terminal/resume가 닫힌다.
9. machine wait/timeout/error/partial semantics가 닫힌다.
10. local endpoint/terminal/export threat model과 Acceptance가 있다.
11. public schema source와 compatibility policy가 하나다.
12. Backend→CLI integrated roadmap과 Vertical Slice가 있다.
13. 실제 CI 계획과 실행 가능한 Gate가 문서 assertion을 대체한다.
14. P0 normative documents가 승인 상태다.
15. TUI/Web active milestone/artifact/gate가 없다.
16. Backend v0.6/v0.7 identity·memory·scheduler·security·resource semantic 회귀가 없다.
17. Review 1/2/3이 각각 전체 package를 대상으로 PASS한다.
18. 각 Review 발견 사항 수정 후 동일 범위를 재실행한 evidence가 `manifest.md`에 있다.

---

## 14. 최종 산출물 지시

v0.8 상세 개발기획 패키지는 기존 naming 규칙에 따라 다음 형식으로 생성한다.

```text
docs/plan/<yyyymmdd-hhmm>-v0-8-detailed-development-plan/
```

기본 문서 inventory는 v0.7 active package를 유지한다.

- Governance: 6
- Architecture: 8
- Domains: 10
- Runtime: 9
- Interfaces: 2
- Engineering: 5
- Delivery: 5
- `readme.md`
- `manifest.md`

새 Canonical 문서를 먼저 추가하지 않는다. Command Matrix는 `DXB-IFC-041`, Effective Baseline resolution은 `readme.md`/`manifest.md`, Operation Receipt와 page/stream contract는 `DXB-IFC-040`이 소유한다.

파일·디렉터리명은 lowercase kebab-case를 사용한다. Rust source naming은 실제 구현 단계에서 snake_case를 따른다.

---

## 15. 최종 지시 요약

v0.8은 새로운 기능 버전이 아니라 **구현 모호성을 제거하는 기준선 버전**이다.

다음 순서를 지킨다.

```text
Hidden inheritance 제거
→ P0 Scope Freeze
→ Operation/Selector/Page/Stream/Machine Contract Closure
→ Runtime Instance / Local Security Closure
→ Bot-only Vertical Slice
→ Project/Channel Vertical Slice
→ Recovery/Streaming/Automation
→ Actual CI
→ 3회 전체 적대적 재검수
```

v0.8 문서가 완료된 뒤에는 “무엇을 구현할지”보다 “구현자가 어떤 정책을 임의로 발명해도 되는지”가 남아 있어서는 안 된다. 동시에 미래 Interface와 ecosystem을 이유로 현재 사용하지 않는 추상화를 추가해서도 안 된다.

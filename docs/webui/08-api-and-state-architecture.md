---
title: "Web UI API 접근 경로와 상태 아키텍처"
document_id: "DXB-WEB-008"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-001", "DXB-IFC-040"]
---
# Web UI API 접근 경로와 상태 아키텍처

## 목적

Frontend가 DXBOT Runtime의 상태를 **어떻게 읽고 어떻게 바꾸는지**의 구조를 소유한다. 계약 자체는 소유하지 않는다. 계약의 Canonical Owner는 활성 plan의 [`DXB-IFC-040`](../plan/20260823-1310-v0-8-10-detailed-development-plan/40-interfaces/40-api-protocols.md)이다.

## 1. 선행 조건 — 브라우저가 접근할 수 있는 전송 경로가 아직 없다

현재 저장소의 control plane 전송은 **Unix domain socket local transport**다. 근거: `crates/control-server/src/local_transport.rs`가 `std::os::unix::net::UnixListener`로 endpoint를 bind한다. HTTP 서버, SSE/WebSocket endpoint, 브라우저 인증 경로는 존재하지 않는다.

따라서 다음이 사실이다.

1. 브라우저는 현재 DXBOT Runtime에 직접 연결할 수 없다. Web UI는 전송 경로가 생기기 전까지 **mock 데이터로만 동작**한다.
2. Web UI가 실제 데이터를 붙이려면 Rust 측에 **HTTP + streaming projection gateway**가 필요하다. 이는 Frontend 작업이 아니라 Rust Interface 작업이며 [01-scope-and-ownership.md](01-scope-and-ownership.md) §4 기준 **Tier A**다.
3. 이 gateway의 설계·소유는 활성 `docs/plan/` 패키지의 Interface Owner가 가져야 한다. 이 문서 패키지가 gateway 계약을 자체적으로 확정하지 않는다. 추적: [17-open-decisions.md](17-open-decisions.md) `WEB-OQ-005`.

### 1.1 gateway에 요구되는 성질

```text
Runtime (canonical state)
   ↓ local control transport (기존)
web gateway (신규, Rust)
   ↓ HTTP  : bounded query / command submission
   ↓ stream: cursor 기반 subscription (SSE 또는 WebSocket)
browser
```

- gateway는 **projection과 전송만** 담당한다. 상태·정책의 두 번째 Owner가 되지 않는다. 자체 판단으로 Domain 상태를 합성·보정하지 않는다.
- Command 경로는 `DXB-IFC-040`의 Receipt 의미(`Accepted | Committed | Rejected | Superseded | RecoveryRequired`)와 idempotency key 규칙을 **그대로 전달**한다. Web 전용 성공/실패 의미를 새로 만들지 않는다.
- Subscription은 cursor 관측이다. 재연결 시 cursor로 재개하고, gap이 생기면 gap을 숨기지 않고 재동기화 신호를 올린다.
- 오류는 `DXB-IFC-040`의 stable error 구조(code/category/retryable/refs/violations/next_actions)를 유지한다. HTTP status로 의미를 뭉개지 않는다.

### 1.2 보안 요구 (필수)

gateway는 **네트워크에 노출되는 새 표면**이다. 인증·인가 없이 만들면 로컬 Runtime 전체 제어 권한이 브라우저로 열린다.

- 인증되지 않은 요청을 허용하는 gateway를 만들지 않는다.
- 기본 bind 주소는 loopback으로 제한하고, 외부 노출은 명시적 설정으로만 허용한다.
- Principal 해석과 권한 판정은 Runtime이 수행한다. gateway가 권한을 자체 판단하지 않는다.
- Command 제출은 `DXB-IFC-040`의 principal-bound idempotency key 규칙을 따른다.
- 승인 필요(`approval-required`) 상태를 UI에서 성공으로 표시하지 않는다.

이 조건이 충족되기 전까지 Web UI를 실제 Runtime에 연결하지 않는다.

### 1.3 plan 범위 결정이 먼저다

`DXB-BASE-000` §3은 **Web과 BFF를 Active P0 비범위**로 선언한다. 여기서 요구하는 gateway는 정확히 그 BFF/Web transport에 해당한다. 따라서 순서는 다음과 같다.

```text
plan 범위 결정 (Web/BFF를 언제, 어떤 형태로 범위에 넣을지)
→ Interface Owner 문서에서 gateway 계약 정의
→ gateway 구현 + 인증
→ Web UI 연결 (Phase 7)
```

Frontend 편의를 위해 임시 HTTP 서버를 만들어 이 순서를 우회하지 않는다. 우회하면 P0 baseline이 배제한 표면이 계약 없이 생긴다.

### 1.4 현재 executable contract coverage와 연결 blocker

현재 `crates/application-contract/golden/contract-snapshot-v1.json`에는 63개 operation이 있고 built CLI가 이를 사용한다. 그러나 이것이 Cockpit의 모든 시각 영역에 안정된 read projection이 있다는 뜻은 아니다.

| Cockpit 요구 | 현재 stable operation | 현재 판정 |
|---|---|---|
| Runtime 상태·진단 | `runtime-status`, `runtime-doctor` | 구현됨. 현재 전송은 동기 Unix socket request/response |
| Bot / Project 선택과 상세 | `bot-list/show`, `project-list/show` | 구현됨 |
| Conversation / Thread | `conversation-show/history/send`, `thread-list/show/history/send` | 구현됨 |
| Task 실행·결과·관측 | `task-list/show/result/watch`와 typed control | 구현됨. watch는 cursor/gap/resync 계약 사용 |
| Provider 상태 | `provider-list/show` | 구현됨. exact id/generation이며 fallback 없음 |
| Memory / Approval | `memory-get/search/history`, `approval-list/show/approve/deny` | 구현됨 |
| 단일 Durable Process | `process-show/watch` | 알려진 ProcessRef 관측만 가능 |
| Flow catalog / definition | 없음 | **contract gap.** `/flows`와 Flow 선택 UI를 live data에 결박할 수 없음 |
| Run Queue(Process 목록) | process list operation 없음 | **contract gap.** `task-list`를 Run 목록으로 재해석하면 Domain 의미 위반 |
| Recent Events / audit timeline | public event/audit query 없음 | **contract gap.** `runtime-doctor`를 event timeline으로 사용하지 않음 |
| Active Agents | 안정된 Domain 개념·query 없음 | `WEB-OQ-003` blocker |
| CPU/Memory/Disk/Network 카드 | 시안과 동일한 안정 metric DTO 없음 | `WEB-OQ-012` blocker |
| Sessions / Branch / Memory health % | 안정된 의미·query 없음 | `WEB-OQ-011`, `WEB-OQ-004`, `WEB-OQ-013` blocker |

따라서 **gateway 구현만으로 전체 live Cockpit이 완성되지 않는다.** 기존 operation으로 읽을 수 있는 영역만 연결하고, 누락된 projection은 Frontend 추론이나 CLI output scraping으로 메우지 않는다. 필요한 query/subscription은 Interface Owner의 Tier A contract 변경으로 먼저 정의해야 한다(`WEB-OQ-016`).

활성 로드맵의 freeze 규칙도 별도로 유지한다.

| Web UI 요구 | 의존 milestone | 의미 |
|---|---|---|
| Task / Process watch | `M5` | subscription subset의 Canonical freeze 지점 |
| Operation / Provider 등 개별 read | command별 milestone | `operation-show`는 M3, `provider-list/show`는 M5처럼 registry의 freeze owner를 각각 따름 |
| 전체 63 operation schema freeze | `M6` | M6 이전 생성 타입 전체 안정성을 가정하지 않음 |

현재 코드에 operation이 존재하는 사실과 plan의 freeze 완료 판정은 같은 의미가 아니다. 실제 Web 연결은 gateway, wire schema 생성, 필요한 read contract, 해당 milestone Gate가 모두 충족된 영역만 단계적으로 허용한다([09-type-contract.md](09-type-contract.md) §2).

### 1.5 correlation과 관측 안전

`DXB-RUN-034` §1의 correlation 체인은 `ClientRequestId → CommandId → OperationId`로 시작한다. Web UI는 요청마다 `ClientRequestId`를 생성해 전달하고, 화면의 오류 표시에 이를 노출해 사용자가 서버 기록과 연결할 수 있게 한다.

동시에 `DXB-RUN-034`는 raw Message/Memory/secret payload를 기본 trace에 남기지 않고, metric label에 ResourceId·raw text·path·token·cursor를 넣지 않는다고 규정한다. Web UI도 같은 규칙을 따른다.

- 메시지 본문·Memory assertion·토큰·경로를 클라이언트 로그나 외부 telemetry로 보내지 않는다.
- 오류 화면에 secret이나 내부 policy 값을 표시하지 않는다.
- 진단 정보는 `ClientRequestId`, `OperationId`, error `code/category` 같은 안전한 참조로 제한한다.

## 2. 상태 분류

### 2.1 Server state — 복제하지 않는다

다음은 query cache에 두고, Frontend store에 정규화해 영구 보관하지 않는다.

```text
Bot 목록/상태          Runtime/Core 상태       Flow 정의
Run/Execution 상태     Task 상태               Message history
Provider/Agent health  Runtime event stream    Resource/Health metric
Memory 상태            권한/승인 상태
```

### 2.2 UI state — Frontend가 소유한다

```text
rail 확장/축소         선택된 tab (All Runs / My Runs / Watchlist)
모달/시트 열림          composer draft 입력
로컬 필터/검색어        theme preference
그래프 확대 상태        스크롤 위치
```

### 2.3 도구 경계

- Server state: bounded query/cache abstraction을 사용한다. 구체 library는 `WEB-OQ-007` stack 결정 이후 선택한다.
- UI-only 공유 상태: 실제로 여러 화면이 공유할 때만 경량 store를 도입한다.
- 컴포넌트 수명 상태: 컴포넌트에 둔다.
- 라이브러리는 책임 경계를 구현하는 수단이며, 경계를 대신 결정하지 않는다.

## 3. Real-time 반영

```text
초기 query (bounded)
     +
subscription event (cursor)
     ↓
정규화 캐시 갱신 (idempotent)
     ↓
selector 기반 부분 재렌더
```

규칙:

1. 이벤트는 **중복 전달을 정상**으로 가정한다. event id 또는 sequence로 idempotent 적용한다.
2. 순서가 어긋난 이벤트를 무시하지 않고 cursor 기준으로 판정한다.
3. cursor gap·재시작이 발생하면 해당 뷰를 재동기화 상태로 표시한다([10-error-and-exceptional-state.md](10-error-and-exceptional-state.md)).
4. 재연결은 backoff를 적용하고, 재연결 중임을 UI에 표시한다.
5. 이벤트 하나가 화면 전체 재렌더를 유발하지 않도록 캐시 갱신 단위를 항목 수준으로 유지한다.
6. 무한히 누적되는 이벤트 버퍼를 만들지 않는다. 보관 한도는 [12-performance-budget.md](12-performance-budget.md)가 소유한다.
7. **구독 종료는 실행 종료가 아니다.** 화면 이탈·구독 해제가 Process/Task를 취소하는 것처럼 표현하지 않는다(`DXB-RUN-038` §6, `DXB-DOM-027`, [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) §5).

## 4. Mutation

1. Command 결과는 Receipt로 판정한다. HTTP 200을 성공으로 간주하지 않는다.
2. Receipt 상태(`Accepted | Committed | Rejected | Superseded | RecoveryRequired`)와 Domain outcome(Task `Rejected/Deferred` 등)은 **다른 축**이다. Receipt `Committed` + Task outcome `Rejected/Deferred`를 operation 실패로 표시하지 않는다(`DXB-DOM-023`, `DXB-DOM-025`).
3. 낙관적 UI는 **Runtime이 거부할 수 없는 변경**에만 적용한다. 실행 지시·승인 필요 동작에는 적용하지 않는다.
4. conflict(stale revision/generation)는 자동 재시도하지 않고 사용자에게 현재 상태를 보여준다.
5. Approval이 필요한 operation은 `Accepted + ApprovalRef` 상태를 유지한다. UI는 ActionGrant나 resume token을 직접 만들지 않고, approval decision 후 **원 operation의 재평가 결과**를 관측한다(`DXB-DOM-026`).
6. `next_actions`가 있으면 그것을 UI 액션으로 노출한다. Frontend가 임의의 후속 동작을 발명하지 않는다.
7. idempotency key는 principal scope에 결박된다. 재시도 시 새 key를 임의 생성해 중복 operation을 만들지 않는다(`DXB-IFC-040`).

## 5. 개발 단계의 mock 경계

gateway가 없는 동안 사용하는 mock은 다음 조건을 지킨다.

- 현재 wire generator가 없으므로 visual mock의 임시 shape는 `mocks/` 안에만 둔다. 이를 generated wire type이나 production API contract로 부르지 않는다. `WEB-OQ-006`이 닫히면 canonical wire artifact에서 생성한 타입으로 교체한다.
- mock 데이터는 시안의 값을 그대로 사용해 시각 검증이 가능하게 한다.
- mock 계층은 단일 위치에 격리하고 화면 코드에 흩뿌리지 않는다.
- 실제 연결 단계에서 mock을 삭제한다. 병행 유지하지 않는다.

## 6. 검증

- Domain 상태가 Frontend store에 재정규화되어 영구 보관되지 않는지 확인
- 동일 이벤트 2회 전달 시 화면 상태가 동일한지 테스트
- 재연결·cursor gap 시나리오 테스트
- 인증 없는 gateway 접근이 거부되는지 확인 (gateway 구현 시)

## 관련 문서

- [01-scope-and-ownership.md](01-scope-and-ownership.md) — 소유권 경계
- [09-type-contract.md](09-type-contract.md) — 타입 생성
- [10-error-and-exceptional-state.md](10-error-and-exceptional-state.md) — 실패 표현
- [17-open-decisions.md](17-open-decisions.md) — gateway 미결정 사항

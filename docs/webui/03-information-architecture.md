---
title: "Web UI Information Architecture와 용어 매핑"
document_id: "DXB-WEB-003"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-002"]
---
# Web UI Information Architecture와 용어 매핑

## 목적

내비게이션 구조, route 이름, UI 라벨과 Domain 용어의 매핑을 소유한다.

## 1. 내비게이션은 시안에서 도출한다

route는 개발 편의로 만들지 않고 시안의 내비게이션 요소에서 도출한다. 근거는 [02-screen-inventory.md](02-screen-inventory.md) §2.1(Desktop rail)과 §3(Mobile tab bar)이다.

Desktop rail 항목: `Workspace`, `Bots`, `Flows`, `Tasks`, `Memory`, `Settings`
Mobile tab bar 항목: `Home`, `Runs`, `(+)`, `Agents`, `Menu`

두 목록은 다르다. 이것은 모순이 아니라 **폼팩터별 우선순위 선택**이다. Mobile은 5개 슬롯만 있으므로 자주 쓰는 항목을 노출하고 나머지는 `Menu`로 접는다.

## 2. Proposed route namespace와 현재 지원

아래 경로는 시안의 navigation label에서 도출한 **후보 namespace**다. `workspace-cockpit` 외 화면은 시안이 없고, route 또는 backend read contract가 모두 승인된 상태가 아니므로 implementation commitment로 해석하지 않는다.

| 후보 route | 시안 근거 | 현재 stable contract | 판정 |
|---|---|---|---|
| `/` | Workspace Cockpit | 여러 existing query를 조합할 수 있으나 전체 Cockpit projection은 없음 | visual screen 확정, live composition은 `WEB-OQ-016` blocker |
| `/bots`, `/bots/[bot-id]` | rail `Bots` | `bot-list`, `bot-show` | data contract 있음, 상세 시안 없음 |
| `/flows`, `/flows/[flow-id]` | rail `Flows` | Flow catalog/definition query 없음 | reserved candidate, live 구현 금지 |
| `/runs` | Run Queue `View all` | Process list query 없음 | reserved candidate, `task-list`로 대체 금지 |
| `/runs/[run-id]` | Run badge/Workflow expand | `process-show/watch`는 known ProcessRef에 사용 가능 | route/visual 미확정 |
| `/tasks`, `/tasks/[task-id]` | rail `Tasks` | `task-list`, `task-show/result/watch` | data contract 있음, 시안 없음 |
| `/memory` | rail `Memory` | `memory-get/search/history` | data contract 있음, 시안 없음 |
| `/agents` | Mobile Agents/View all | Agent 개념·query 없음 | `WEB-OQ-003` blocker |
| `/events` | Recent Events `View all` | public audit/event timeline query 없음 | `WEB-OQ-016` blocker |
| `/settings` | rail `Settings` | Web settings contract 없음; theme은 local UI preference | reserved candidate |

규칙:

1. `/`는 Cockpit 후보 route이며 `/dashboard` 별칭을 만들지 않는다.
2. 시안 없는 route는 [02-screen-inventory.md](02-screen-inventory.md) §4와 `WEB-OQ-008` 결정 전 시각 규범으로 승인되지 않는다.
3. `View all`은 실제 목적 route와 필요한 query가 모두 있을 때만 production interaction으로 활성화한다. Visual regression fixture에서는 PNG 재현용 non-production control로 렌더할 수 있지만 이를 live navigation 완료로 계산하지 않는다.
4. Mobile `Menu`의 항목은 Flows / Tasks / Memory / Settings 후보를 포함하되, Desktop Favorites parity는 `WEB-OQ-017` 결정 전 추가하지 않는다.
5. Mobile FAB는 route가 아니라 생성 action 후보이며 command 집합은 `WEB-OQ-002`가 소유한다.
6. 승인된 상세 route는 deep link와 새로고침 복구를 제공해야 한다. 후보 route 전체에 이 완료 조건을 선적용하지 않는다.

## 3. Mobile tab ↔ route 매핑

| tab | route | 비고 |
|---|---|---|
| Home | `/` | Cockpit mobile layout |
| Runs | `/runs` | Run Queue 전체 |
| (+) | 없음 | 생성 액션 시트 |
| Agents | `/agents` | Active Agents 전체 |
| Menu | 시트 | Flows / Tasks / Memory / Settings 진입 |

## 4. 표현 어휘 ↔ Domain 용어 매핑

시안의 UI 라벨과 DXBOT Domain 용어는 **1:1이 아니다.** UI 라벨은 표현이고, 의미의 Canonical Owner는 활성 plan의 Domain 문서다. 이 표가 유일한 매핑 지점이며 다른 문서·코드에서 다시 정의하지 않는다.

| UI 라벨 (시안) | Domain 개념 | Canonical Owner |
|---|---|---|
| Workspace | Project (협업·권한·자원 Scope) | `DXB-DOM-028` |
| Bot | Bot | `DXB-DOM-020` |
| Brain / Dynamic Core | Brain semantics / Core Lease | `DXB-DOM-021`, `DXB-DOM-024` |
| Flow | Durable Process **정의**(`DefinitionId`/`DefinitionVersion`) | `DXB-RUN-038` |
| Run | Durable Process **인스턴스**(`ProcessId`/`ProcessRevision`) | `DXB-RUN-038` |
| Step | Process step / activity 참조 | `DXB-RUN-038` |
| Parallel job | Process가 만든 child Task / Execution | `DXB-RUN-038`, `DXB-DOM-023` |
| Task (rail 항목) | Task aggregate (durable work intent) | `DXB-DOM-023` |
| Thread / Conversation | Bot Main Conversation 또는 Channel Conversation + Thread lineage | `DXB-DOM-027` |
| Agent | **미확정.** admitted Execution / Core Lease / Provider registration / typed delegation 대상 Bot 중 무엇인지 결정 전 | `WEB-OQ-003`, `DXB-DOM-025`, [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) §3 |
| Memory State | Memory assertion 영역 상태. `Healthy 92%` 지표의 근거는 미확정 | `DXB-DOM-022`, `WEB-OQ-013` |
| Sessions | **미확정.** Interface Session / Provider Session / Core Lease는 서로 다른 identity(`INV-005`) | `WEB-OQ-011` |
| Recent Events | 관측·audit projection | `DXB-RUN-034` |
| Resource / Health | Runtime 자원·admission 상태. host CPU/Disk/Network 출처는 미확정 | `DXB-RUN-031`, `WEB-OQ-012` |
| Branch | 미확정 — Domain 대응 없음 | `WEB-OQ-004` |
| Environment pill (`Acme Production`) | local Runtime Instance/profile **선택 hint**. endpoint discovery 전용이며 Authority나 배포 대상 의미가 아님 | `DXB-RUN-035`, `DXB-BASE-000` §4.21 |

매핑 규칙:

1. UI 라벨을 바꿀 때 Domain 이름을 따라가지 않는다. 반대로 Domain 의미가 바뀌면 이 표를 같은 change set에서 갱신한다.
2. 매핑이 **미확정**인 항목(`Agent`, `Sessions`, `Branch`, Memory/Resource 지표)은 표현 전용으로 두고, 그 값을 근거로 동작을 분기하지 않는다.
3. Frontend 타입 이름은 Domain 이름을 따른다. 화면 라벨만 UI 어휘를 쓴다. 예: 타입은 `DurableProcess`, 라벨은 `Run`.
4. `Run`을 단일 `Execution`으로 구현하지 않는다. 시안의 Run은 다단계·병렬·입출력을 가지므로 Process 의미에 해당하고, `Execution`은 immutable attempt다(`DXB-DOM-023`).

## 5. Domain 상태 ↔ UI 상태 라벨 매핑

시안의 legend 라벨(`Completed / Running / Queued / Pending`)은 **Domain 상태 이름이 아니다.** Domain 상태기계가 원본이고 UI 라벨은 그 투영이다. 표현(아이콘·색)은 [04-design-tokens.md](04-design-tokens.md) §5가 소유하고, **의미 매핑은 이 절이 소유한다.**

Task (`DXB-DOM-023`)

```text
Submitted → Admitted → Running
Submitted/Admitted → Deferred | Rejected | Cancelled
Running → Suspended | Completed | Failed | Cancelled | RecoveryRequired
```

| Domain 상태 | UI 라벨 |
|---|---|
| `Submitted`, `Admitted` | Queued |
| `Running` | Running |
| `Suspended` | Suspended |
| `Completed` | Completed |
| `Failed` | Failed |
| `Cancelled` | Cancelled |
| `Deferred` | Deferred |
| `Rejected` | Rejected |
| `RecoveryRequired` | Recovery required |

Durable Process (`DXB-RUN-038`)

```text
Created → Running → Waiting
Running/Waiting → Suspending → Suspended → Resuming → Running
Running/Waiting/Suspended → Completing → Completed | Cancelling → Cancelled | Failed | RecoveryRequired
```

`Created`는 UI `Pending`, `Waiting`은 UI `Waiting`으로 표시한다. `Suspending / Resuming / Completing / Cancelling`은 전이 중 상태이므로 목적 상태에 진행 표시를 덧붙여 표현하고, 목적 상태에 이미 도달한 것처럼 표시하지 않는다.

Bot lifecycle (`DXB-DOM-020`)

```text
Provisioning → Inactive → Activating → Active
                        ↘ Degraded
Active/Degraded → Quiescing → Inactive
Inactive/Active/Degraded → Archived → Restoring → Inactive
```

시안 Bot State 카드의 pill `Operational`은 Domain 상태가 아니다. `Active`를 카드용으로 표현하는 라벨이며, 같은 카드의 key-value `State` 행이나 compact metric은 원문 값 `Active`를 표시할 수 있다. 두 표현은 서로 다른 상태가 아니다. `Degraded`·`Quiescing`·`Archived` 등은 각각 별도 라벨을 갖고 여러 Domain 상태를 `Operational`로 뭉개지 않는다.

Delegation (`DXB-DOM-025`)

Delegation은 독립적인 "Agent lifecycle"이 아니라 recipient Bot에 생성되는 durable Task intent다. UI는 sender/recipient Bot reference, submission Receipt, recipient Task outcome을 각 owner에서 읽으며 별도 Delegation 상태기계를 만들지 않는다. recipient Task가 생성된 뒤 `Rejected/Deferred`이면 Receipt는 `Committed`일 수 있다.

Operation Receipt (`DXB-IFC-040`)

```text
Accepted | Committed | Rejected | Superseded | RecoveryRequired
```

Receipt 상태는 Task/Process 상태와 다른 축이다. 같은 배지에 섞어 표시하지 않는다. 특히 **Receipt `Committed` + Task outcome `Rejected/Deferred`는 실패가 아니다**([10-error-and-exceptional-state.md](10-error-and-exceptional-state.md) §3).

매핑 규칙:

1. Domain이 상태를 추가하면 이 표를 먼저 갱신한다. 매핑 없는 값은 `unknown`으로 안전 렌더링한다.
2. UI 라벨을 근거로 동작을 분기하지 않는다. 분기는 Domain 상태 값으로 한다.
3. 서로 다른 상태기계(Task / Process / Bot / Receipt)의 값을 하나의 필드에 합치지 않는다(`INV-005` 취지).

## 6. 검증

- rail/tab 항목과 route 표가 일치하는지 확인
- 시안에 없는 route가 추가되지 않았는지 확인
- 코드의 라벨 문자열이 §4 표의 UI 라벨과 일치하는지 확인
- Domain 상태 열거값 중 §5 매핑이 없는 값이 있는지 확인
- 상태 분기 코드가 UI 라벨 문자열이 아니라 Domain 상태 값을 사용하는지 확인
- 모든 deep link가 새로고침 후 복구되는지 확인 ([16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) Review 2)

## 관련 문서

- [02-screen-inventory.md](02-screen-inventory.md) — 내비게이션 요소의 출처
- [04-design-tokens.md](04-design-tokens.md) — 상태 표현(아이콘·색)
- [06-responsive-layout.md](06-responsive-layout.md) — 폼팩터별 내비게이션 전환
- [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) — 불변조건 제약
- [17-open-decisions.md](17-open-decisions.md) — 미결정 매핑

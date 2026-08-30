---
title: "Web UI의 Domain 불변조건 준수 규칙"
document_id: "DXB-WEB-018"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-001", "DXB-BASE-000"]
---
# Web UI의 Domain 불변조건 준수 규칙

## 목적

활성 plan의 제품 불변조건(`DXB-BASE-000` §2 `INV-001`~`INV-015`)이 Web 화면 표현에 부과하는 제약을 소유한다. 불변조건 자체는 소유하지 않는다. 원문은 [00-normative-baseline.md](../plan/20260823-1310-v0-8-10-detailed-development-plan/00-governance/00-normative-baseline.md)가 소유한다.

시안은 시각의 기준이지만 **Domain 의미의 기준은 아니다.** 시안이 그린 요소가 불변조건과 충돌하면 시각은 유지하고 의미를 교정하거나, 교정이 불가능하면 [17-open-decisions.md](17-open-decisions.md)로 올린다. 충돌을 모른 채 구현해서 UI가 잘못된 Domain 모델을 사용자에게 가르치는 것이 최악의 결과다.

## 1. UI에 직접 제약을 주는 불변조건

| 불변조건 | 요지 | Web UI 규칙 |
|---|---|---|
| `INV-001` | Bot은 Interface Session과 독립된 지속 identity | 브라우저 세션 종료·탭 닫기가 Bot 상태에 영향을 주는 것처럼 표현하지 않는다 |
| `INV-002` | Bot 하나는 하나의 logical Brain | Brain을 여러 개 선택·교체하는 UI를 만들지 않는다. Bot State 카드의 `Brain` 행은 단일 값이다 |
| `INV-003` | Core는 Scheduler 소유 Dynamic Lease이며 Bot 복제본·독립 Agent가 아니다 | Core를 독립 실행 주체나 하위 Bot으로 보이게 표현하지 않는다 |
| `INV-004` | Bot당 Main Conversation 하나 | Bot에 여러 Main Conversation을 만들 수 있는 UI를 노출하지 않는다 |
| `INV-005` | Conversation / Thread / Task / Execution / Core Lease / Provider Session / Interface Session은 서로 다른 identity | 하나의 ID 칸에 서로 다른 identity를 섞어 표시하지 않는다. `Sessions` 표시는 어떤 session인지 확정 후 렌더한다([17-open-decisions.md](17-open-decisions.md) `WEB-OQ-011`) |
| `INV-006` | Project·Channel은 Scope이며 Brain을 소유하지 않는다 | Workspace 전환이 Bot의 Brain·Memory를 바꾸는 것처럼 표현하지 않는다 |
| `INV-007` | Conversation History와 Memory는 구분된다 | Thread 패널의 메시지를 Memory로 표시하지 않는다. rail `Memory`는 assertion 영역이다 |
| `INV-008` | running Execution의 Context Plan·Task revision·Provider binding은 immutable snapshot | 실행 중 항목의 snapshot 값을 실시간 변동값처럼 갱신 표시하지 않는다 |
| `INV-009` | Durable Process는 cross-aggregate 진행만 소유하고 child state를 복제하지 않는다 | Workflow 화면이 child Task 상태의 두 번째 원본이 되지 않는다. child 상태는 child owner 참조로 읽는다 |
| `INV-011` | Provider·Plugin·Interface는 Domain identity·canonical state를 소유하지 않는다 | Web UI가 상태를 보정·합성해 표시하지 않는다 |
| `INV-012` | Bot-only path는 Project·Channel·Durable Process 없이 완전히 동작한다 | **Cockpit은 Project 없음 / Process 없음 상태에서도 유용하게 동작해야 한다**(§2) |
| `INV-014` | Multi-Bot은 typed delegation과 membership 경계로 동작하며 anonymous sub-agent로 축약하지 않는다 | `Active Agents` 카드를 익명 sub-agent 목록으로 구현하지 않는다(§3) |
| `INV-015` | P0 병렬성은 서로 다른 admitted Execution 간 병렬성이며 한 Execution 내부 anonymous fan-out은 P0가 아니다 | Workflow의 `PARALLEL JOBS`를 한 Execution 내부 익명 병렬로 구현하지 않는다(§4) |

## 2. Bot-only 기본 상태 (INV-012)

시안은 Workspace(Project), Flows(Process), Agents가 모두 채워진 상태를 보여준다. 그러나 `INV-012`에 따라 **Project와 Durable Process가 없어도 제품은 완전해야 한다.** 따라서 Cockpit의 기본 대상은 Bot과 Main Conversation이다.

구현 규칙:

1. Project가 없을 때 Workspace 셀은 오류가 아니라 "Project 없음" 표현을 사용하고, Cockpit의 나머지 영역은 정상 동작한다.
2. Durable Process가 없을 때 Workflow Overview는 오류가 아니라 빈 상태를 표시하고, Thread·Bot State·Run 목록은 정상 동작한다.
3. Workspace 선택을 Cockpit 진입의 전제 조건으로 만들지 않는다.
4. Bot-only 상태를 "설정 미완료"로 규정하지 않는다. 그것은 정상 상태다.

이 요구는 시안에는 없지만 **비협상**이다. 시안 상태만 구현하면 P0 제품 요구를 위반한다.

## 3. `Active Agents` 카드의 충돌 (INV-003 / INV-014)

시안의 `Active Agents`는 `Orchestrator`, `Data Fetcher`, `Transformer`, `Notifier`를 개별 실행 주체처럼 나열한다. 이는 다음과 충돌할 수 있다.

- `INV-003`: Core는 독립 Agent가 아니다.
- `INV-014`: Multi-Bot 협업은 typed delegation이며 anonymous sub-agent로 축약하지 않는다.

따라서 이 카드를 그대로 "익명 agent 목록"으로 구현하면 Domain 모델을 잘못 표현한다. 결정 전 구현 규칙:

1. 카드의 시각 형태는 유지한다.
2. 각 행의 대상이 무엇인지 확정하기 전에는 실제 데이터를 결박하지 않는다. 후보는 admitted Execution, Core Lease, Provider registration, typed delegation의 recipient Bot(`DXB-DOM-025`)이다. 현재 stable `Agent` query는 없다.
3. 라벨을 임의로 "Agent"로 굳히지 않는다. `WEB-OQ-003`이 닫히기 전에는 표현 전용으로 취급한다.

## 4. `PARALLEL JOBS` 그룹의 의미 (INV-015)

시안은 한 Run 안에서 `4A`, `4B`가 동시에 도는 모습을 보여준다. `INV-015`에 따라 P0의 병렬성은 **서로 다른 admitted Execution 사이**의 병렬성이다.

따라서 병렬 그룹은 Durable Process가 만든 **child Task/Execution 참조**로 해석한다. 한 Execution 내부의 익명 fan-out으로 구현하지 않는다. 시각 표현(점선 그룹 박스)은 유지한다.

## 5. 취소·종료 의미 (수명 경계)

Domain은 관측자 종료가 작업을 취소하지 않는다고 명시한다.

- `DXB-RUN-038` §6: subscriber disconnect가 process를 cancel하지 않는다.
- `DXB-DOM-027`: Thread나 CLI exit가 linked Task를 암묵 cancel하지 않는다.

Web UI 규칙:

1. 화면 이탈·탭 닫기·구독 해제가 실행을 취소하는 것처럼 표현하지 않는다.
2. 취소는 명시적 typed operation으로만 제공한다. "닫기"와 "취소"를 같은 버튼에 두지 않는다.
3. 구독이 끊긴 상태를 "작업 중단"으로 표시하지 않는다. 관측 중단과 실행 중단을 구분한다([10-error-and-exceptional-state.md](10-error-and-exceptional-state.md)).

## 6. 검증

- Project 없음 / Process 없음 / Agent 없음 상태에서 Cockpit이 동작하는지 확인한다(fixture 필수).
- 각 identity 칸에 다른 종류의 ID가 들어가지 않는지 확인한다.
- 화면 이탈 후 실행이 계속되는지 통합 테스트로 확인한다.
- 이 문서의 규칙 위반은 [16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) Review 2 항목이다.

## 관련 문서

- [01-scope-and-ownership.md](01-scope-and-ownership.md) — 활성 P0 baseline과의 범위 관계
- [03-information-architecture.md](03-information-architecture.md) — UI 라벨 ↔ Domain 의미 매핑
- [17-open-decisions.md](17-open-decisions.md) — 미결정 충돌 항목

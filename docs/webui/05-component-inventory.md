---
title: "Web UI Component Inventory"
document_id: "DXB-WEB-005"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-002", "DXB-WEB-004"]
---
# Web UI Component Inventory

## 목적

시안 해부에서 도출된 컴포넌트 목록과 계층 규칙을 소유한다. 각 컴포넌트가 시안의 어느 요소에서 나왔는지 추적 가능해야 한다.

## 1. 추상화 순서

```text
Design Token → Primitive → Composite → Feature → Screen
```

화면을 먼저 만들고 나중에 공통화하지 않는다. Cockpit이 유일한 시안 화면이므로 **Cockpit에서 반복되는 요소가 곧 공통 컴포넌트**다.

## 2. Primitive

| 컴포넌트 | 시안 근거 |
|---|---|
| `app-text` | 모든 텍스트. typography token만 받는다 |
| `app-icon` | rail/노드/legend/카드 header 아이콘 |
| `app-button` | `View Run`, `View all runs` |
| `icon-button` | 필터, kebab, expand, 첨부/멘션/이모지, rail collapse |
| `surface` | 카드 컨테이너 |
| `divider` | header 셀 구분선, stat strip 세로 구분선, 카드 내부 구분선 |
| `badge` | `BOT`, Run Queue 개수, Active Agents 개수 |
| `pill` | `Run #8421`, `Operational`, 환경 pill |
| `avatar` | `JD`, Bot 마크 (presence dot 지원) |
| `status-dot` | Bot 상태, agent 상태, Run 상태, 환경 상태 |
| `progress-bar` | header Memory/Progress, Run Progress 카드, 테이블 mini bar |
| `sparkline` | Sessions, Resource/Health |
| `text-field` | composer 입력, Run Queue 검색 |
| `skeleton` | 로딩 표현 ([10-error-and-exceptional-state.md](10-error-and-exceptional-state.md)) |

`progress-bar`는 **크기 variant가 아니라 하나의 컴포넌트 + size token**으로 구현한다. 다만 Memory health, overall Progress, Run status의 semantic tone은 호출부가 [04-design-tokens.md](04-design-tokens.md) §5의 token role로 주입한다. 컴포넌트 내부에서 항상 파랑/초록으로 고정하지 않는다.

## 3. Layout

| 컴포넌트 | 시안 근거 |
|---|---|
| `app-shell` | 전체 골격. Desktop 4-column / Mobile 세로 스택을 결정 |
| `nav-rail` | 좌측 내비게이션(확장·축소 상태 포함) |
| `nav-section-label` | `FAVORITES` |
| `environment-switcher` | rail 하단 환경 pill |
| `global-header` | 상단 셀 행 |
| `header-metric-cell` | header의 라벨+값 셀 (Mobile stat strip 셀과 동일 컴포넌트) |
| `mobile-app-header` | Mobile 상단 로고 + avatar + kebab |
| `mobile-tab-bar` | 하단 tab bar + 중앙 FAB |
| `card` | header(제목 + 우측 액션) + body + optional footer |
| `card-footer-link` | `View all runs`, `View all agents` |
| `column-stack` | 우측 상태 카드 스택 |

`header-metric-cell`은 Desktop header와 Mobile stat strip의 공통 **label/value layout shell**이다. text, status dot, link, percent, progress bar, sparkline을 bounded slot으로 받을 수 있지만 metric 의미는 결정하지 않는다. Dark/Light Mobile의 metric과 표현 차이는 [17-open-decisions.md](17-open-decisions.md) `WEB-OQ-001`이 소유한다.

## 4. Data Display

| 컴포넌트 | 시안 근거 |
|---|---|
| `key-value-row` | Bot State 카드 행 (보조 우측 값 지원) |
| `entity-status-row` | Active Agents 행 (아이콘 타일 + 이름 + 상태) |
| `event-row` | Recent Events 행 (시각 + 아이콘 + 제목 + 보조) |
| `resource-row` | Resource / Health 행 (이름 + 값 + sparkline) |
| `data-table` | Run Queue 테이블 (열 정의, 행 액션, 선택 상태) |
| `data-table-compact-row` | Mobile Run Queue 압축 행 |
| `tab-bar` | `All Runs / My Runs / Watchlist` |
| `legend` | Workflow 상태 legend |
| `tile-grid` | Mobile `System / Agents` 4타일 |
| `empty-state` | 시안 없음. 토큰·밀도 재사용 |
| `error-state` | 시안 없음. 토큰·밀도 재사용 |

## 5. Feature Component

| 컴포넌트 | 시안 근거 |
|---|---|
| `workflow-graph` | Workflow Overview 그래프 전체 |
| `workflow-node` | step 노드(번호/이름/상태/경과 시간) |
| `workflow-io-node` | INPUT/OUTPUT 노드 |
| `workflow-parallel-group` | `PARALLEL JOBS` 점선 그룹 |
| `workflow-edge` | 실선/점선 화살표 |
| `message-list` | Thread 메시지 목록(가상화) |
| `message-item` | avatar + 이름 + BOT 배지 + 시각 + 본문 |
| `message-embed-flow-triggered` | `Flow Triggered` 임베드 카드 |
| `message-embed-run-progress` | `Run Progress` 임베드 카드 |
| `message-composer` | 입력 + 액션 아이콘 + 전송 버튼 |
| `run-queue` | 검색/필터/tab/테이블 조합 |
| `bot-state-card` | Bot State 카드 |
| `agent-list-card` | Active Agents 카드 |
| `recent-events-card` | Recent Events 카드 |
| `resource-health-card` | Resource / Health 카드 |

`workflow-graph`는 이 프로젝트에서 **가장 위험한 컴포넌트**다. 다음을 지킨다.

1. 노드 배치는 데이터에서 계산하고, 좌표를 화면에 하드코딩하지 않는다.
2. 병렬 그룹과 점선 분기·병합을 표현할 수 있어야 한다. 표현 못 하는 라이브러리로 그래프를 리스트로 낮추지 않는다.
3. production에서는 Desktop과 Mobile이 같은 canonical Process graph를 입력으로 받아야 한다. 다만 현재 PNG의 node set과 축약이 실제로 다르므로 `WEB-OQ-014`가 닫히기 전까지 두 visual fixture를 하나의 graph 변환 규칙으로 일반화하지 않는다([06-responsive-layout.md](06-responsive-layout.md)).
4. 노드 수가 늘어날 때 전체 재렌더가 아니라 노드 단위 갱신이 되도록 경계를 나눈다([12-performance-budget.md](12-performance-budget.md)).

## 6. 컴포넌트 규칙

- 의미가 다르면 `variant`로 합치지 않고 별도 컴포넌트로 분리한다.
- 컴포넌트는 자기 데이터를 fetch하지 않는다. 데이터는 화면/feature 경계에서 주입한다.
- 컴포넌트는 Domain 판정을 하지 않는다. "이 버튼을 눌러도 되는가"는 서버가 제공한 값으로 결정한다([01-scope-and-ownership.md](01-scope-and-ownership.md)).
- 새 컴포넌트를 추가할 때 이 표에 시안 근거 또는 파생 근거를 함께 기록한다.
- 플랫폼 분기 방식은 `WEB-OQ-007`에서 stack이 확정된 뒤 해당 framework convention으로 정한다. Expo/RNW가 채택되는 경우 `.web.tsx` / `.native.tsx` 접미사를 사용할 수 있으나, 현재 이 이름을 규범으로 고정하지 않는다.
- 어떤 stack에서도 Web 전용 API(`window`, DOM 이벤트, CSS-only 트릭)를 cross-platform 공통 모듈에 두지 않는다.

## 7. 검증

- 인벤토리에 없는 ad-hoc 컴포넌트가 화면 파일에 인라인으로 생기지 않았는지 확인
- 동일 의미 컴포넌트의 중복 구현 여부 확인 (Review 1 항목)
- 각 컴포넌트가 token만으로 스타일링되는지 확인
- orphan/dead 컴포넌트 확인

## 관련 문서

- [02-screen-inventory.md](02-screen-inventory.md) — 컴포넌트의 출처
- [04-design-tokens.md](04-design-tokens.md) — 스타일 값
- [06-responsive-layout.md](06-responsive-layout.md) — 폼팩터별 조합

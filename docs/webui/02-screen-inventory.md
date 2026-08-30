---
title: "Web UI 화면 인벤토리 — Workspace Cockpit 해부"
document_id: "DXB-WEB-002"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-000"]
---
# Web UI 화면 인벤토리 — Workspace Cockpit 해부

## 목적

시안이 확정한 화면의 **영역 구조와 각 영역의 구성 요소**를 소유한다. 시각 값(색·간격·크기)은 소유하지 않는다. 값은 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §4 절차로 추출해 [04-design-tokens.md](04-design-tokens.md)가 소유한다.

## 1. Canonical Screen은 하나다

시안이 확정한 화면은 **Workspace Cockpit** 하나다. 이름은 `workspace-cockpit`으로 고정한다.

```text
workspace-cockpit
├─ desktop-layout       ← sample/desktop-main-dark.png, sample/desktop-main-light.png
└─ mobile-layout        ← sample/mobile-main-dark.png.png, sample/mobile-main-light.png
```

두 layout은 **다른 화면이 아니라 같은 화면의 다른 폼팩터**다. 동일한 Cockpit 목적과 컴포넌트 어휘를 공유하지만, PNG의 mock data와 node/row 집합까지 동일하지는 않다. 확인된 차이는 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §6과 `WEB-OQ-014`~`017`이 추적한다.

용어 주의: 폼팩터 표현을 `projection`이라 부르지 않고 `layout`이라 부른다. DXBOT에서 `Projection`은 canonical state의 read model을 뜻하는 Domain 용어이며([documentation-rules.md](../agent/documentation-rules.md) §7), 화면 배치와 섞으면 의미가 흐려진다.

Cockpit의 성격은 다음과 같다.

- 대화(Thread), 실행 그래프(Workflow Overview), 실행 대기열(Run Queue), Runtime 상태(Bot State / Agents / Events / Health)를 **한 화면에서 동시에** 관찰한다.
- 사용자는 화면을 이동하지 않고 "지금 무슨 일이 벌어지는가"와 "무엇을 지시할 것인가"를 함께 처리한다.
- 따라서 Cockpit은 여러 페이지의 요약 진입점이 아니라 **작업 자체가 일어나는 장소**다.

Cockpit이 표현하는 Domain 의미에는 제약이 있다. 특히 Project와 Durable Process가 없는 Bot-only 상태에서도 Cockpit은 동작해야 하고(`INV-012`), `Active Agents`와 `PARALLEL JOBS`의 의미는 불변조건과 충돌할 수 있다. 규칙은 [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md)가 소유한다.

## 2. Desktop layout 구조

`sample/desktop-main-dark.png` 기준. 상단 1행 header + 하단 4-column body다.

```text
┌───────────┬──────────────────────────────────────────────────────────────┐
│  nav rail │                       global header                          │
│           ├───────────────┬──────────────────────────┬───────────────────┤
│  logo     │ Thread /      │  Workflow Overview       │  Bot State        │
│  nav      │ Conversation  │  (graph card)            │  Active Agents    │
│  favorites│ (message list)├──────────────────────────┤  Recent Events    │
│  collapse │               │  Run Queue               │  Resource/Health  │
│  env pill │ composer      │  (tabs + table)          │                   │
└───────────┴───────────────┴──────────────────────────┴───────────────────┘
```

폭 관계는 `nav rail < right column ≈ thread column < center column`이다. 정확한 비율은 PNG에서 추출한다.

### 2.1 Navigation rail (좌측, 전체 높이)

- 상단: 제품 로고 마크 + `DXBOT` 워드마크
- 주 내비게이션(아이콘 + 라벨, 위→아래): `Workspace`, `Bots`, `Flows`, `Tasks`, `Memory`, `Settings`
- 활성 항목은 배경 surface + accent 텍스트/아이콘으로 구분한다(색만으로 구분하지 않는 요구는 [11-accessibility.md](11-accessibility.md))
- 구분 여백 후 `FAVORITES` 섹션 라벨(대문자 caption)
- Favorites 항목: 엔티티 이름 + 상태 dot 또는 타입 아이콘 (시안 예: `DXBOT Brain`(상태 dot), `Data Sync Flow`, `Release Pipeline`)
- 하단: rail 축소/확장 토글(원형 버튼), 그 아래 환경 선택 pill (시안 예: `Acme Production` + 상태 dot + chevron)

### 2.2 Global header (상단 1행)

세로 구분선으로 나뉜 라벨 + 값 셀의 연속이다. 각 셀은 `caption 라벨` 위, `값` 아래 구조다.

| 셀 | 구성 |
|---|---|
| Workspace | 조직/워크스페이스 이름 + chevron (선택 가능) |
| Active Bot | Bot 아이콘 + Bot 이름 + 상태 dot |
| Current Branch / Run | branch 아이콘 + branch 이름 + 구분점 + Run 링크. Branch의 Domain 대응은 미확정(`WEB-OQ-004`) |
| Memory State | 상태 라벨 + 퍼센트 + 가로 progress bar |
| Progress | `n / m steps` + 퍼센트 + 가로 progress bar |
| Sessions | 사용자 아이콘 + 개수 + sparkline |
| (우측 끝) | 사용자 avatar + presence dot |

Memory State와 Progress의 bar는 색이 다르다(상태 계열 vs accent 계열). 이는 상태 표현 매핑([04-design-tokens.md](04-design-tokens.md) §5)을 따른다.

### 2.3 Thread / Conversation column

- 카드 header: 제목 `Thread / Conversation` + 필터 icon-button + kebab(overflow) icon-button
- 메시지 항목 구성: 좌측 avatar(사용자 이니셜 / Bot 마크) → 이름 → `BOT` 배지(Bot 메시지에만) → 우측 정렬 타임스탬프 → 본문 텍스트
- 메시지 본문 아래 **임베드 카드**가 붙는다. 시안이 확정한 2종:
  - `Flow Triggered`: 성공 icon + 제목, 보조행(`Flow 이름 · Run 링크`), 우측 또는 하단에 `View Run` 버튼
  - `Run Progress`: 제목 + progress bar + 퍼센트 + `n / m steps completed` + `Est. hh:mm:ss remaining`
- 하단 고정 composer: placeholder(`Message {botName}...`), 하단 좌측 아이콘 3종(첨부/멘션/이모지), 우측 accent 원형 전송 버튼
- 스크롤은 메시지 목록만 발생하고 composer는 고정된다.

### 2.4 Center column

**Workflow Overview 카드**

- header: `Workflow Overview – {Flow 이름}` + Run 배지 pill + 확대(expand) icon-button + kebab
- 그래프 본문:
  - `INPUT` 캡션 + 입력 노드(문서 아이콘 + 2행 텍스트)
  - 순차 step 노드: `번호. 이름` + 상태 아이콘 + 상태 텍스트 + 경과 시간
  - 병렬 구간: `PARALLEL JOBS` 라벨이 붙은 점선 그룹 박스가 하위 노드 2개를 감싼다
  - 대기 step 노드: 상태 아이콘 + `Queued`
  - `OUTPUT` 캡션 + 출력 노드(메일 아이콘 + 2행 텍스트)
  - 연결선: 실선 화살표(순차), 점선 화살표(분기/병합). 점선은 병렬 구간 계열 색을 사용한다
  - 실행 중 노드는 테두리 강조 + 회전 spinner 아이콘을 가진다
- 하단 legend 행: `Completed` / `Running` / `Queued` / `Pending`. 각각 **모양이 다른 아이콘**을 사용한다(색만으로 구분하지 않는다)

**Run Queue 카드**

- header: 제목 + 개수 배지, 우측에 검색 입력(아이콘 포함) + 필터 icon-button
- tab 행: `All Runs` / `My Runs` / `Watchlist`, 활성 tab은 밑줄 + accent
- table 열: `Run ID`(링크) / `Flow` / `Status`(dot + 텍스트) / `Progress`(mini bar + %) / `Started` / `Owner` / 행 kebab
- 선택된 행은 배경 강조를 가진다(Light 시안에서 확인 가능)
- footer: 가운데 정렬 `View all runs` 링크

### 2.5 Right column (상태 스택)

| 카드 | 구성 |
|---|---|
| Bot State | header 제목 + 상태 pill(`Operational`). key-value 행: Bot(+버전 보조값), Brain(+버전), State, Uptime, Sessions, Last Heartbeat(dot + 시각) |
| Active Agents | header 제목 + 개수 배지. 행: 아이콘 타일 + agent 이름 + 우측 상태(dot + 텍스트). footer 링크 `View all agents` |
| Recent Events | header 제목 + `View all` 링크. 행: 시각(등폭, 좌측) + 타입 아이콘 + 제목 + 보조 설명 |
| Resource / Health | 행: 자원명 + 값(%/단위) + sparkline. footer: 성공 아이콘 + 요약 문장 |

## 3. Mobile layout 구조

`sample/mobile-main-dark.png.png` 기준. 단일 세로 스크롤 + 하단 고정 tab bar다.

```text
app header (logo + wordmark | avatar + overflow)
summary stat strip (4 cells, 세로 구분선)
Workflow Overview card  (compact graph + legend + Run 배지 + chevron)
Thread / Conversation card (View all 링크 + 메시지 + 임베드 카드 + composer)
Run Queue card (View all 링크 + 압축 행 + chevron)
System / Agents card (View all agents 링크 + 4 타일)
bottom tab bar (Home | Runs | (+) FAB | Agents | Menu)
```

Mobile에서 사라지는 것: nav rail, Desktop global header의 다중 셀, 우측 상태 스택의 카드 4장 구조.
Mobile에서 대체되는 것: nav rail → bottom tab bar, global header → app header + stat strip, 우측 상태 스택 → `System / Agents` 카드 + 각 카드의 `View all` 링크.

세부:

- app header: 로고 마크 + `DXBOT`, 우측에 avatar(presence dot) + kebab
- stat strip: 4개 셀, 각 셀은 caption + 값(일부 셀은 dot/퍼센트/mini bar/sparkline 포함). 셀 구성과 값 표현은 Dark/Light 시안이 다르다 → [17-open-decisions.md](17-open-decisions.md) `WEB-OQ-001`
- Workflow Overview: 그래프 어휘와 legend는 유지하지만 Desktop과 동일한 node set은 아니다. Mobile은 `INPUT`을 렌더하지 않고 순차 `4. Update Database` running 노드를 추가한 뒤 병렬 `4A. Update DB` / `4B. Update Cache`를 배치하며 일부 이름을 축약한다. 이는 [17-open-decisions.md](17-open-decisions.md) `WEB-OQ-014`가 닫힐 때까지 visual fixture 차이로만 취급하며 실제 Process graph를 임의 변환하지 않는다. **리스트/stepper로 치환 금지**
- Thread / Conversation: 메시지 2건 + 임베드 카드 + composer가 하나의 카드 안에 들어간다. Desktop처럼 열 전체를 차지하지 않는다
- Run Queue: 테이블이 아니라 압축 행(id / dot+flow / dot+status / % / mini bar / chevron)이며 시안에는 4개 행만 보이지만 count badge는 `5`다. badge total과 visible window의 production 의미는 `WEB-OQ-015` 결정 전 고정하지 않는다
- System / Agents: agent 4개를 한 행 4열 타일로 배치(아이콘 + 이름 + dot + 상태)
- bottom tab bar: `Home`(활성) / `Runs` / 중앙 accent 원형 FAB(`+`) / `Agents` / `Menu`. FAB는 bar 위로 돌출한다
- Desktop Favorites는 Mobile에 직접 대응 영역이 없다. 접근 경로는 `WEB-OQ-017` 결정 전 추정하지 않는다

## 4. 시안이 없는 화면의 처리 규칙

nav rail의 `Bots`, `Flows`, `Tasks`, `Memory`, `Settings`와 각 `View all` 목적지는 **시안이 없다.** 다음 규칙을 적용한다.

1. 시안 없는 화면의 시각 규범을 이 문서 패키지에서 새로 발명하지 않는다.
2. 구현이 필요하면 Cockpit에서 이미 확정된 컴포넌트(카드, key-value 행, 상태 dot, 테이블 행, legend, 배지, pill)를 **조합**해서 만든다. 새 시각 언어·새 밀도·새 색을 도입하지 않는다.
3. 새 컴포넌트가 필요하면 [05-component-inventory.md](05-component-inventory.md)에 추가하고 근거를 남긴다.
4. 시안 없는 화면은 [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md)의 pixel diff 대상이 아니다. 대신 토큰 준수와 컴포넌트 재사용으로 검증한다.
5. 해당 화면의 디자인이 확정되면 PNG를 `sample/`에 추가하고 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §1 표를 같은 change set에서 갱신한다.

## 5. 컴포넌트 추출 결과

이 문서의 영역 해부에서 도출되는 재사용 컴포넌트 목록은 [05-component-inventory.md](05-component-inventory.md)가 소유한다. 같은 목록을 이 문서에 복제하지 않는다.

## 관련 문서

- [00-visual-source-of-truth.md](00-visual-source-of-truth.md) — 절대 시각 기준
- [03-information-architecture.md](03-information-architecture.md) — 내비게이션과 route
- [05-component-inventory.md](05-component-inventory.md) — 컴포넌트 계층
- [06-responsive-layout.md](06-responsive-layout.md) — layout 전환 규칙

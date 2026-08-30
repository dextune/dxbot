---
title: "Web UI 반응형 레이아웃"
document_id: "DXB-WEB-006"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-002"]
---
# Web UI 반응형 레이아웃

## 목적

Desktop layout과 Mobile layout 사이의 전환 규칙, 그리고 시안이 없는 중간 폭(Tablet)의 처리 규칙을 소유한다.

## 1. 시안이 확정한 두 극단

| 폼팩터 | 시안 | 확정 여부 |
|---|---|---|
| Desktop | `sample/desktop-main-dark.png`, `sample/desktop-main-light.png` (1448 × 1086) | 확정 |
| Mobile | `sample/mobile-main-dark.png.png`, `sample/mobile-main-light.png` (941 × 1672, 기기 프레임 포함) | 확정 |
| Tablet | 없음 | **미확정 — 파생** |

Mobile 시안은 기기 목업 프레임 안에 렌더링되어 있다. 실제 콘텐츠 폭은 프레임 내부 영역이며, 프레임·상태바·홈 인디케이터는 구현 대상이 아니다. 콘텐츠 폭 비율은 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §4로 추출한다.

## 2. Breakpoint

```text
desktop  >= 1200px    4-column layout (rail + thread + center + right)
tablet   768–1199px   파생 규칙 (§3)
mobile   < 768px      단일 컬럼 layout
```

숫자보다 **실제 overflow 여부**를 우선한다. 특히 Run Queue 테이블 열과 Workflow 노드 폭이 잘리기 시작하는 지점을 기준으로 조정한다. 조정 시 이 표를 갱신한다.

## 3. Tablet 파생 규칙

시안이 없으므로 새 시각 언어를 만들지 않고 두 확정 layout의 요소만 재배치한다.

1. nav rail은 축소 상태(아이콘 전용)로 전환한다. rail 자체를 없애지 않는다. 축소 상태는 시안의 collapse 토글이 이미 정의한 상태다.
2. 우측 상태 스택은 center column 아래로 이동해 2열 그리드로 배치한다. 카드 내부 구조는 바꾸지 않는다.
3. Thread column과 center column은 유지한다. Thread를 오버레이/드로어로 숨기는 것은 Cockpit 성격을 훼손하므로 마지막 수단이다.
4. 그래도 폭이 부족하면 Mobile layout으로 내린다. 어중간한 3열 재해석을 만들지 않는다.

## 4. Mobile 전환 규칙

| Desktop 요소 | Mobile 대응 |
|---|---|
| nav rail | bottom tab bar + `Menu` 시트 |
| Desktop Favorites | Mobile 직접 대응 없음 — `WEB-OQ-017` 결정 전 임의로 Menu에 넣지 않음 |
| global header 다중 셀 | app header + 4셀 stat strip |
| Thread column | Thread / Conversation 카드 (composer 포함) |
| Workflow Overview | 동일 카드, 노드 배치만 압축 |
| Run Queue 테이블 | 압축 행 리스트 + chevron |
| 우측 상태 카드 4장 | `System / Agents` 카드 + 각 카드의 `View all` 링크 |

세부 규칙:

1. Workflow 그래프는 **그래프로 유지한다.** Mobile 시안은 INPUT을 생략하고 순차 step 4를 추가하며 일부 라벨을 축약한다. `WEB-OQ-014`가 닫히기 전 이 차이를 일반 responsive algorithm으로 고정하지 않고 각 PNG visual fixture로 재현한다. 병렬 그룹과 legend를 유지하며 리스트/stepper 치환은 금지다.
2. Run Queue는 테이블을 그대로 축소하지 않고 압축 행으로 재배치한다. 시안은 total badge `5`와 visible row 4개를 보이지만 truncation/pagination 의미는 `WEB-OQ-015` 결정 전 고정하지 않는다. 가로 스크롤 테이블을 만들지 않는다.
3. composer는 Thread 카드 내부에 있고, 키보드가 올라올 때 입력 영역이 가려지지 않아야 한다.
4. bottom tab bar와 FAB는 safe-area inset을 존중한다.
5. 터치 타겟은 최소 44×44 논리 픽셀을 확보한다. 시안의 시각 크기가 그보다 작아 보이면 **시각 크기는 유지하고 히트 영역만 확장**한다.

## 5. 검증 viewport

```text
1448 x 1086   시안 Desktop 원본 비율 (1:1 diff 기준)
Mobile content viewport  Phase 0에서 941×1672 device frame 내부를 측정해 1:1 diff 기준으로 기록
1440 x 1024
1280 x 800
1024 x 768    tablet 경계
 768 x 1024   tablet 하단 경계
 430 x 932
 390 x 844
 360 x 800    mobile 최소 지원 폭
```

`1024×768`과 `768×1024` Tablet은 reference PNG가 없으므로 pixel baseline을 만들지 않는다. §3 파생 규칙, token, component reuse, interaction reachability로 검증한다.

각 viewport에서 확인할 항목:

- 가로 스크롤 발생 여부 (허용하지 않음)
- 텍스트 잘림·말줄임 위치
- 카드 내부 요소 겹침
- 테이블/그래프 overflow
- 고정 요소(composer, tab bar)와 콘텐츠 충돌
- 긴 이름·긴 메시지의 줄바꿈

절차와 baseline 관리는 [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md)가 소유한다.

## 6. 금지 사항

- 폼팩터별로 화면 컴포넌트를 각각 복제 작성
- breakpoint 분기를 개별 컴포넌트에 흩뿌리기 (shell/layout 계층에서 결정한다)
- Mobile에서 Desktop 테이블을 가로 스크롤로 그대로 노출
- Desktop에서만 동작하는 hover 전용 액션 (터치에서 도달 불가한 기능을 만들지 않는다)

## 관련 문서

- [02-screen-inventory.md](02-screen-inventory.md) — 두 layout의 구조
- [05-component-inventory.md](05-component-inventory.md) — layout 컴포넌트
- [11-accessibility.md](11-accessibility.md) — 터치·키보드 요구

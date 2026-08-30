---
title: "Web UI Visual Source of Truth — sample/*.png 절대 기준"
document_id: "DXB-WEB-000"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
---
# Web UI Visual Source of Truth — sample/*.png 절대 기준

## 목적

이 문서는 DXBOT Web UI의 **시각적 정답이 무엇인지**를 소유한다. 색·간격·타이포·레이아웃·컴포넌트 형태에 대한 모든 분쟁은 이 문서가 지정한 파일로 판정한다. 다른 문서는 값을 복제하지 않고 이 문서를 참조한다.

## 1. 절대 기준 파일 — 협상 불가

DXBOT Web UI의 시각 기준은 **저장소의 `sample/` 디렉터리에 있는 PNG 파일 그 자체**다. 대화 기록, 요약문, 기억, 일반적인 Admin Dashboard 관례, 이 문서 패키지의 산문 설명은 **모두 기준이 아니다**.

| 역할 | 파일 | 크기(px) | sha256 (앞 16자) |
|---|---|---|---|
| Desktop / Dark (Primary) | [`sample/desktop-main-dark.png`](../../sample/desktop-main-dark.png) | 1448 × 1086 | `583b199065bd10d4` |
| Desktop / Light | [`sample/desktop-main-light.png`](../../sample/desktop-main-light.png) | 1448 × 1086 | `396ce58e7d991f62` |
| Mobile / Dark | [`sample/mobile-main-dark.png.png`](../../sample/mobile-main-dark.png.png) | 941 × 1672 | `2fa0054542f2d4b9` |
| Mobile / Light | [`sample/mobile-main-light.png`](../../sample/mobile-main-light.png) | 941 × 1672 | `2bb178e73c1a9b50` |

`mobile-main-dark.png.png`의 이중 확장자는 현재 저장소의 **실제 파일명**이다. 문서는 실제 파일명을 그대로 기록한다. 파일명을 정리하려면 rename과 이 문서의 표 갱신을 **같은 change set**에서 수행한다.

sha256은 시안 교체를 감지하기 위한 것이다. 해시가 달라졌다면 시안이 변경된 것이므로 [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md)의 baseline 재승인 절차를 먼저 수행한다.

## 2. 구속력 있는 규칙

1. **PNG가 정답이다.** 구현 결과가 PNG와 다르면 구현이 틀린 것이다. 반대 방향의 추정은 허용하지 않는다.
2. **재해석 금지.** "더 나은 UX", "일반적인 패턴", "Material/Ant/Tailwind 기본값", "이 정도면 비슷하다"를 근거로 레이아웃·밀도·계층·색을 바꾸지 않는다.
3. **치환 금지.** 시안의 3-column Cockpit을 흔한 좌측 사이드바 + 단일 콘텐츠 Dashboard로 치환하지 않는다. 시안의 Workflow 그래프를 리스트나 stepper로 치환하지 않는다. 시안의 Run Queue 테이블을 카드 그리드로 치환하지 않는다(Mobile 규칙은 [06-responsive-layout.md](06-responsive-layout.md)가 소유한다).
4. **값은 추측하지 않고 추출한다.** color/spacing/radius/font-size는 눈대중이 아니라 PNG **픽셀 샘플링**으로 확정한다. 절차는 §4에 있다.
5. **동작 요구와 충돌할 때만 예외.** 시안이 표현하지 못하는 동작 요구(빈 상태, 실패, 스트리밍 중단, 권한 거부 등)는 [10-error-and-exceptional-state.md](10-error-and-exceptional-state.md)가 소유한다. 이때도 시안의 시각 언어(토큰·컴포넌트·밀도)를 그대로 사용하고, 새 시각 언어를 발명하지 않는다.
6. **시안 없는 화면은 "미확정"으로 취급한다.** 시안에 존재하지 않는 화면을 상상해서 규범으로 굳히지 않는다. 파생 규칙은 [02-screen-inventory.md](02-screen-inventory.md) §4가 소유한다.
7. **AI Agent 작업 규칙.** Web UI 구현·수정 작업을 시작하는 Agent는 코드를 쓰기 전에 해당 화면의 PNG를 **실제로 열어서 확인**한다. 이 문서 패키지의 산문만 읽고 구현하지 않는다.

## 3. 시안의 개수와 의미 — 흔한 오독 교정

`sample/`에는 PNG가 4개 있다. 이것은 **4개의 서로 다른 화면이 아니다.**

```text
sample/*.png  =  1개 화면(Workspace Cockpit)
               × 2개 플랫폼 폼팩터(Desktop, Mobile)
               × 2개 색상 모드(Dark, Light)
```

즉 시안이 확정한 것은 **단일 통합 화면**이며, Dashboard / Bot Detail / Conversation / Workflow를 각각 다른 페이지로 분리하는 구조는 **시안이 승인한 바 없다**. 시안은 오히려 그 반대를 주장한다. 대화, 워크플로우 그래프, Run Queue, Bot/Runtime 상태를 **한 화면에 동시에** 배치한다.

이 구분을 잃으면 구현이 시안과 근본적으로 달라진다. 상세한 화면 정의는 [02-screen-inventory.md](02-screen-inventory.md)가 소유한다.

## 4. 값 추출 절차 (Phase 0 필수 산출물)

구현 이전에 다음을 PNG에서 직접 측정해 `visual-spec` 표로 기록한다. 측정 결과의 Canonical Owner는 [04-design-tokens.md](04-design-tokens.md)다.

측정 대상:

```text
canvas 비율 및 안전 여백
좌측 rail 폭 (확장/축소 상태)
global header 높이 및 셀 구분선 위치
3개 content column의 폭 비율
card padding / gap / radius / border 두께
그림자 유무와 강도
font family 계열, size, weight, line-height 단계
icon box 크기와 stroke 두께
status dot 지름
progress bar 높이와 radius
table row 높이, header 높이, 정렬 기준
mobile stat strip 셀 높이와 구분선
mobile bottom tab bar 높이, FAB 지름과 돌출량
```

측정 방법:

1. PNG를 100% 배율로 열고 좌표를 읽는다. 확대·보간된 뷰에서 값을 읽지 않는다.
2. color는 **단색 영역의 중앙 픽셀**을 샘플링한다. 경계·그라디언트·안티에일리어싱 픽셀을 샘플링하지 않는다.
3. 같은 의미의 색은 Dark/Light 두 파일에서 **쌍으로** 추출해 하나의 semantic token key에 매핑한다.
4. 각 값에 출처를 남긴다: `파일명 + 대략 좌표 + 대상 요소`. 출처 없는 값은 토큰으로 승인하지 않는다.

산출물:

- `token` ← `sample 파일/좌표` 역추적 표
- screen inventory ([02-screen-inventory.md](02-screen-inventory.md))
- component inventory ([05-component-inventory.md](05-component-inventory.md))

## 5. 관찰된 시각 특성 (비규범 요약)

아래는 방향을 잃지 않기 위한 **요약이며 규범이 아니다.** 실제 값은 §4로 확정한다.

- 정보 밀도가 높은 운영 콘솔 계열이다. 여백이 넉넉한 마케팅형 Dashboard가 아니다.
- 카드 경계가 얇고 대비가 낮다. 강한 그림자나 두꺼운 테두리로 카드를 분리하지 않는다.
- 색·모양과 Domain 상태의 대응은 [04-design-tokens.md](04-design-tokens.md) §5만 소유한다. 이 요약의 accent 관찰을 상태 의미표로 사용하지 않는다.
- Dark와 Light는 **동일한 레이아웃·간격·타이포**를 공유하고 색만 교체한다. 구조 차이는 없다.
- 숫자·시간·ID는 시각적으로 정렬되어 있다. 등폭 계열 처리와 우측 정렬 규칙을 지킨다.

## 6. 알려진 시안 간 차이

시안 파일 사이에는 색 모드별 mock 데이터 차이뿐 아니라 Desktop/Mobile 구조 차이도 존재한다. PNG 충실도와 실제 제품 의미를 분리하기 위해 임의로 통일하지 않고 [17-open-decisions.md](17-open-decisions.md)에서 결정한다.

| 비교 | 관찰된 차이 | 처리 |
|---|---|---|
| Mobile Dark ↔ Light stat strip | Dark: Bot / State / Dynamic Core / Sessions. Light: Bot / Run / Memory / Progress. 값 표현도 dot, 단순 text, version, sparkline, progress bar처럼 다르다 | `WEB-OQ-001`. `header-metric-cell`이 지원할 표현 범위와 실제 metric 집합을 함께 결정 |
| Desktop ↔ Mobile Workflow | Mobile은 `INPUT` 노드를 렌더하지 않고, 순차 `4. Update Database` running 노드를 추가한 뒤 병렬 `4A. Update DB` / `4B. Update Cache`를 표시한다. Desktop은 `INPUT`과 1–3 다음 병렬 4A/4B를 표시한다 | `WEB-OQ-014`. 같은 Process를 다른 축약으로 본 것인지 서로 다른 fixture인지 결정 전 실제 Process에 결박하지 않음 |
| Desktop ↔ Mobile Workflow 라벨 | Mobile은 `Update Database → Update DB`, `Notify Analytics Team → Notify Team`처럼 일부 라벨을 축약한다 | `WEB-OQ-014`. PNG 재현용 승인 표현이며 임의 치환의 일반 허가가 아님 |
| Desktop ↔ Mobile Run Queue | Desktop은 5개 행, Mobile은 4개 행을 보이지만 둘 다 count badge는 `5`다 | `WEB-OQ-015`. badge=total과 visible-window 관계, truncation 규칙을 결정 |
| Desktop favorites ↔ Mobile navigation | Desktop rail에는 Favorites 3개가 있으나 Mobile에는 직접 대응 영역이 없다 | `WEB-OQ-017`. `Menu` 시트에 넣을지 별도 접근 경로를 둘지 결정 |
| Dark ↔ Light Workflow 시간 | 같은 위치의 일부 경과 시간이 다르다 | 시각 규범이 아닌 mock 데이터 차이로 취급 |

Desktop Dark/Light 사이에는 mock 값 외 구조적 차이가 관찰되지 않는다. 따라서 Desktop 구조 판정의 Primary는 `desktop-main-dark.png`이며 Light는 색 매핑 검증용이다.

## 7. 검증

- Phase 0 종료 시 `visual-spec` 표의 모든 토큰에 sample 출처가 있는지 확인한다.
- 화면 구현 시 동일 viewport에서 reference/구현 스크린샷을 만들어 diff한다. 절차와 허용 오차는 [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md)가 소유한다.
- 시안 파일 해시 변경은 디자인 변경으로 간주하고 baseline 재승인 없이 반영하지 않는다.

## 관련 문서

- [index.md](index.md) — Web UI 문서 Router
- [02-screen-inventory.md](02-screen-inventory.md) — 시안이 정의하는 실제 화면 구성
- [04-design-tokens.md](04-design-tokens.md) — 추출된 값의 Canonical Owner
- [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) — 시각 검증
- [17-open-decisions.md](17-open-decisions.md) — 미결정 사항

---
title: "Web UI 접근성 요구"
document_id: "DXB-WEB-011"
status: "Accepted"
normative: true
priority: "web-P1"
plan_baseline: "0.8.10"
last_updated: "2026-08-29"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-004", "DXB-WEB-005"]
---
# Web UI 접근성 요구

## 목적

시안의 시각을 유지하면서 만족해야 하는 접근성 최소 기준을 소유한다.

## 1. 시안과 접근성이 충돌할 때

시안의 **시각적 결과는 유지**하고 접근성은 히트 영역·라벨·포커스 표현으로 확보한다. 접근성을 이유로 레이아웃·밀도·색 체계를 바꾸지 않는다. 그것으로 해결되지 않는 항목만 [17-open-decisions.md](17-open-decisions.md)에 올린다.

## 2. 필수 요구

**키보드**

- 모든 조작 가능 요소는 Tab 순서에 포함된다. rail 항목, tab, 테이블 행 액션, kebab 메뉴, composer 액션, FAB를 포함한다.
- 포커스 표시는 `focus-ring` token으로 명확히 보인다. `outline: none`만 적용하고 대체 표시를 두지 않는 구현을 금지한다.
- 모달/시트는 focus trap과 Escape 닫기를 갖는다. 닫은 뒤 이전 포커스로 복귀한다.
- 테이블·목록은 방향키 이동을 지원하고, 행 진입은 Enter로 가능하다.
- composer에서 전송 키 조합과 줄바꿈 키 조합이 구분된다.

**스크린 리더 / 라벨**

- icon-only 버튼(필터, kebab, expand, collapse, 첨부/멘션/이모지, FAB)은 접근성 라벨을 갖는다.
- 상태 dot·progress bar·sparkline은 텍스트 대안을 갖는다. 색·그래픽만으로 정보를 전달하지 않는다.
- Workflow 그래프는 노드 상태와 연결 관계를 순차 탐색 가능한 대안으로 제공한다. 그래프를 이미지로만 렌더링하지 않는다.
- 실시간 갱신 영역(Recent Events, Run 상태)은 과도한 알림을 만들지 않는 범위에서 변경을 알린다.

**색과 대비**

- 상태는 색 + 아이콘 모양 + 텍스트의 3중 표현을 유지한다([04-design-tokens.md](04-design-tokens.md) §5). 시안의 legend가 이미 모양을 구분하고 있으므로 이를 훼손하지 않는다.
- 본문 텍스트와 배경 대비를 Dark/Light 두 모드에서 모두 확인한다. `text-muted`가 대비 요구를 만족하지 못하면 token 값을 조정하고 근거를 남긴다.
- 링크는 색만으로 구분하지 않는다.

**터치와 확대**

- 터치 타겟 최소 44×44 논리 픽셀. 시각 크기 유지 + 히트 영역 확장으로 확보한다([06-responsive-layout.md](06-responsive-layout.md) §4).
- OS 글자 크기 확대에서 레이아웃이 깨지지 않는다. 고정 높이 컨테이너에 텍스트를 가두지 않는다.
- 200% 확대에서 가로 스크롤이 발생하지 않는다.

**모션**

- reduced motion 설정을 존중한다. spinner·전환·sparkline 애니메이션을 정적 표현으로 대체한다.
- 애니메이션은 상태 이해에 필요한 범위로 제한한다.

## 3. 검증

- 마우스 없이 주요 흐름(메시지 전송, Run 열기, tab 전환, 모달 열고 닫기)을 완주한다.
- 각 icon-only 버튼의 접근성 라벨 존재를 컴포넌트 테스트로 검사한다.
- Dark/Light 각각에서 대비를 측정한다.
- reduced motion, 글자 확대, 200% 확대 상태를 수동 확인한다.

## 관련 문서

- [05-component-inventory.md](05-component-inventory.md) — 컴포넌트별 요구
- [10-error-and-exceptional-state.md](10-error-and-exceptional-state.md) — 오류 전달
- [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) — 검증 절차

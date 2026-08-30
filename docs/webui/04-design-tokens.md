---
title: "Web UI Design Token"
document_id: "DXB-WEB-004"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-29"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-000"]
---
# Web UI Design Token

## 목적

시안에서 추출한 시각 값의 **key 체계와 추출 규칙**을 소유한다. 실제 값은 코드의 token 정의가 소유하고, 이 문서는 key와 근거 추적 방식을 고정한다.

## 1. 값의 출처는 언제나 sample PNG다

토큰 값은 [00-visual-source-of-truth.md](00-visual-source-of-truth.md) §4 절차로 PNG에서 추출한다. 추측·기억·프레임워크 기본값·디자인 시스템 관례에서 값을 가져오지 않는다.

각 토큰은 다음 형태의 근거를 가진다.

```text
token-key | value(dark) | value(light) | 출처 파일 | 대략 좌표 | 대상 요소
```

근거 없는 토큰은 승인하지 않는다. 이 표(`visual-spec`)는 구현 저장소의 token 소스와 같은 위치에서 관리한다([13-frontend-repository-layout.md](13-frontend-repository-layout.md)).

## 2. Semantic color token

컴포넌트는 hex를 직접 쓰지 않고 semantic key만 참조한다. Dark/Light는 동일 key에 다른 값을 매핑한다.

```text
background            페이지 최하단 배경
surface               카드 기본 배경
surface-raised        카드 위 요소(입력, 내부 노드) 배경
surface-muted         비활성/보조 영역 배경
surface-selected      선택된 테이블 행/내비 항목 배경
border                기본 경계선
border-strong         강조 경계선(실행 중 노드 등)
border-dashed         병렬 그룹 점선 경계
text-primary          본문/값
text-secondary        보조 설명
text-muted            caption/라벨/비활성
text-link             링크(Run ID, View all)
accent                주 강조(활성 내비, 전송 버튼, FAB, progress)
accent-hover
accent-pressed
accent-on             accent 배경 위 전경색
status-success        완료/정상
status-running        실행 중
status-queued         큐/대기
status-warning        주의
status-danger         실패
status-neutral        미시작/비활성
sparkline-stroke
overlay               모달/시트 배경
focus-ring            키보드 포커스 링
```

규칙:

1. 상태 색은 `status-*`만 사용한다. 화면에서 초록/빨강을 직접 고르지 않는다.
2. `accent`와 `status-running`이 같은 계열로 보여도 **다른 key로 유지**한다. 의미가 다르면 key를 합치지 않는다.
3. Dark/Light 값은 반드시 쌍으로 추출한다. 한쪽만 정의된 key를 만들지 않는다.
4. 투명도 변형이 필요하면 별도 key를 만들고, 컴포넌트에서 알파를 즉석 계산하지 않는다.

## 3. Layout / Radius token

```text
space-1 space-2 space-3 space-4 space-5 space-6 space-8 space-10 space-12
radius-sm radius-md radius-lg radius-xl radius-full
border-width-hairline border-width-strong
```

단계 수는 추출 결과에 맞춘다. 시안이 사용하지 않는 단계를 미리 만들지 않는다. 시안에 존재하는 간격이 단계에 없으면 단계를 추가하고 근거를 남긴다.

## 4. Typography token

```text
display        대형 수치 표시
heading-1      화면 제목
heading-2      카드 header 제목
heading-3      섹션 소제목
body           본문(메시지, 값)
body-strong    강조 본문(이름, 상태 텍스트)
caption        header 셀 라벨, FAVORITES 라벨
label          배지/pill/tab
mono           시각, Run ID, 경과 시간, 퍼센트
```

규칙:

- 각 key는 `font-family / size / weight / line-height / letter-spacing`을 한 묶음으로 정의한다.
- 시안의 시각·ID·경과 시간은 정렬되어 보인다. `mono` 또는 등폭 숫자(tabular numbers)를 사용하고, 폭이 흔들리는 비례 숫자를 쓰지 않는다.
- 화면에서 font size를 직접 지정하지 않는다.

## 5. Status presentation 매핑

이 절은 **표현**만 소유한다. `Domain 상태 → UI 상태 라벨`의 의미 매핑은 [03-information-architecture.md](03-information-architecture.md) §5가 소유한다. 두 문서에 같은 매핑을 복제하지 않는다.

표현 매핑이 결정하는 것: `아이콘 모양`, `색 token`, `라벨 텍스트`, `progress bar 색`. 구현은 **하나의 테이블**이 소유하고 화면·컴포넌트마다 다시 정의하지 않는다.

시안이 시각을 확정한 상태(Workflow legend 및 Run Queue 기준):

```text
completed    성공 계열 + 체크 아이콘
running      실행 계열 + 회전/열린 원 아이콘
queued       큐 계열 + 점 채워진 원 아이콘
pending      중립 계열 + 빈 원 아이콘
failed       위험 계열 + 오류 아이콘
```

시안에 시각이 없으나 Domain 상태기계에 존재하므로 **표현을 반드시 정의해야 하는** 상태:

```text
suspended  waiting  deferred  rejected  cancelled  recovery-required
degraded   archived  restoring  provisioning  activating  quiescing
approval-required   superseded
```

전이 중 상태(`Suspending / Resuming / Completing / Cancelling`)는 별도 색을 만들지 않고 목적 상태 표현 + 진행 표시로 조합한다.

연결·관측 상태는 Domain 상태가 아니므로 별도 축으로 표현한다.

```text
offline  reconnecting  resync-required  unknown
```

규칙:

1. 상태를 **색만으로** 구분하지 않는다. 아이콘 모양과 텍스트를 함께 제공한다([11-accessibility.md](11-accessibility.md)).
2. 표현이 없는 Domain 상태 값을 임의로 가까운 상태에 흡수하지 않는다. `unknown`으로 안전 렌더링하고 UI에 "알 수 없는 상태"로 드러낸다.
3. 서로 다른 상태기계의 값을 같은 배지 컴포넌트로 렌더링해도 무방하지만, 같은 필드에 섞어 담지 않는다.
4. 라벨 텍스트는 [03-information-architecture.md](03-information-architecture.md) §5의 UI 라벨을 사용한다.

## 6. 금지 사항

- 컴포넌트/화면 파일의 hex, px, rem 리터럴
- 같은 값을 여러 token key에 중복 정의
- Dark 전용 스타일시트와 Light 전용 스타일시트의 분리 작성
- 시안 근거 없는 token 추가

## 7. 검증

- token 소스의 모든 key에 `visual-spec` 근거 행이 있는지 확인
- 화면/컴포넌트 파일에 스타일 리터럴이 없는지 확인
- Dark/Light 값 쌍의 누락 여부 확인
- 상태 매핑 테이블 단일성 확인

## 관련 문서

- [00-visual-source-of-truth.md](00-visual-source-of-truth.md) — 추출 절차
- [05-component-inventory.md](05-component-inventory.md) — token 소비 지점
- [07-theme-and-color-mode.md](07-theme-and-color-mode.md) — 모드 전환

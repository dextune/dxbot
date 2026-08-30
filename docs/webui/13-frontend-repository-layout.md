---
title: "Frontend 저장소 배치와 기술 스택"
document_id: "DXB-WEB-013"
status: "Proposed"
normative: false
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-001"]
---
# Frontend 저장소 배치와 기술 스택

## 목적

Web UI 코드가 이 저장소의 어디에 놓일지와 기술 스택 **후보**를 기록한다. `WEB-OQ-007`이 닫히고 첫 구현 change가 승인되기 전에는 이 문서가 실제 배치나 dependency를 규범으로 고정하지 않는다. 배치는 [repository/repository-layout.md](../agent/repository/repository-layout.md)와 [repository/naming-and-paths.md](../agent/repository/naming-and-paths.md)를 따른다.

이 문서의 status는 `Proposed`다. 첫 구현 커밋에서 실제 배치가 확정되면 `Accepted`로 올리고 실제 트리와 일치시킨다.

## 1. 현재 저장소 상태

- 저장소 루트는 Rust workspace다. `Cargo.toml`의 members는 모두 `crates/*`다.
- Frontend 코드, Node 툴체인, package manifest는 아직 존재하지 않는다.
- `.gitignore`는 `/target/`, `*.db*`, `.DS_Store`만 무시한다. Node 산출물 무시 규칙이 없다.

따라서 첫 Frontend 커밋은 **새 top-level directory 도입**을 포함한다. 이는 독립 빌드 툴체인·독립 배포 산출물·독립 의존성 트리를 갖기 때문에 정당하다.

## 2. 배치

```text
apps/
└─ control-center/
   ├─ app/                    승인된 route만 배치 (03 문서의 후보 route 전체를 선생성하지 않음)
   ├─ src/
   │  ├─ components/
   │  │  ├─ primitives/
   │  │  ├─ layout/
   │  │  ├─ data-display/
   │  │  └─ feedback/
   │  ├─ features/
   │  │  ├─ cockpit/          workspace-cockpit 조립
   │  │  ├─ conversation/
   │  │  ├─ workflow/
   │  │  ├─ runs/
   │  │  └─ runtime-state/
   │  ├─ api/                 client + query 정의
   │  ├─ api/generated/       생성물 전용 (수동 수정 금지)
   │  ├─ mocks/               gateway 도입 전 임시 데이터 격리
   │  ├─ theme/               token 정의 + visual-spec 근거 표
   │  ├─ view-model/
   │  ├─ hooks/
   │  ├─ state/               UI-only 상태
   │  └─ types/
   ├─ assets/
   └─ visual/                 reference/구현 스크린샷과 diff 산출물
```

규칙:

1. directory·파일 이름은 lowercase kebab-case다. React 관례를 이유로 PascalCase 파일명을 만들지 않는다.
2. `src/api/generated/`는 생성물 전용이다. 손수 작성 파일을 섞지 않는다([09-type-contract.md](09-type-contract.md)).
3. `mocks/`는 실제 연결 단계에서 제거한다.
4. `theme/`이 token과 `visual-spec` 근거 표를 함께 소유한다([04-design-tokens.md](04-design-tokens.md) §1).
5. `common/`, `utils/`, `helpers/`, `misc/` 같은 소유권 없는 디렉터리를 만들지 않는다.

## 3. `packages/`는 지금 만들지 않는다

Web과 Native가 실제로 **독립 재사용**하는 계약·토큰·클라이언트가 생길 때까지 `packages/`를 도입하지 않는다. 하나의 앱만 소비하는 코드를 별도 package로 쪼개는 것은 [repository-layout.md](../agent/repository/repository-layout.md)의 "독립 lifecycle 없으면 package를 늘리지 않는다"에 어긋난다.

Native 앱이 실제로 추가되는 시점에 다음 순서로 판단한다.

```text
두 앱이 같은 코드를 쓰는가?
  → 예: 독립 버전·독립 릴리스가 필요한가?
      → 예: packages/ 로 승격
      → 아니오: apps/control-center 내부 유지 + 경로 alias
  → 아니오: 승격하지 않음
```

## 4. 제안 기술 스택 (미확정)

| 항목 | 선택 | 근거 |
|---|---|---|
| 언어 | TypeScript (strict) | 계약 기반 타입 사용 |
| 프레임워크 후보 | Expo + React Native + React Native Web | Web 우선 구현 후 iOS/Android 재사용 목표; `WEB-OQ-007` 검증 전 확정 아님 |
| 라우팅 후보 | Expo Router | 승인된 route만 파일로 만들며 [03-information-architecture.md](03-information-architecture.md)의 reserved candidate 전체를 선생성하지 않음 |
| Server state | query/cache 계층 (TanStack Query 계열) | [08-api-and-state-architecture.md](08-api-and-state-architecture.md) §2.3 |
| 스타일 | token 기반 스타일 계층 | [04-design-tokens.md](04-design-tokens.md) |
| 그래프 | 자체 구현 (SVG 기반) | 시안의 병렬 그룹·점선 분기·상태 노드를 범용 라이브러리로 재현하기 어렵다 |

Expo/RNW 후보가 `WEB-OQ-007` 검증을 통과한 경우 React Native primitive(`View`, `Text`, `Pressable`, `ScrollView`, `FlatList`, `TextInput`)와 platform file convention을 사용한다. 결정 전에는 이를 production 구현 규칙으로 적용하지 않는다.

### 4.1 스택 리스크 (숨기지 않는다)

시안은 정보 밀도가 높은 Desktop 운영 콘솔이다. React Native Web으로 재현할 때 다음이 실제 난점이다.

- 다열 테이블의 열 정렬·행 hover·헤더 고정
- hover / focus-visible 등 포인터 상태 표현
- 대량 목록 가상화의 Web/Native 동작 차이
- SVG 그래프의 텍스트 배치·측정
- 키보드 내비게이션과 focus trap

대응: Phase 1에서 **Desktop shell과 Run Queue 테이블, Workflow 그래프를 먼저 시험 구현**해 스택 적합성을 검증한다. 여기서 시안 재현이 불가능하다고 판단되면 스택 결정을 재검토한다([17-open-decisions.md](17-open-decisions.md) `WEB-OQ-007`). 시안을 낮추는 방향으로 타협하지 않는다.

## 5. Rust workspace 영향

- `apps/` 도입은 `Cargo.toml` members에 영향을 주지 않는다.
- Node 산출물(`node_modules/`, 빌드 output, Expo 캐시)을 `.gitignore`에 추가한다. 같은 커밋에서 처리한다.
- Rust CI는 Frontend 변경으로 실패하지 않아야 한다. Frontend 검증은 별도 스크립트 범주로 둔다([15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) §5).

## 6. 검증

- 실제 트리와 이 문서의 §2가 일치하는지 확인
- kebab-case 위반 파일 확인
- 생성물/손수 작성 파일 혼재 확인
- `.gitignore` 누락으로 Node 산출물이 커밋되지 않았는지 확인

## 관련 문서

- [09-type-contract.md](09-type-contract.md) — 생성물 위치
- [14-implementation-phases.md](14-implementation-phases.md) — 구현 순서
- [17-open-decisions.md](17-open-decisions.md) — 스택 재검토 조건

---
title: "Web UI 범위와 Canonical Ownership"
document_id: "DXB-WEB-001"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-000", "DXB-BASE-000", "DXB-IFC-040"]
---
# Web UI 범위와 Canonical Ownership

## 목적

Web UI가 **무엇을 소유하고 무엇을 소유하지 않는지**를 고정한다. 이 경계는 [AGENTS.md](../../AGENTS.md) §1의 비협상 원칙 3·6과 [architecture/canonical-ownership.md](../agent/architecture/canonical-ownership.md)를 Web 계층에 적용한 결과다.

## 0. 활성 P0 baseline과의 관계 — Web은 P0 비범위다

활성 plan의 최상위 기준선 [`DXB-BASE-000`](../plan/20260823-1310-v0-8-10-detailed-development-plan/00-governance/00-normative-baseline.md) §3은 Active P0 Scope를 정의하면서 다음을 **명시적으로 비범위**로 선언한다.

```text
TUI, Web, BFF, remote multi-tenant transport, distributed HA,
generic Workflow/RPC/IDL framework, Plugin ecosystem 전체 CLI
```

따라서 이 문서 패키지의 위치는 다음과 같다.

1. Web Control Center는 **활성 P0 범위 밖의 후속 표면**이다. 이 패키지의 `priority`는 `web-P0`처럼 Web 내부 우선순위를 뜻하며, plan의 P0 Gate와 동일한 층위가 아니다.
2. 이 패키지는 **Web 표현 계층에 대해서만 normative**하다. Domain·Contract·Runtime 의미를 정의하거나 override하지 않는다.
3. Web UI 구현이 Domain·Contract·Runtime 변경을 요구하면 그 변경은 Web 문서가 결정할 수 없다. 활성 plan의 Owner 문서로 올려 plan 범위 결정을 받는다.
4. 특히 브라우저용 HTTP gateway는 baseline이 비범위로 지목한 **BFF/Web transport에 해당**한다. 따라서 gateway 도입은 Frontend 결정이 아니라 plan 범위 결정이다([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §1, [17-open-decisions.md](17-open-decisions.md) `WEB-OQ-005`).
5. Web UI 작업이 P0 milestone Gate(`DXB-DEL-060`)의 PASS 조건을 대체하거나 완화하지 않는다.

이 문서 패키지가 존재하는 이유는 P0 범위를 넓히려는 것이 아니라, 시안이 확정된 표현 계층의 설계를 **P0 진행과 충돌하지 않는 형태로 미리 고정**하는 것이다.

## 1. 목표

- `sample/*.png`와 시각적으로 동일한 Web 구현 ([00-visual-source-of-truth.md](00-visual-source-of-truth.md))
- Dark / Light 완전 지원 ([07-theme-and-color-mode.md](07-theme-and-color-mode.md))
- Desktop / Tablet / Mobile 대응 ([06-responsive-layout.md](06-responsive-layout.md))
- Web 구현물을 iOS / Android로 확장할 때 UI·상태 모델 재사용
- Rust Runtime이 상태·정책의 Canonical Owner로 유지되는 구조
- 로딩·빈 상태·부분 실패·오프라인·재연결까지 포함한 제품 수준 완성도

## 2. 비범위

- 새 Domain 개념, 새 상태기계, 새 권한 규칙의 정의
- Runtime이 판정해야 하는 실행 가능 여부·정책·승인 판단을 Frontend에서 재현하는 것
- Web UI가 자체 저장소를 두고 Domain 상태를 영구 보관하는 것
- 시안에 없는 화면의 시각 규범 확정 ([02-screen-inventory.md](02-screen-inventory.md) §4)
- CLI/TUI 표면 변경 (활성 plan의 `DXB-IFC-041`, `DXB-IFC-042`, `DXB-IFC-043`이 소유)

## 3. Canonical Ownership 경계

```text
Rust Domain / Runtime         ← 상태·정책·권한의 Canonical Owner
        ↓
Application Contract          ← DXB-IFC-040 (command/query/subscription 계약)
        ↓
Web transport projection      ← HTTP/SSE gateway (선행 조건, 08 문서)
        ↓
Frontend query / event layer  ← 캐시와 idempotent 이벤트 적용
        ↓
View Model                    ← 표현 전용 변환
        ↓
Screen / Component            ← 시안 재현
```

Frontend가 **소유하지 않는** 것:

| 상태 | Canonical Owner |
|---|---|
| Bot 목록·식별·수명 | Rust Domain (`DXB-DOM-020`) |
| Brain / Core 의미 | Rust Domain (`DXB-DOM-021`, `DXB-DOM-024`) |
| Task / Execution 상태 | Rust Domain (`DXB-DOM-023`) |
| Durable Process 진행 | Rust Runtime (`DXB-RUN-038`) |
| Conversation / Thread / Message | Rust Domain (`DXB-DOM-027`) |
| Memory assertion과 scope | Rust Domain (`DXB-DOM-022`) |
| Project / Channel scope와 membership | Rust Domain (`DXB-DOM-028`, `DXB-DOM-029`) |
| Provider / Capability health | Rust Provider Host (`DXB-ARC-013`; `DXB-ARC-019`와 `DXB-PRV-001`은 공통 인프라/adapter 구현) |
| 자원·비용·admission 상태 | Rust Runtime Resource Governor (`DXB-RUN-031`) |
| Event / audit / metric 노출 형태 | Rust Runtime (`DXB-RUN-034`) |
| 권한·승인·실행 가능 여부 | Rust Runtime (`DXB-RUN-032`, `DXB-DOM-026`, `DXB-IFC-040`) |
| Operation Receipt와 exit 의미 | `DXB-IFC-040` |
| 제품 불변조건 | `DXB-BASE-000` §2 → [18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) |

Frontend가 **소유하는** 것:

- 시각 토큰과 컴포넌트 ([04-design-tokens.md](04-design-tokens.md), [05-component-inventory.md](05-component-inventory.md))
- theme preference ([07-theme-and-color-mode.md](07-theme-and-color-mode.md))
- 화면 내 UI 전용 상태(열림/선택/입력 draft/로컬 필터) ([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §2)
- Runtime 상태 → 표현으로의 매핑 규칙(상태 라벨/색/아이콘)

## 4. 변경 분류와 품질 등급

[AGENTS.md](../../AGENTS.md) §3에 따라 Web UI 작업은 다음으로 분류한다.

| 작업 | 분류 | Tier |
|---|---|---|
| 토큰·컴포넌트·화면 구현 | Interface | B |
| View Model 매핑, 에러 표현 | Interface | B |
| HTTP/SSE gateway 신설·인증·노출 정책 | Interface + security | **A** (+ plan 범위 결정, §0.4) |
| Application Contract 의미 변경이 필요한 경우 | Core / Contract | **A** |
| 새 Domain 상태·정책 요구 발견 | Core Domain | **A** |
| 불변조건과 충돌하는 표현 요구 | Core Domain | **A** ([18-domain-invariant-compliance.md](18-domain-invariant-compliance.md)) |

Tier B 작업 중 Runtime이 제공하지 않는 정보가 필요해지면 Frontend에서 추론으로 메우지 않는다. Tier A로 승격해 계약을 먼저 정의한다.

## 5. 금지 사항

- 화면 파일에서 hex 색상, 픽셀 spacing, radius 리터럴 선언
- 같은 의미의 상태 라벨/색을 여러 화면에서 각각 정의
- Domain 상태를 Frontend global store에 재정규화해 영구 보관
- Runtime이 거부할 동작을 Frontend에서 미리 성공으로 표시(낙관적 커밋)
- Provider·Runtime 실패를 UI에서 무음 처리
- `variant` 난립으로 의미가 다른 컴포넌트를 하나로 합치기

## 6. 검증

- 컴포넌트에 hardcoded 스타일 리터럴이 없는지 lint/grep으로 확인
- View Model이 Domain 판정을 재구현하지 않는지 코드 리뷰에서 확인
- Ownership 위반은 [16-review-and-definition-of-done.md](16-review-and-definition-of-done.md) Review 1 항목이다

## 관련 문서

- [08-api-and-state-architecture.md](08-api-and-state-architecture.md) — 계층 구현과 gateway 선행 조건
- [09-type-contract.md](09-type-contract.md) — 타입 생성 경로
- [13-frontend-repository-layout.md](13-frontend-repository-layout.md) — 저장소 배치

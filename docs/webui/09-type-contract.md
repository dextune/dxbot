---
title: "Web UI Type Contract"
document_id: "DXB-WEB-009"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-30"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-008", "DXB-IFC-040"]
---
# Web UI Type Contract

## 목적

Rust Domain/Contract 타입이 TypeScript에 도달하는 경로와 금지 사항을 소유한다.

## 1. 원칙

Rust 타입을 TypeScript에서 **손으로 다시 쓰지 않는다.** 수동 재작성은 두 번째 Canonical Owner를 만드는 행위다([AGENTS.md](../../AGENTS.md) §1.3).

우선순위:

1. Rust가 소유한 canonical schema
2. schema로부터 생성한 TypeScript 타입
3. 생성된 타입을 사용하는 API client
4. client를 사용하는 feature query

```text
Rust Application Contract
      ↓ generate
TypeScript 타입
      ↓
API client
      ↓
feature query / view model
      ↓
screen / component
```

## 2. 현재 저장소의 근거 artifact와 한계

- `crates/application-contract/golden/contract-snapshot-v1.json` — 63개 operation의 command key / kind / input·target·output schema **이름** / typed field metadata / wait 정책 / security class 스냅샷
- `crates/application-contract/src/` — contract, command, invocation, execution, registry 정의

이 스냅샷은 operation registry와 CLI projection의 drift 검사용이지 wire DTO의 JSON Schema가 아니다. `typed_fields`에는 `ready_at`, local timeout처럼 `@local` CLI field도 포함될 수 있으며, 이 값은 `application-contract/src/execution.rs` projection에서 wire payload와 `RequestDigest`로부터 제거된다. 따라서 snapshot의 field 목록을 그대로 TypeScript request type으로 생성하면 안 된다.

현재 source에는 wire DTO용 JSON Schema exporter(`schemars`/`JsonSchema` 또는 동등한 generator)와 checked-in Web wire schema artifact가 **없다**. 이는 미확인 사항이 아니라 현재 구현 사실이다. 생성 방식과 artifact owner 결정은 [17-open-decisions.md](17-open-decisions.md) `WEB-OQ-006`에서 추적한다.

확정되기 전에는 다음을 지킨다.

- contract snapshot에서 Web request/response 타입을 직접 생성하지 않는다.
- 생성 경로가 없다고 해서 production API 타입을 손으로 만들어 굳히지 않는다.
- Phase 0~6 visual mock에 필요한 임시 shape는 `mocks/` 경계에 두고 `generated/`와 섞지 않으며, **실제 Runtime 연결의 근거로 사용하지 않는다.**
- 임시 shape에는 `WEB-OQ-006`과 "wire schema 생성 후 교체" 표시를 남긴다.

## 2.1 CLI 입력 계약을 Web 입력 계약으로 쓰지 않는다

`DXB-IFC-042`의 `in-*` metadata는 **CLI 입력 표면**을 위한 것이다. `DXB-IFC-040`은 `@local` field, file path, file descriptor, source variant가 **wire DTO에 존재하지 않는다**고 규정한다.

따라서 Web UI는 다음을 구분한다.

- 사용해야 하는 것: wire 수준 `CommandPayload` / output / error schema
- 사용하지 않아야 하는 것: CLI 전용 `@local` 옵션, CLI 렌더링 힌트, CLI journal 개념

CLI 화면 흐름을 Web에 그대로 옮기지 않는다. CLI와 Web은 같은 Application Contract를 공유하되 입력 표면과 사용자 여정은 각자 소유한다(`DXB-IFC-043`은 CLI 여정의 Owner다).

## 2.2 schema freeze 시점

`DXB-IFC-040`은 M2/M3/M4/M5 subset을 누적 PASS 후에만 freeze하고 **전체 63 operation schema freeze는 M6**이라고 규정한다.

- M6 이전에는 생성 타입이 변동한다고 가정하고, 타입 변화에 취약한 광범위 수동 매핑을 만들지 않는다.
- 생성 타입 변경이 화면을 깨는 것을 조기에 발견하도록 typecheck를 상시 검증 범주에 둔다([15-verification-and-visual-regression.md](15-verification-and-visual-regression.md) §5).

## 3. 규칙

1. 생성 파일은 사람이 수정하지 않는다. 생성 위치와 생성 명령을 문서화한다.
2. 생성 파일과 손수 작성 파일을 같은 디렉터리에 섞지 않는다.
3. enum/status는 **미래 값 확장을 가정**한다. 알 수 없는 값이 오면 런타임 예외를 던지지 않고 `unknown` 표현으로 처리한다([04-design-tokens.md](04-design-tokens.md) §5).
4. nullable/optional 필드를 화면에서 임의 기본값으로 채우지 않는다. "값 없음"은 표현으로 구분한다.
5. View Model은 표현 전용 변환만 한다. Domain 판정(권한, 실행 가능 여부, 상태 전이 유효성)을 재구현하지 않는다.
6. UI 라벨과 타입 이름을 혼동하지 않는다. 타입 이름은 Domain 어휘, 라벨은 UI 어휘다([03-information-architecture.md](03-information-architecture.md) §4).

## 4. 검증

- 생성 타입이 최신 contract snapshot과 일치하는지 확인 (drift 검사)
- 알 수 없는 enum 값 주입 테스트가 통과하는지 확인
- nullable 필드 누락 시 화면이 깨지지 않는지 확인
- 손수 작성 타입이 생성 타입과 중복 정의되지 않았는지 확인

## 관련 문서

- [08-api-and-state-architecture.md](08-api-and-state-architecture.md) — 접근 경로
- [13-frontend-repository-layout.md](13-frontend-repository-layout.md) — 생성물 배치
- [17-open-decisions.md](17-open-decisions.md) — 생성 경로 미결정

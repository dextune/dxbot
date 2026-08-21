# Documentation Rules

이 문서는 Agent가 설계 문서, 운영 문서, API 문서, 개발 규칙을 작성하거나 수정할 때 따라야 하는 범용 규칙을 정의한다.

## 1. 문서의 역할

문서는 구현의 부록이 아니라 제품 의미와 책임 경계를 유지하는 계약이다. 문서와 코드가 충돌하면 어느 쪽이 Canonical인지 확인하고, 모순을 방치하지 않는다.

문서에는 필요한 경우 다음을 명시한다.

- 목적과 범위
- 명시적 비범위
- Canonical Owner
- Lifecycle Owner
- 입력/출력과 상호작용
- 상태기계와 불변조건
- 실패/취소/복구 의미
- concurrency와 consistency
- resource/memory/cache 상한
- security/permission
- version/migration/removal 영향
- 검증 기준

## 2. Single Source of Truth

같은 의미의 규칙, default, limit, 상태 정의를 여러 Normative 문서에 복사하지 않는다.

- 하나의 문서를 Canonical Owner로 지정한다.
- 다른 문서는 링크 또는 ID로 참조한다.
- 예시를 위해 값을 반복할 경우 예시임을 명확히 하고 Canonical 정의를 가리킨다.
- Config/Policy가 값을 소유한다면 문서가 별도 숫자 원본이 되지 않게 한다.

## 3. 문서 변경 단위

의미 변경은 관련 영역을 한 변경 세트에서 함께 검토한다.

검토 대상:
- 상위 설계 원칙
- 관련 Domain 문서
- API/Wire/Event schema
- Storage/Data model
- Config/Policy
- Migration/Compatibility
- Security/Permission
- Testing/Acceptance
- Risk/Operational behavior

하나만 수정하고 나머지에 모순을 남기지 않는다.

## 4. Agent 전용 문서

Agent가 반복 작업을 수행하기 위해 별도의 장기 지침이 필요한 경우:

1. `docs/agent/` 아래에 작성한다.
2. 파일명은 lowercase kebab-case를 사용한다.
3. 루트 `AGENTS.md`에서 상대 링크로 연결한다.
4. 특정 일회성 작업의 임시 메모를 장기 규칙처럼 저장하지 않는다.
5. 기존 문서와 책임이 중복되면 새 문서를 만들지 말고 기존 문서를 확장한다.

Agent 문서는 실제 프로젝트 규범을 대체하는 숨은 문서가 되어서는 안 된다. 코드/도메인 규칙의 Canonical 문서가 따로 있다면 Agent 문서는 그 규칙을 요약하거나 작업 절차만 정의한다.

## 5. 파일 및 경로 이름

기본 규칙:

- Repository의 사용자 정의 파일/디렉터리: lowercase kebab-case
- Rust module source(`*.rs`): snake_case
- 외부 도구가 고정 이름을 요구하는 파일: 명시적 allowlist 예외

새 예외를 편의상 만들지 않는다. 대소문자만 다른 파일, 공백, invisible/confusable Unicode를 포함하는 경로는 피한다.

## 6. 문서 구조와 제목

- 한 문서는 하나의 명확한 책임을 가진다.
- 제목은 문서가 소유하는 개념을 직접 나타낸다.
- 너무 큰 문서는 책임별로 분리하되 단순히 길다는 이유로 파편화하지 않는다.
- 여러 문서에 동일한 긴 설명을 복사하지 않는다.
- 관련 문서 링크는 상대 경로를 우선한다.
- 이동/대체 시 깨진 링크를 남기지 않는다.

## 7. 용어

Glossary/Domain에서 정의된 용어가 있으면 동일 의미로 사용한다.

특히 다음처럼 의미가 다른 단어를 편의상 섞지 않는다.
- Bot / Core / Worker / Session
- Capability / Provider / Plugin
- Canonical State / Projection / Cache
- Task / Execution / Attempt
- Command / Event / Notification

새 용어는 기존 개념과 수명, 소유권, 실패 의미가 실제로 다를 때 도입한다.

## 8. Normative 표현

규범 문서는 모호한 표현보다 검증 가능한 문장을 사용한다.

좋은 규칙은 다음을 답할 수 있다.
- 누가 해야 하는가?
- 언제 적용되는가?
- 무엇이 금지되는가?
- 실패하면 어떤 상태가 되는가?
- 어떻게 자동 검증할 수 있는가?

`적절히`, `가능하면`, `필요 시` 같은 표현을 사용할 경우 판단 기준 또는 owner를 함께 명시한다.

## 9. 구현 세부의 고정

문서가 필요 이상으로 자료구조, library, 숫자 threshold를 고정하지 않게 한다.

고정해야 하는 것:
- 제품 의미
- 책임 경계
- 불변조건
- consistency/recovery semantics
- public contract
- security requirement

측정 후 결정해야 하는 것:
- 구체 자료구조
- cache size
- timeout 숫자
- batch size
- 특정 최적화

구현 선택을 고정하려면 ADR 또는 동등한 결정 기록과 근거를 남긴다.

## 10. Diagram과 예시

Diagram은 실제 소유권과 dependency 방향을 정확히 표현해야 한다. 그림과 본문이 다르면 수정한다.

예시:
- 실제 contract를 단순화할 수 있으나 본문과 모순되면 안 된다.
- 실제 secret/token/credential을 넣지 않는다.
- 특정 Provider 구현을 전체 시스템의 필수 요소처럼 표현하지 않는다.

## 11. 링크와 외부 참조

- Repository 내부 계약은 가능한 한 상대 링크를 사용한다.
- 외부 자료가 Normative 근거가 되면 확인 범위와 채택한 개념을 명확히 한다.
- 외부 문서 구조나 타입을 프로젝트 자체 계약으로 자동 승격하지 않는다.
- 깨진 링크와 존재하지 않는 문서 참조를 검수한다.

## 12. 코드와 문서의 동기화

코드 변경 시 문서 갱신이 필요한 대표 사례:
- 공개 API/schema 변경
- 상태기계 변경
- 새로운 persistent state
- Provider/Plugin lifecycle 변경
- config/default/limit 변경
- migration/rollback 의미 변경
- permission/security 경계 변경
- repository layout/naming 변경
- 새로운 failure/recovery state

문서 변경 시에도 반대로 구현과 테스트가 이미 다른 계약을 갖는지 확인한다.

## 13. 문서 검수 체크리스트

완료 전 확인:

- Canonical Owner가 하나인가?
- 기존 문서와 같은 규칙을 중복 정의하지 않았는가?
- 상태와 lifecycle 소유자가 명확한가?
- dependency 방향이 실제 구조와 맞는가?
- failure/recovery/removal이 빠지지 않았는가?
- 코드/API/schema/config/test와 모순되지 않는가?
- 상대 링크가 유효한가?
- 파일명이 규칙을 따르는가?
- 특정 구현을 불필요하게 영구 고정하지 않았는가?
- 검증 가능한 acceptance가 있는가?

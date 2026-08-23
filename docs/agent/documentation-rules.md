# Documentation Rules

이 문서는 설계 문서, 운영 문서, API 문서, Agent Guide를 작성·수정할 때 적용하는 장기 규칙의 Canonical Guide다.

## 1. 문서의 역할

문서는 구현의 부록이 아니라 제품 의미와 책임 경계를 유지하는 계약이다. 문서와 코드가 충돌하면 어느 쪽이 Canonical인지 확인하고 모순을 방치하지 않는다.

문서에는 적용 가능한 범위에서 목적/비범위, Canonical Owner, lifecycle, 입력/출력, 상태·불변조건, failure/recovery, concurrency, resource, security, compatibility, verification을 명시한다.

## 2. Single Source of Truth

같은 의미의 rule/default/limit/state를 여러 Normative 문서에 복사하지 않는다.

- 하나의 Canonical Owner를 정한다.
- 다른 문서는 상대 링크나 stable ID로 참조한다.
- Config/Policy가 값을 소유하면 문서가 별도 숫자 원본이 되지 않는다.
- 예시는 규범과 구분하고 Canonical 정의를 가리킨다.

## 3. Progressive Disclosure

Agent Guide는 다음 탐색 계층을 따른다.

`AGENTS.md → guide-index.md → category index → detailed guide → canonical plan/code/schema`

- 상위 Router는 링크와 선택 기준을 중심으로 유지한다.
- 하위 Guide의 긴 규칙을 상위 문서에 복사하지 않는다.
- 상세 Guide는 `Applies When`, `Does Not Apply When`, `Core Rules`, `Decision Rules`, `Forbidden Patterns`, `Verification`, `Related Guides`, `Canonical Contracts` 구조를 우선한다.
- 한 작업에서 필요하지 않은 category 전체를 읽도록 강제하지 않는다.

## 4. Agent Guide와 제품 규범 분리

`docs/agent/`는 장기적인 **작업 방법과 판단 기준**을 소유한다. 특정 릴리스의 상태기계, CLI surface, Provider inventory, milestone 같은 제품 의미는 활성 `docs/plan/`의 Canonical Owner가 소유한다.

Agent Guide가 버전별 제품 의미를 숨은 규범으로 복제하거나 override해서는 안 된다.

## 5. 문서 변경 단위

의미 변경 시 상위 원칙, Domain, API/Wire/Event, Storage, Config/Policy, Migration, Security, Testing/Acceptance, Risk/Operations 관계를 같은 change set에서 검토한다. 하나만 수정하고 나머지에 모순을 남기지 않는다.

## 6. 파일·경로·제목

- 사용자 정의 파일/디렉터리: lowercase kebab-case
- Rust module source: snake_case
- 외부 도구 고정 이름: 명시적 예외
- 한 문서는 하나의 명확한 책임을 가진다.
- 단순히 길다는 이유로 파편화하지 않고 독립 책임이 있을 때 분리한다.
- Repository 내부 링크는 상대 경로를 우선한다.

## 7. 용어와 표현

Glossary/Domain의 기존 용어를 재사용한다. Bot/Core/Session, Capability/Provider/Plugin, Canonical State/Projection/Cache, Task/Execution/Attempt, Command/Event/Notification처럼 수명과 소유가 다른 개념을 섞지 않는다.

규범 문장은 누가, 언제, 무엇을 해야/금지해야 하는지와 실패 의미·검증법을 답할 수 있어야 한다.

## 8. 구현 세부 고정 방지

고정해야 하는 것은 제품 의미, 책임 경계, invariant, consistency/recovery, public contract, security requirement다. 자료구조, cache size, timeout, batch size, 특정 최적화는 측정이나 ADR 근거 없이 영구 고정하지 않는다.

## 9. 링크와 이동

- 깨진 상대 링크를 남기지 않는다.
- 이동/대체 시 inbound reference를 점검한다.
- 외부 문서 구조나 타입을 프로젝트 자체 contract로 자동 승격하지 않는다.
- Guide Router와 상세 Guide 사이에 orphan 문서를 만들지 않는다.

## 10. 검수

문서 변경 후 [quality/documentation-consistency.md](quality/documentation-consistency.md)와 [quality/review-completion.md](quality/review-completion.md)를 적용한다.

# Repository and Git Rules

이 문서는 Agent가 파일을 추가·이동·삭제하고 Git 변경을 만들 때 따라야 하는 범용 규칙을 정의한다.

## 1. Repository 원칙

- 코드, 설계 문서, migration, 테스트는 가능한 한 같은 변경에서 관계성을 유지한다.
- 디렉터리는 책임/소유권을 반영한다.
- 비슷한 파일을 찾기 쉽도록 예측 가능한 layout을 유지한다.
- 독립 배포/소유/의존성 경계가 없는 기능을 이유 없이 별도 package/crate로 분리하지 않는다.
- 반대로 concrete Provider/Plugin처럼 dependency와 lifecycle을 분리해야 하는 기능을 Core module에 섞지 않는다.

## 2. Naming

기본:
- 사용자 정의 파일/디렉터리: lowercase kebab-case
- Rust `*.rs`: snake_case
- 외부 도구가 고정한 이름: allowlist 예외

금지:
- 대소문자만 다른 경로
- 공백 중심 naming
- invisible/confusable Unicode
- `temp`, `new`, `final2`, `misc`처럼 의미 없는 장기 이름
- 명시적 이유 없는 약어

`AGENTS.md`는 Agent 도구 호환성을 위한 고정 이름 예외다.

## 3. 새 디렉터리/파일 추가

추가 전 확인:
- 기존 소유 위치가 있는가?
- 비슷한 파일이 이미 있는가?
- 새 파일이 독립 책임을 가지는가?
- 새 공개 surface가 필요한가?
- 삭제할 때 어떤 dependency가 남는가?

Agent 전용 장기 지침은 `docs/agent/`에 두고 루트 `AGENTS.md`에서 링크한다.

## 4. Generated 파일

Generated artifact에는 source-of-truth와 생성 방법이 명확해야 한다.

- generated 결과를 수동 수정하지 않는다.
- source 변경 후 generated output을 함께 commit해야 하는지 정책을 확인한다.
- 재현되지 않는 generated diff를 그대로 받아들이지 않는다.
- 매우 큰 generated artifact를 이유 없이 repository에 저장하지 않는다.

## 5. Dependency 변경

새 dependency 추가 전:
- 기존 dependency로 해결 가능한지
- 작은 owned code가 더 단순한지
- 유지보수 상태
- security/license
- transitive graph
- compile/binary/runtime cost
- platform/MSRV 영향
을 검토한다.

Provider-specific dependency는 가능하면 해당 Provider 소유 경계에 격리한다. Domain public API에 dependency type을 불필요하게 노출하지 않는다.

## 6. Migration과 데이터 변경

Persistent data/schema를 변경할 때:
- forward migration
- restart safety
- compatibility window
- rollback 가능 여부
- irreversible step
- backup/recovery
- old fixture 검증
을 고려한다.

데이터가 없어질 수 있는 migration을 단순 refactor와 같이 취급하지 않는다.

## 7. Git 변경 안전성

- 사용자가 만든 기존 변경을 임의로 되돌리지 않는다.
- unrelated file을 정리 목적으로 함께 수정하지 않는다.
- 작업 전 현재 branch/status/diff를 가능한 범위에서 확인한다.
- 같은 파일에 다른 변경이 있으면 보존하면서 최소 diff를 만든다.
- merge conflict를 해결할 때 한쪽 전체를 덮어쓰지 않고 의미를 비교한다.

명시적 요청 없이 금지:
- force push
- hard reset
- rebase로 공유 history rewrite
- 대량 파일 삭제
- branch 보호 우회
- 기존 commit 수정/amend

## 8. Commit 단위

좋은 commit은 하나의 논리적 목적을 가진다.

- 코드와 그에 필요한 테스트/문서는 같은 commit에 포함할 수 있다.
- unrelated formatting을 기능 변경과 섞지 않는다.
- mechanical rename과 semantic change가 매우 크면 분리한다.
- 임시 debugging 코드/로그를 제거한다.
- commit 직전에 diff를 다시 읽는다.

Commit message는 저장소 관례를 따르며, 관례가 없다면 목적이 드러나는 간결한 형태를 사용한다.

예시 범주:
- `feat:` 기능
- `fix:` 결함
- `refactor:` 의미 유지 구조 변경
- `docs:` 문서
- `test:` 테스트
- `perf:` 성능
- `ci:` 자동화
- `chore:` 기타 유지보수

## 9. Branch와 PR

Branch/PR 사용 여부는 저장소 정책과 사용자 요청을 따른다.

PR 또는 리뷰 설명에는 관련되는 경우 다음을 포함한다.
- 변경 목적
- 주요 설계 결정
- API/schema/config 영향
- migration/rollback
- security/resource 영향
- 수행한 테스트
- 남은 위험 또는 미수행 검증

대규모 장기 branch보다 작은 완결 변경을 선호한다.

## 10. Secret와 로컬 데이터

commit 금지:
- API key/token/password
- private key
- 실제 credential이 든 config
- 개인 식별 가능 runtime dump
- 로컬 DB/cache/artifact
- 민감한 trace/log 원문

의심되는 경우 먼저 redaction/ignore 정책을 확인한다.

## 11. 삭제 작업

파일/기능 삭제 전:
- references/imports
- docs links
- config keys
- feature flags
- schema/data
- tests/fixtures
- CI/scripts
- Provider/Plugin registry
을 검색한다.

삭제 후 orphan reference와 silent fallback이 없어야 한다.

## 12. 최종 Repository 검수

변경 완료 후 확인:
- naming 규칙
- 새 orphan 파일/디렉터리 없음
- broken internal link 없음
- unused dependency/feature 없음
- generated drift 없음
- secret/local artifact 없음
- unrelated diff 없음
- 삭제된 기능의 stale config/schema/reference 없음

이 검수는 코드 기능 테스트를 대체하지 않으며 `testing-and-review-rules.md`의 두 차례 재검수와 함께 수행한다.

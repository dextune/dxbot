# Testing and Review Rules

이 문서는 Agent가 변경을 검증하고 완료 판정을 내릴 때 따라야 하는 범용 테스트 및 재검수 규칙을 정의한다.

## 1. 기본 원칙

테스트는 구현 세부가 아니라 계약과 불변조건을 검증한다. 외부 모델이나 네트워크의 비결정성 때문에 Core Runtime correctness 검증이 흔들리지 않게 한다.

우선순위:
1. deterministic unit/state tests
2. property/model tests
3. component/contract tests
4. Provider Host/Conformance integration
5. integration tests
6. fault/recovery/concurrency tests
7. performance/soak tests
8. 실제 외부 Provider canary/E2E

모든 변경에 모든 계층을 강제하지는 않지만 영향 범위에 맞는 검증을 선택해야 한다.

## 2. 변경 종류별 최소 검증

### Domain State 변경
- 유효/금지 전이
- terminal invariant
- expected revision/conflict
- serialization/replay
- cancellation/completion race
- crash 후 복구

### Capability Contract / Common Provider Framework 변경
- Request/Response/Error/Event semantics
- cancellation/deadline/resource/idempotency/side-effect contract
- Provider lifecycle transition
- Host permission/resource/deadline/cancel enforcement
- Conformance suite와 negative fixture
- Provider SDK compatibility/surface
- multiple provider selection/generation pin
- drain/quiescence/restart/removal
- forbidden dependency/Host bypass architecture test
- affected Provider inventory와 compatibility/migration

### Provider 변경
- Capability Conformance Suite
- 실제 Common invocation/Provider Host path 최소 1개
- unavailable/error/timeout/cancel/partial/large-output
- lifecycle start/health/drain/stop
- multiple provider selection에서 의미 유지
- forbidden dependency 없음
- Common permission/retry/resource/scheduler/persistence/telemetry 우회 없음
- provider 제거 build 또는 dependency 검사
- activity/permit/process/resource leak 없음
- Domain 의미 불변

### Plugin 변경
- manifest/config validation
- permission denied
- enable/disable
- 제공 Provider의 Host/Conformance 경로
- in-flight drain
- upgrade/rollback
- uninstall data lifecycle
- resource/registry leak
- failure isolation

### Storage/Migration 변경
- atomic commit/rollback
- crash failpoint
- old fixture load/migrate
- partial migration restart
- backup/restore 또는 equivalent recovery
- derived projection/index rebuild

### Concurrency 변경
- race winner의 명확한 의미
- cancellation/timeout 경쟁
- Provider drain/new-call, stop/late-callback
- permit/activity/lease/resource leak
- slow consumer/backpressure
- shutdown under load
- deterministic barrier/fake clock 기반 테스트

### API/Interface 변경
- schema compatibility
- error semantics
- idempotency
- authorization
- pagination/stream backpressure
- reconnect/resume
- Interface 종료가 Runtime에 영향 없는지 확인

### Documentation/Repository Rule 변경
- 상대 링크
- naming/path
- duplicate rule/Canonical Owner
- dependency graph
- Contract/Host/Runtime/Testing/Acceptance/Risk 관계
- code/docs/test 관계성

## 3. Deterministic Test 원칙

- system clock 대신 injectable/fake clock을 사용한다.
- random은 seed 또는 injectable source를 사용한다.
- 실제 모델 응답을 상태기계 acceptance의 기준으로 삼지 않는다.
- timing race를 `sleep` 길이로 맞추지 말고 barrier/event probe를 사용한다.
- 실제 process crash 의미가 중요하면 mock만으로 대체하지 않는다.
- fixture는 versioned contract의 증거로 유지한다.

## 4. Property와 Invariant

대표 invariant:
- terminal decision은 하나만 존재
- stale revision/lease/provider generation은 write 불가
- Canonical State는 cache eviction으로 손실되지 않음
- Provider 교체가 Domain Identity/Memory 의미를 바꾸지 않음
- Draining Provider가 신규 Common activity를 획득하지 못함
- duplicate delivery가 duplicate semantic effect를 만들지 않음
- disabled/removed feature가 silent fallback하지 않음
- bounded resource 상한을 넘지 않음
- Provider가 Common security/resource/retry 권한을 자체 생성하지 못함

## 5. Executable Provider Conformance

Capability별 Conformance는 Common/Testkit이 시나리오를 소유하고 Provider는 fixture/factory만 제공하는 방식을 우선한다.

검증 후보:
- normal/streaming
- malformed/unsupported/version mismatch
- timeout/cancellation
- Provider failure/partial result
- large output/backpressure/resource exhaustion
- permission denied
- Side Effect Unknown/reconciliation
- lifecycle start/degrade/drain/stop
- restart/re-registration
- multiple selection
- removal/unavailable

Conformance가 direct Provider call만 검증해 실제 Runtime Host 경로와 분리되지 않게 한다. 최소 한 계층은 `Consumer → Common invocation boundary → Provider` 전체 경로를 검증한다.

## 6. Fault와 Recovery

다음 crash window를 의도적으로 검토한다.
- commit 직전/직후
- external Side Effect 전/후
- outcome 기록 전
- waiting/checkpoint 저장 중
- provider/plugin generation swap 중
- Provider starting/draining/stopping 중
- migration 중
- shutdown/drain 중

복구는 transient in-memory state가 아니라 persisted state와 stable contract에서 판정해야 한다.

## 7. Performance와 Memory

성능 변경은 latency p50/p95/p99, throughput, allocations/op, bytes/op, retained RSS/heap, queue/channel fill, cache hit/eviction/retained bytes, Provider time과 Runtime/Host overhead를 구분해서 측정한다.

Provider Host/SDK abstraction이 불필요한 allocation/copy를 추가하는지 benchmark하되 correctness/security boundary를 없애는 최적화는 허용하지 않는다.

장시간 실행 대상은 soak에서 선형 memory/resource/activity 증가가 없는지 본다.

## 8. Security 검증

변경 영향이 있을 경우:
- secret redaction/canary
- path traversal/link escape
- permission deny/default
- revoked/stale grant/provider generation
- Plugin/Provider capability mismatch
- Provider Host bypass/ambient authority
- untrusted content가 permission을 우회하지 않는지
- audit-required action의 기록
을 검증한다.

## 9. 재검수 1 — Structural / Consistency Review

모든 수정 완료 후 별도로 수행한다.

검사:
- 파일/경로/naming
- module/crate dependency 방향
- Canonical Owner
- duplicated state/policy/default
- Capability/Provider/Provider Host/Plugin 용어
- config/schema/migration 관계
- 문서 링크와 ID
- Acceptance/Risk/Test 연결
- orphan dependency/feature/config

발견한 문제를 수정한 뒤 관련 항목을 다시 검사한다.

## 10. 재검수 2 — Cross-Layer Executability Review

1차 검수와 분리하여 실제 Runtime 흐름처럼 추적한다.

기본 추적 경로:

`Input/Command → Application → Domain → Persistence → Scheduler/Runtime → Provider Host → Provider/Plugin → Recovery → Projection → Interface`

변경에 관련된 경우 정상 실행, Provider 없음/교체/복수/제거, lifecycle drain/restart, Plugin disable/uninstall/upgrade, crash/restart, cancellation/timeout, duplicate/partial success, migration/rollback, stale config/data, resource pressure를 대입한다.

각 단계에서 상태 소유자가 없는 정보나 암묵 fallback이 생기지 않는지 확인한다.

## 11. 추가 Review 요구

위 두 Review는 Repository 범용 **최소 기준**이다. 사용자 요청, 하위 `AGENTS.md`, Normative 기획/릴리스 문서가 더 많은 review 횟수나 별도 목적을 요구하면 추가 Review를 수행한다.

추가 Review는 단순 반복 체크가 아니라 예를 들어 `Contract/Traceability`, `Security/Resource`, `Migration/Compatibility`, `Development Flow`처럼 서로 다른 검사 목적과 증거를 가져야 한다. 발견 사항을 수정한 뒤 해당 review 범위를 재확인한다.

## 12. 완료 조건

Agent는 다음이 충족되기 전 작업을 완료했다고 보고하지 않는다.

- 요청한 변경이 실제로 적용됨
- 영향받는 테스트/문서가 갱신됨
- 실행 가능한 관련 검증을 수행함
- 수행하지 못한 검증은 명확히 밝힘
- 최소 재검수 1/2 완료
- 범위별 추가 Review 요구 완료
- 모든 재검수 발견 사항을 수정하고 재확인함
- unrelated 변경을 손상시키지 않음

검증 결과는 가능하면 실행한 명령, 테스트 이름, CI 결과 또는 재현 가능한 증거로 남긴다.

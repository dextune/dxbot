# Architecture and Code Rules

이 문서는 DXBOT 코드와 아키텍처를 수정하는 Agent가 따라야 하는 범용 구현 규칙을 정의한다.

## 1. 책임과 의존성

기능은 먼저 다음 범주 중 하나로 분류한다.

| 범주 | 책임 | 변경 특성 |
|---|---|---|
| Core Domain | 제품의 지속적 의미, 불변조건, Canonical State | 작고 안정적으로 유지 |
| Optional Capability | 선택 가능한 기능의 안정된 계약 | 추가·제거 가능 |
| Provider | Capability의 실제 구현 | 교체·복수 등록 가능 |
| Interface | 사용자/외부 시스템 접근 표면 | Runtime과 독립 |
| Plugin | 외부 확장 계약을 사용하는 독립 기능 패키지 | 별도 lifecycle/permission/version |

의존성은 원칙적으로 **정책과 의미를 소유한 안쪽 계층을 향한다**. Domain이 Infrastructure 구현 세부를 알아서는 안 된다.

### 금지

- Domain → DB/HTTP/UI/Harness concrete Provider 의존
- Provider A → Provider B 직접 의존
- Interface가 DB나 내부 Runtime state를 직접 수정
- Plugin이 내부 Rust trait를 안정 ABI라고 가정
- 외부 Session/Process ID를 Domain Identity로 사용
- 편의를 위한 service locator 또는 거대한 mutable context

## 2. Canonical Owner

모든 상태와 정책은 하나의 Canonical Owner를 가진다.

새 상태를 추가하기 전에 다음을 답할 수 있어야 한다.

- 누가 생성하는가?
- 누가 변경할 수 있는가?
- 누가 영속화하는가?
- 누가 삭제/보존을 결정하는가?
- crash 후 어디에서 복구하는가?
- derived/cache/index 복사본은 어떻게 무효화되는가?

같은 사실을 DB, cache, UI local state, Provider state에 각각 원본처럼 보관하지 않는다. Projection/Index/Cache는 반드시 재생성 또는 재조회 가능한 Derived State여야 한다.

## 3. 모듈성과 제거 가능성

Optional Capability, Provider, Interface, Plugin은 **추가 계약과 제거 계약**을 함께 가져야 한다.

제거 시 검토:

1. 신규 사용 차단
2. in-flight drain 또는 안전한 termination
3. config/reference 정리
4. persisted data의 retain/migrate/export/purge/block 의미
5. cache/index/projection의 stale reference 제거
6. dependency와 feature flag 제거
7. 관련 없는 Core Domain의 compile/restore 보장
8. unsupported/removed 상태의 명시적 표면 제공

silent fallback으로 제거 문제를 숨기지 않는다.

## 4. Rust 타입 설계

- 식별자는 가능한 한 의미별 Newtype으로 분리한다.
- 상태 전이는 직접 field mutation보다 명시적 command/method를 사용한다.
- 불가능한 상태를 표현하기 어렵게 설계한다.
- 단위(bytes, tokens, duration, revision 등)는 의미를 잃지 않도록 타입화한다.
- 모호한 boolean 조합보다 의미 있는 enum/value object를 우선한다.
- 외부 Provider DTO와 Domain type을 분리한다.
- Persistence/Wire schema를 Domain struct의 단순 derive 결과로 고정하지 않는다.
- 공개 field보다 최소 공개 API를 우선한다.

## 5. Trait, Enum, Generic

Trait은 실제 교체 지점과 명시적 Port에 사용한다.

적절한 경우:
- 외부 Provider
- 비결정 요소의 테스트 대체
- 독립 구현이 실제로 존재하는 Capability

부적절한 경우:
- 닫힌 Domain 상태 집합
- 구현이 하나뿐인 단순 내부 로직
- 테스트를 위해 모든 struct를 trait로 감싸는 경우
- 이름만 비슷한 기능의 억지 공통화

Domain 내부의 닫힌 선택지는 enum을 우선한다. Generic과 dynamic dispatch는 비용 모델과 API 안정성을 보고 선택한다.

## 6. 공통화와 중복 금지

금지 패턴:

- `common`, `utils`, `helpers`, `misc`에 소유권 없는 기능 축적
- 같은 validation/policy/retry를 여러 layer에서 구현
- 동일 DTO/error code/state machine의 복사본
- 동일 Canonical data를 여러 cache가 독립 소유
- copy-paste state transition

공통화는 단순 코드 길이 감소가 아니라 **같은 의미와 같은 불변조건**을 공유할 때 수행한다. 보안/정합성 규칙은 첫 사례부터 중앙 소유자가 필요할 수 있다.

## 7. 소유권, 복사, 할당

- 호출자가 소유하고 callee는 가능한 범위에서 borrow한다.
- hot path의 불필요한 `clone`, `String`, `Vec`, serialization intermediate를 피한다.
- 장기 공유 불변 데이터는 `Arc` 등 명시적 shared ownership을 검토한다.
- 큰 payload는 참조/Artifact/streaming으로 전달한다.
- collection capacity는 실제 분포나 상한에 기반한다.
- pool/arena는 owner와 reset lifetime이 명확할 때만 사용한다.
- allocation 감소가 retained memory 증가로 이어지지 않는지 측정한다.

`clone()` 자체를 기계적으로 금지하지 않는다. 의미와 비용을 설명할 수 있어야 한다.

## 8. Queue, Channel, Cache

모든 queue/channel/cache에는 다음이 정의되어야 한다.

- owner
- maximum entries 및 필요 시 maximum bytes
- overflow/backpressure 정책
- cancellation/shutdown 처리
- cache canonical source
- cache key/version
- invalidation
- stale 허용 범위
- eviction/TTL
- metrics

unbounded channel과 무기한 누적 background queue는 금지한다.

## 9. Async와 동시성

- spawn된 모든 작업은 supervisor/owner를 가진다.
- fire-and-forget을 만들지 않는다.
- cancellation 요청과 실제 종료를 구분한다.
- shutdown은 child 작업, process, channel, buffer가 정리됐음을 확인한 뒤 완료한다.
- async lock guard를 잡은 채 장시간 I/O를 await하지 않는다.
- blocking 작업은 bounded blocking execution 경계로 격리한다.
- race는 event/revision/fencing/idempotency 등 명시적 규칙으로 해결한다.
- lock-free/atomic 최적화는 단순 mutex보다 실제 이점이 측정되고 memory ordering 근거가 있을 때만 사용한다.

## 10. 오류와 복구

오류는 문자열 하나로 축약하지 않는다.

최소 구분 후보:
- validation
- authorization
- conflict
- resource/backpressure
- transient provider
- permanent provider
- cancellation
- timeout
- corruption
- invariant violation
- security incident

retry 가능성은 오류 class, idempotency, Side Effect 여부, deadline, budget을 함께 판단한다. 외부 변경 결과가 불명확하면 자동 성공/실패를 추측하지 않고 reconciliation 상태로 보존한다.

## 11. Provider 규칙

- Consumer는 stable Capability contract만 의존한다.
- Provider registration/unregistration은 명시적 registry를 사용한다.
- 동일 Capability Provider가 여러 개면 selector/policy가 결정한다.
- "마지막 등록 승리" 같은 암묵 선택을 금지한다.
- 하나의 실행 단위 안에서는 선택된 Provider generation을 pin한다.
- Provider hot reload는 신규 실행부터 적용하는 것을 기본으로 한다.
- Provider가 없거나 제거됐을 때 명시적인 unavailable/unsupported 상태를 반환한다.
- 모든 동등 Provider는 가능한 한 같은 conformance suite를 통과해야 한다.

## 12. Plugin 규칙

Plugin은 Provider나 내부 module의 동의어가 아니다.

Plugin에는 최소한 다음 개념이 필요하다.
- stable Plugin ID/version
- API compatibility
- provided/required capabilities
- permission declaration/grant
- config schema
- lifecycle
- resource budget
- failure isolation
- data ownership/lifecycle
- upgrade/rollback/uninstall semantics

Plugin이 Core Domain store를 임의 수정하게 하지 않는다. Plugin unload가 in-flight 사용을 깨뜨리지 않도록 drain/generation pinning을 고려한다.

## 13. 보안

- 기본 권한은 최소 권한과 deny 우선으로 설계한다.
- secret 원문을 Domain/Event/Memory/로그에 저장하지 않는다.
- untrusted input이 policy나 permission 자체를 만들 수 없게 한다.
- sandbox가 불가능할 때 host unrestricted fallback을 기본값으로 두지 않는다.
- stale/revoked execution이 write하지 못하도록 revision/lease/fencing을 사용한다.
- 고위험 Side Effect는 approval/idempotency/audit 경계를 가진다.

## 14. Unsafe와 Dependency

`unsafe`는 기본적으로 피한다. 필요한 경우:
- 격리된 모듈
- 명시적 safety contract
- caller obligation
- safe wrapper
- Miri/sanitizer/fuzz 등 적절한 검증
- benchmark상 필요성
이 있어야 한다.

새 dependency는 유지보수성, 보안, 라이선스, binary/build 비용, transitive dependency를 검토한다. 작은 편의를 위해 거대한 dependency graph를 추가하지 않는다.

## 15. 성능 원칙

최적화 순서:

1. 올바른 ownership과 데이터 흐름
2. 중복 작업/직렬화 제거
3. bounded resource
4. cache locality와 batch I/O
5. profile/benchmark
6. 필요한 경우 specialized optimization

성능 개선은 correctness, 복구 가능성, 보안 경계를 약화시키면 안 된다. specialized path는 일반 path와 동일 의미임을 테스트한다.

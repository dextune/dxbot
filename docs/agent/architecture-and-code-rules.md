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

의존성은 원칙적으로 정책과 의미를 소유한 안쪽 계층을 향한다. Domain이 Infrastructure 구현 세부를 알아서는 안 된다.

### 금지

- Domain → DB/HTTP/UI/Harness concrete Provider 의존
- Provider A → Provider B 직접 의존
- Interface가 DB나 내부 Runtime state를 직접 수정
- Plugin이 내부 Rust trait를 안정 ABI라고 가정
- 외부 Session/Process ID를 Domain Identity로 사용
- 편의를 위한 service locator 또는 거대한 mutable context
- Extension 편의를 위해 Domain/Common Contract를 provider-specific shape로 변경

## 2. Canonical Owner

모든 상태와 정책은 하나의 Canonical Owner를 가진다.

새 상태를 추가하기 전에 다음을 답할 수 있어야 한다.

- 누가 생성하는가?
- 누가 변경할 수 있는가?
- 누가 영속화하는가?
- 누가 삭제/보존을 결정하는가?
- crash 후 어디에서 복구하는가?
- derived/cache/index 복사본은 어떻게 무효화되는가?

같은 사실을 DB, cache, UI local state, Provider state에 각각 원본처럼 보관하지 않는다. Projection/Index/Cache는 재생성 또는 재조회 가능한 Derived State여야 한다.

## 3. Common Framework와 Extension 책임

교체 가능한 Provider가 존재하는 경우 어려운 cross-cutting 문제는 Common Framework가 소유한다.

Common 기본 책임:
- lifecycle와 activity/quiescence 판정
- registration/immutable generation
- Provider selection
- permission/approval
- resource admission/accounting
- deadline/cancellation
- semantic retry disposition
- Side Effect idempotency/reconciliation guard
- 공통 telemetry/audit
- config/version compatibility
- recovery/restart orchestration
- Conformance/Testkit

Provider 기본 책임:
- Provider-specific typed config
- external SDK/API adapter
- Capability-specific logic
- Provider DTO ↔ stable DTO mapping
- Provider error → stable error mapping

Provider가 Common 책임을 똑같이 구현하더라도 중복 소유는 허용되지 않는다.

## 4. Provider Host / Invocation Boundary

production Consumer는 stable Capability Contract를 사용하며 concrete Provider를 직접 호출하지 않는다.

```text
Consumer → Common Provider Host → Stable Provider SPI → Provider
```

Common invocation boundary는 적용 가능한 범위에서 request validation, registry/selection, permission, resource admission, deadline/cancellation, side-effect guard, telemetry/audit, error normalization, output validation/accounting을 수행한다.

규칙:
- capability별 call site에서 같은 pipeline을 반복 구현하지 않는다.
- Host가 Domain state machine을 소유하지 않는다. 해당 owner의 Port를 호출한다.
- Provider가 Host를 우회하는 production path를 만들지 않는다.
- test-only direct SPI invocation과 actual Host-path integration test를 구분한다.
- Plugin이 제공하는 Provider도 동일 invocation boundary를 사용한다.

## 5. Minimum Provider Surface

Provider에 거대한 `RuntimeContext` 또는 범용 service registry를 전달하지 않는다.

Provider가 받을 수 있는 것은 Capability가 실제로 필요한 scoped 값/handle에 한한다. 예를 들어 effective deadline, cancellation, approved credential/transport handle, resource grant, bounded output sink, telemetry facade 등이 될 수 있다.

기본 금지:
- raw Domain Store
- Scheduler mutator
- Registry mutator
- global Secret Store
- unrestricted filesystem/network handle
- generic `get<T>()` service lookup

새 Common service가 필요해 보이면 임시 context 확장보다 Stable Contract/SDK 승격 필요성을 먼저 검토한다.

## 6. Provider Lifecycle

Provider lifecycle state와 transition 의미는 Common이 소유한다.

기본 축:
`Declared → Validated → Starting → Ready → Draining → Stopped`

오류/제한 상태:
`Degraded / Failed / Quarantined / Incompatible`

불변조건:
- Ready 전 신규 selection 금지
- Draining 후 신규 call/activity 금지
- Provider health observation과 lifecycle state 구분
- `idle`/quiesced 판정은 Common in-flight/reference 상태 기반
- stop은 child/process/channel/resource cleanup 완료 후 성립
- stale generation의 late write/callback 차단

## 7. Capability Contract Completeness

Optional Capability는 이름/trait만 정의해서는 안 된다. 적용 가능한 경우 다음 semantic contract를 가진다.

- Request / Response / Streaming Event
- Stable Error / Outcome
- Config Contract
- Capability Metadata / feature negotiation
- Cancellation / Deadline
- Resource
- Idempotency / Side Effect Classification
- Version / Compatibility
- Conformance Suite

Provider-specific optional feature 때문에 Consumer가 downcast/concrete type branch를 하게 만들지 않는다. 실제 semantic이 다르면 새 Capability 또는 version을 검토한다.

## 8. Extension Dependency Firewall

Provider 기본 허용 dependency:
- 해당 Capability Contract
- Provider SDK
- 명시적으로 공개된 Kernel/value type
- 승인된 external SDK/library

기본 금지:
- Domain internals
- Application internals
- Runtime internals
- Storage internals
- 다른 Provider
- UI/Interface internals
- private application module

Provider SDK가 금지된 내부 API를 re-export하여 firewall을 우회해서도 안 된다.

## 9. 모듈성과 제거 가능성

Optional Capability, Provider, Interface, Plugin은 추가 계약과 제거 계약을 함께 가져야 한다.

제거 시 검토:
1. 신규 사용 차단
2. in-flight drain 또는 안전한 termination
3. config/reference 정리
4. persisted data의 retain/migrate/export/purge/block 의미
5. cache/index/projection의 stale reference 제거
6. dependency와 feature flag 제거
7. 관련 없는 Core Domain compile/restore 보장
8. unsupported/removed 상태의 명시적 표면

silent fallback으로 제거 문제를 숨기지 않는다.

## 10. Rust 타입 설계

- 식별자는 가능한 한 의미별 Newtype으로 분리한다.
- 상태 전이는 직접 field mutation보다 명시적 command/method를 사용한다.
- 불가능한 상태를 표현하기 어렵게 설계한다.
- 단위(bytes, tokens, duration, revision 등)는 타입화한다.
- 모호한 boolean 조합보다 enum/value object를 우선한다.
- 외부 Provider DTO와 Domain type을 분리한다.
- Persistence/Wire schema를 Domain struct의 단순 derive 결과로 고정하지 않는다.
- 공개 field보다 최소 공개 API를 우선한다.

## 11. Trait, Enum, Generic

Trait은 실제 교체 지점과 명시적 Port에 사용한다. Domain 내부의 닫힌 선택지는 enum을 우선한다. Generic과 dynamic dispatch는 비용 모델과 API 안정성을 보고 선택한다.

Trait을 테스트 편의만을 위해 모든 struct에 씌우거나, 하나의 mega-interface로 서로 다른 Capability를 결합하지 않는다.

## 12. 공통화와 중복 금지

금지 패턴:
- `common`, `utils`, `helpers`, `misc`에 소유권 없는 기능 축적
- 같은 validation/policy/retry를 여러 layer/Provider에서 구현
- 동일 DTO/error code/state machine의 복사본
- 동일 Canonical data를 여러 cache가 독립 소유
- copy-paste state transition

공통화는 단순 코드 길이 감소가 아니라 같은 의미와 같은 불변조건을 공유할 때 수행한다.

## 13. 소유권, 복사, 할당

- 호출자가 소유하고 callee는 가능한 범위에서 borrow한다.
- hot path의 불필요한 `clone`, `String`, `Vec`, serialization intermediate를 피한다.
- 장기 공유 불변 데이터는 명시적 shared ownership을 검토한다.
- 큰 payload는 참조/Artifact/streaming으로 전달한다.
- collection capacity는 실제 분포나 상한에 기반한다.
- allocation 감소가 retained memory 증가로 이어지지 않는지 측정한다.

`clone()` 자체를 기계적으로 금지하지 않는다. 의미와 비용을 설명할 수 있어야 한다.

## 14. Queue, Channel, Cache

모든 queue/channel/cache에는 owner, maximum entries/bytes, overflow/backpressure, cancellation/shutdown, canonical source, key/version, invalidation, stale 범위, eviction/TTL, metrics가 정의되어야 한다.

unbounded channel과 무기한 누적 background queue는 금지한다.

Provider 내부 transport buffer도 이 규칙을 따른다. Provider가 Common admission을 우회해 semantic work를 별도 queue에 축적하지 않는다.

## 15. Async와 동시성

- spawn된 모든 작업은 supervisor/owner를 가진다.
- fire-and-forget을 만들지 않는다.
- cancellation 요청과 실제 종료를 구분한다.
- shutdown은 child/process/channel/buffer가 정리됐음을 확인한 뒤 완료한다.
- async lock guard를 잡은 채 장시간 I/O를 await하지 않는다.
- blocking 작업은 bounded blocking execution 경계로 격리한다.
- race는 event/revision/fencing/idempotency/generation/activity guard로 해결한다.
- lock-free/atomic 최적화는 측정과 memory-order 근거가 있을 때만 사용한다.

## 16. 오류, Retry, Side Effect

오류는 validation, authorization, conflict, resource, transient/permanent provider, cancellation, timeout, corruption, invariant, security incident 등을 구분한다.

- Provider error는 typed stable error/outcome으로 mapping한다.
- semantic retry는 error class, idempotency, Side Effect, deadline, budget을 함께 판단하는 Common 책임이다.
- Provider-local transport retry는 contract가 명시적으로 허용하고 observable semantic이 바뀌지 않는 경우에만 가능하다.
- 외부 결과가 불명확하면 `Unknown/Reconciliation Required`를 transient failure로 축약하지 않는다.
- 문자열 parsing으로 retry/security 결정을 내리지 않는다.

## 17. Provider SDK / Conformance / Reference

Provider SDK는 내부 Runtime API가 아니라 허용된 구현 surface만 공개한다.

제공 가능:
- stable contract type/re-export
- config/lifecycle/error helper
- bounded transport/output helper
- telemetry facade
- conformance/test fixture helper

금지:
- mutable Domain/Runtime/Storage internals
- privileged Registry/Scheduler/Secret access

모든 동등 Provider는 가능한 한 동일 Executable Conformance Suite를 통과한다. 최소 한 suite는 actual Common invocation boundary를 사용한다. 주요 Capability에는 deterministic Reference Provider 또는 동등 reference fixture를 두고 intentionally broken fixture로 false-positive를 방지한다.

## 18. Plugin 규칙

Plugin은 Provider나 내부 module의 동의어가 아니다.

Plugin에는 stable ID/version, API compatibility, capabilities, permission, config, lifecycle, resource, isolation, data lifecycle, upgrade/rollback/uninstall semantics가 필요하다.

Plugin Host는 package/isolation boundary이고 Provider Host는 Capability invocation enforcement boundary다. Plugin Provider도 Provider SDK/Firewall/Conformance를 따른다.

## 19. 보안

- 기본 권한은 최소 권한과 deny 우선이다.
- secret 원문을 Domain/Event/Memory/log에 저장하지 않는다.
- untrusted input/Provider output이 policy나 permission 자체를 만들 수 없게 한다.
- sandbox 불가 시 unrestricted fallback을 기본값으로 두지 않는다.
- stale/revoked execution/provider generation이 write하지 못하도록 fencing을 사용한다.
- 고위험 Side Effect는 approval/idempotency/audit 경계를 가진다.

## 20. Unsafe와 Dependency

`unsafe`는 격리 모듈, safety contract, caller obligation, safe wrapper, 적절한 검증, benchmark상 필요성이 있어야 한다.

새 dependency는 유지보수성, 보안, 라이선스, binary/build 비용, transitive graph를 검토한다. Provider-specific dependency는 Provider 경계 안에 격리한다.

## 21. Quality Tier와 변경 승격

Tier A는 Domain/Capability Contract/Provider Host/Lifecycle/Security/Resource/Recovery/Conformance/SDK/Migration과 같은 Common 변경이다. Tier B는 기존 Stable Contract의 Provider/Adapter 구현이다.

Tier B 작업에서 다음이 필요해지면 Tier A로 승격한다.
- Capability input/output/error semantic 변경
- ProviderCallContext에 새 privileged service 추가
- Host pipeline 변경
- 새로운 global retry/resource/security 정책
- Conformance 의미 변경
- SDK breaking surface 변경

Provider 안에서 workaround로 숨기지 않는다.

## 22. 성능 원칙

최적화 순서:
1. 올바른 ownership과 데이터 흐름
2. 중복 작업/직렬화 제거
3. bounded resource
4. cache locality와 batch I/O
5. profile/benchmark
6. 필요한 경우 specialized optimization

성능 개선은 correctness, 복구, 보안, Host enforcement를 약화시키면 안 된다.

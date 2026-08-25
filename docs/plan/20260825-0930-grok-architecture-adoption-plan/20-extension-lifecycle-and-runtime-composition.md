---
title: "Extension lifecycle과 Runtime composition 보완 계획"
document_id: "DXB-ADP-020"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010"]
target_owners: ["runtime-host", "provider-host", "runtime-bootstrap", "runtime-audit"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Extension lifecycle과 Runtime composition 보완 계획

## 1. 판단

**분류: Close.** DXBOT의 Common Extension 가이드와 lifecycle 원칙은 충분하다. 부족한 것은 별도 Extension Framework가 아니라, 실제 runtime 조립 경로에서 원칙을 강제하는 최소 executable contract다.

Grok 재구성본에서 참고할 부분은 다음이다.

- extension ID와 dependency를 명시적으로 선언
- duplicate, missing dependency, self-dependency, cycle을 startup 전에 거부
- deterministic topological order
- partial startup 실패 시 이미 시작된 항목을 역순 종료
- 정상 stop도 역순 teardown
- teardown 실패를 나머지 종료와 분리하여 집계

## 2. 목표 구조

```text
RuntimeBuildSpec
  ├─ ExtensionDescriptor[]
  ├─ enabled capability set
  ├─ resource/security profile refs
  └─ compatibility generation
          ↓ validate
ResolvedRuntimeGraph
          ↓ start
StartingRuntime
          ↓ all mandatory ready
ReadyRuntime + published endpoint
          ↓ close admission
DrainingRuntime
          ↓ reverse teardown
StoppedRuntime
```

`RuntimeBuildSpec`와 `ResolvedRuntimeGraph`는 private type으로 시작한다. public DTO나 범용 동적 plugin schema로 승격하지 않는다.

## 3. 최소 descriptor

```text
ExtensionId
Kind: CommonCapability | ProviderAdapter | InternalService
Dependencies: [ExtensionId]
Required: bool
CompatibilityGeneration
ResourceClaim
SecurityProfileRef
StartPolicy
StopDeadline
```

다음은 descriptor에 넣지 않는다.

- Domain aggregate
- public command/schema owner
- runtime service locator handle
- arbitrary string map 기반 configuration
- extension 간 직접 concrete reference

## 4. Lifecycle contract

```text
Declared
→ Validated
→ Starting
→ Ready | Degraded
→ Draining
→ Stopped

Starting → FailedStart → RollingBack → Stopped | FailedStop
Ready/Degraded → FailedRuntime
```

### 4.1 Start

1. graph validation을 완료한다.
2. endpoint/listener publication 이전에 recovery와 dependency readiness를 완료한다.
3. dependency API는 descriptor에 선언된 항목만 주입한다.
4. start는 deadline과 cancellation을 받는다.
5. start가 resource를 획득하면 같은 scope에 teardown 등록을 강제한다.
6. mandatory extension 하나라도 ready가 아니면 runtime endpoint를 ready로 publish하지 않는다.

### 4.2 Partial failure rollback

- 시작 완료 순서의 역순으로 teardown한다.
- rollback 중 한 teardown이 실패해도 나머지를 계속한다.
- 첫 실패만 반환하지 않고 bounded summary와 audit event를 남긴다.
- secret/raw config/path payload는 diagnostic에 넣지 않는다.
- 반복 stop/drop은 side effect를 중복시키지 않는다.

### 4.3 Drain과 Stop

```text
close new admission
→ stop scheduler poll/new provider activity
→ propagate cancellation/deadline
→ drain bounded inflight
→ flush receipt/outbox/audit/checkpoint
→ stop dependents in reverse order
→ release endpoint/resource
```

stop deadline 초과 시 process kill을 즉시 일반화하지 않는다. extension별 `force_stop_supported` 여부와 resource fencing을 먼저 정의한다.

## 5. Readiness aggregation

Runtime 전체 상태는 단일 derived projection이다.

```text
RuntimeReadiness
- generation
- state
- mandatory_ready_count / mandatory_total
- degraded_extension_ids (bounded)
- reason_codes (bounded, redacted)
- observed_at
- source_watermark
```

개별 extension이 runtime endpoint의 public state를 직접 바꾸지 않는다. `runtime-host`가 readiness owner다.

## 6. Event bus 제한

Grok식 in-process event dispatcher는 유용하지만 다음 제약을 둔다.

- control flow와 durable correctness를 event bus에 의존하지 않는다.
- topic은 typed internal enum 또는 sealed type이다.
- handler 수와 payload byte를 bound한다.
- critical event는 `Result`를 반환하고, best-effort telemetry와 구분한다.
- stop 이후 callback 금지와 unsubscribe idempotency를 테스트한다.
- durable event는 기존 outbox/event owner를 사용한다.

## 7. 코드 배치 원칙

기본 배치:

```text
crates/runtime-host/src/
  composition.rs
  extension.rs
  readiness.rs
  shutdown.rs
```

실제 파일명은 구현 시 현재 모듈 구조를 검토해 결정한다. 새 crate는 금지한다. Provider-specific descriptor adapter는 `provider-host`가 구현하되 lifecycle orchestration은 `runtime-host`가 소유한다.

## 8. 구현 작업

1. 현재 runtime 구성 요소와 dependency를 inventory한다.
2. private `ExtensionDescriptor`와 graph validator를 작성한다.
3. duplicate/missing/self/cycle/deterministic-order test를 추가한다.
4. test extension으로 start/rollback/reverse-stop을 증명한다.
5. runtime-bootstrap의 endpoint publication을 all-mandatory-ready 뒤로 결박한다.
6. runtime-audit에 redacted lifecycle events를 연결한다.
7. provider-host adapter 하나를 실제 graph에 포함한다.
8. stop deadline, cancellation, teardown failure fault test를 추가한다.

## 9. Acceptance

- 같은 descriptor set은 입력 순서와 무관하게 동일 resolved order를 만든다.
- invalid graph에서 어떤 extension도 시작하지 않는다.
- N번째 start 실패 시 1..N-1이 정확히 한 번 역순 종료된다.
- endpoint는 mandatory readiness 전에 관찰되지 않는다.
- drain 이후 신규 work가 admission되지 않는다.
- stop 재호출과 drop이 중복 side effect를 만들지 않는다.
- teardown failure가 다른 teardown을 막지 않는다.
- lifecycle event가 secret, prompt, raw payload를 포함하지 않는다.

## 10. 제거 가능성

이 composition layer는 extension의 존재를 전제로 하지 않는다. 특정 extension 제거 시 descriptor와 dependency를 제거한 build가 graph validation과 전체 test를 통과해야 한다.

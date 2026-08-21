---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 목적

Bot/Brain/Core/Task/Capability/Provider/Provider Host/Plugin처럼 혼동 가능한 용어를 하나의 의미로 고정한다.

## 2. 핵심 용어

| 용어 | 정의 | 수명 | Canonical Owner |
|---|---|---|---|
| Bot | 지속 Identity·Memory·Goal·Permission·State를 가진 논리 엔티티 | 장기 | Bot Aggregate |
| Identity | Bot의 고유성·Persona·역할·계보·버전 | Bot과 동일 | Bot |
| Brain | 한 Bot의 판단·조정·Context 정책 | Bot과 동일 | Bot Runtime |
| Memory | 출처·범위·수명 정책을 가진 정규화 기억 | 임시~장기 | Memory subsystem |
| Goal | 장기간 추구하는 의도와 성공 조건 | 중장기 | Goal Aggregate |
| Task | 실행 가능한 지속 작업과 상태기계 | 완료 후 보존 | Task Aggregate |
| Execution | Task 수행의 한 시도 | 유한 | Execution Aggregate |
| Core Lease | Brain이 Execution을 수행하기 위해 취득한 일시 실행 권한 | 짧음 | Scheduler |
| Working Context | Core 전용 입력·중간 상태 | Core와 동일 | Core Runtime |
| Routine | Bot 소유의 영속 Trigger/Schedule 정의. occurrence마다 일반 Task를 생성 | 장기 | Routine Aggregate/Service |
| Task Continuation | Waiting Task를 재개하기 위한 최소 영속 상태 | Waiting 기간 | Task Aggregate |
| Side Effect Ledger | 외부 변경 Intent·Key·Outcome·Reconciliation을 기록하는 내구 상태 | 정책 기반 | Application/Execution Ledger |
| Session | UI/프로토콜 접속 View | 임시 | Interface/Harness |
| Harness Run | 모델·도구 실행 trajectory | 유한 | Harness Adapter |
| Message | Bot 간 의도를 전달하는 내구 Envelope | 완료 후 보존 | Bot Network |
| Artifact | 큰 결과물의 content reference | 정책 기반 | Artifact Store |
| Command | 상태 변경 의도 | 순간 | Application Layer |
| Domain Event | Commit된 도메인 사실 | 장기 | Event Journal |
| Projection | Event/Canonical State에서 재생성 가능한 조회 모델 | 재생성 가능 | Projection Owner |
| Optional Capability | 교체 가능한 기능의 안정된 semantic contract | 버전 기반 | Capability Definition |
| Capability Contract Package | Request/Response/Event/Error/Config/Cancel/Deadline/Resource/Idempotency/Side Effect/Version/Conformance를 묶은 완전 계약 | 버전 기반 | Capability Definition |
| Provider | Capability 계약의 구체 구현 | 배포 단위 | Provider package/crate |
| Common Framework | Provider 공통 lifecycle/security/resource/recovery/testing 등을 중앙 강제하는 DXBOT 내부 Framework | Runtime 수명 | Common architecture owners |
| Extension Zone | Stable SPI 바깥에서 Provider/Adapter가 provider-specific 로직만 구현하는 영역 | Provider 수명 | Provider package |
| Provider Host | Consumer와 Provider 사이에서 공통 정책·lifecycle·security·resource·deadline·telemetry·normalization을 집행하는 필수 호출 경계 | Runtime 수명 | Common Framework |
| Provider Lifecycle | Declared/Validated/Starting/Ready/Draining/Stopped 및 Degraded/Failed/Quarantined/Incompatible를 포함하는 공통 상태 의미 | Provider 등록 수명 | Common Lifecycle Manager |
| Provider Call Context | 하나의 Provider 호출에 필요한 최소 승인 정보와 handle만 가진 scoped context | 호출 수명 | Provider Host |
| Provider SDK | Provider 구현에 허용된 stable contract/helper/test surface | SDK 버전 수명 | Common Framework |
| Conformance Suite | Capability 의미를 모든 Provider에 동일하게 실행 검증하는 공통 테스트 framework | Contract 버전 수명 | Capability/Testkit |
| Reference Provider | 올바른 SPI 구현 패턴과 Conformance 자체를 검증하는 deterministic 기준 구현 | Contract 버전 수명 | Capability/Common team |
| Provider Scaffold | 표준 Provider package/config/mapping/error/conformance 구조를 생성하는 개발 도구 | Tool 버전 수명 | Engineering tooling |
| Interface | Runtime 접근 표현 계층 | 배포 단위 | Interface package |
| Plugin | 안정된 외부 확장 계약을 통해 독립 설치·배포되는 확장 패키지 | 설치 수명 | Plugin Runtime/Manager |
| Policy | 허용·제한·선택·예산을 결정하는 버전된 규칙 | 버전 기반 | Policy Owner |
| Quality Tier A | Domain/Common Contract/Host/Security/Recovery/Conformance 등 변경 비용이 높은 공통 설계 영역 | 변경 단위 | Common owners |
| Quality Tier B | 기존 Stable Contract를 구현하는 Provider/Adapter 영역 | 변경 단위 | Extension owner |

## 3. Capability / Provider / Host 구분

- Capability는 **무엇을 의미하는지** 정의한다.
- Provider는 **그 의미를 외부 구현으로 어떻게 수행하는지** 구현한다.
- Provider Host는 **모든 Provider 호출에 어떤 공통 제약을 반드시 적용하는지** 집행한다.
- Provider SDK는 **Provider가 사용할 수 있는 허용된 구현 표면**이다.
- Conformance Suite는 **Provider가 같은 의미를 지키는지** 검증한다.

Provider가 Host의 책임을 재구현해도 Provider Host를 대체할 수 없다.

## 4. Plugin과 Provider 구분

- Provider는 이미 정의된 Capability를 구현한다.
- Plugin은 외부 배포 단위이며 하나 이상의 Capability Provider, Hook, Command/Query extension 등을 제공할 수 있다.
- 모든 Provider가 Plugin인 것은 아니며 built-in/native Provider는 일반 crate일 수 있다.
- Plugin이 Capability Provider를 제공하면 실제 invocation은 일반 Provider와 동일하게 Provider Host를 거친다.
- Plugin Host는 package isolation/lifecycle boundary이고 Provider Host는 capability invocation enforcement boundary다.
- Plugin이 내부 Rust trait를 직접 ABI로 사용하는 것을 기본 계약으로 삼지 않는다.

## 5. Routine 의미

Routine은 별도 Bot/Core/Agent/Plugin이 아니다. 실제 실행은 Routine이 직접 Harness를 호출하는 것이 아니라 일반 Task를 생성하는 것으로 시작한다.

## 6. Waiting Continuation 의미

`Waiting`은 단순 상태 enum이 아니다. Waiting Commit 시 Task/Specification revision, wait condition, dependency/delegated child reference, checkpoint/artifact, deadline/not-before, correlation/causation, resume guard가 함께 보존되어야 한다.

## 7. Side Effect 의미

비멱등 Tool/외부 API 변경은 실행 전에 Intent와 stable key/action digest를 기록한다. `Unknown`을 Retry 가능한 실패로 축약해서는 안 된다. Provider Host는 Side Effect classification에 따라 guard가 준비되지 않은 호출을 실행 경계로 통과시키지 않는다.

## 8. Provider Lifecycle 의미

Provider 자체의 health 응답은 lifecycle state가 아니다. Common Lifecycle Manager가 config/compatibility/health/activity/administrative operation을 조합해 state를 결정한다. 특히 `idle`, `drain complete`, `quiesced`는 Common이 in-flight/reference 상태를 보고 판정한다.

## 9. 소유권 원칙

- Bot Identity 변경은 Bot Aggregate가 소유한다.
- Task 상태와 Continuation은 Task Aggregate가 소유한다.
- Scheduler는 Task 성공 의미를 소유하지 않는다.
- Memory Canonical Record는 Memory subsystem만 Commit한다.
- Capability Registry는 Provider 가용성을 소유하지만 Domain 의미를 소유하지 않는다.
- Provider Host는 cross-cutting enforcement를 소유하지만 Domain 상태기계를 소유하지 않는다.
- Provider는 provider-specific integration을 소유하지만 global retry/security/resource/lifecycle policy를 소유하지 않는다.
- Plugin Manager는 install/enable/disable/version/data lifecycle을 소유한다.

## 10. 검증 기준

- 공개 문서/API에서 Capability/Provider/Provider Host/Plugin이 혼용되지 않는다.
- Provider 제거가 Capability 계약 또는 Domain type 삭제를 요구하지 않는다.
- Provider Call Context가 Runtime service locator로 확장되지 않는다.
- Plugin이 제공하는 Provider도 동일 Host/Conformance 경로를 사용한다.

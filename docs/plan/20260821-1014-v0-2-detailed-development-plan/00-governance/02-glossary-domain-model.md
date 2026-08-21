---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 목적

Bot/Brain/Core/Task/Capability/Provider/Plugin/Routine처럼 혼동 가능한 용어를 하나의 의미로 고정한다.

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
| Optional Capability | 교체 가능한 기능의 안정 계약 | 버전 기반 | Capability Definition |
| Provider | Capability 계약의 구체 구현 | 배포 단위 | Provider package/crate |
| Interface | Runtime 접근 표현 계층 | 배포 단위 | Interface package |
| Plugin | 안정된 외부 확장 계약을 통해 독립 설치·배포되는 확장 패키지 | 설치 수명 | Plugin Runtime/Manager |
| Policy | 허용·제한·선택·예산을 결정하는 버전된 규칙 | 버전 기반 | Policy Owner |

## 3. Plugin과 Provider 구분

- Provider는 **이미 정의된 Capability를 구현**한다.
- Plugin은 **외부 배포 단위**이며 하나 이상의 Capability Provider, Hook, Command/Query extension 등을 제공할 수 있다.
- 모든 Provider가 Plugin인 것은 아니며 built-in/native Provider는 일반 crate일 수 있다.
- Plugin이 내부 Rust trait를 직접 ABI로 사용하는 것을 기본 계약으로 삼지 않는다.
- `Plugin`, `module`, `feature flag`, `Provider`를 교환 가능한 단어로 사용하지 않는다.

## 4. Routine 의미

Routine은 별도 Bot/Core/Agent/Plugin이 아니다. 구성에는 `RoutineId`, owner `BotId`, trigger/schedule, Task template/reference, enable state, overlap policy, missed-run policy, next occurrence, revision이 포함될 수 있다. 실제 실행은 Routine이 직접 Harness를 호출하는 것이 아니라 **일반 Task를 생성**하는 것으로 시작한다.

## 5. Waiting Continuation 의미

`Waiting`은 단순 상태 enum이 아니다. Waiting Commit 시 다음 재개 정보가 함께 보존되어야 한다.

- Task/Specification revision
- wait condition
- 완료/미완료 dependency 또는 delegated child reference
- checkpoint/artifact reference
- deadline/not-before
- correlation/causation
- resume idempotency/guard

Continuation은 Working Context 전체 dump가 아니라 재현 가능한 최소 상태다.

## 6. Side Effect 의미

비멱등 Tool/외부 API 변경은 실행 전에 Intent와 stable key/action digest를 기록한다. semantic state는 최소 `Prepared`, 실행 진행, `Confirmed`, `Failed`, `Unknown`, `Reconciled`를 구분할 수 있어야 한다. 정확한 구현 enum은 ADR로 결정할 수 있으나 `Unknown`을 Retry 가능한 실패로 축약해서는 안 된다.

## 7. 소유권 원칙

- Bot Identity 변경은 Bot Aggregate가 소유한다.
- Task 상태와 Continuation은 Task Aggregate가 소유한다.
- Scheduler는 Task 성공 의미를 소유하지 않는다.
- Memory Canonical Record는 Memory subsystem만 Commit한다.
- Routine은 Task를 생성할 수 있지만 생성된 Task 상태를 소유하지 않는다.
- Capability Registry는 Provider 가용성을 소유하지만 Domain 의미를 소유하지 않는다.
- Plugin Manager는 install/enable/disable/version/data lifecycle을 소유하지만 Plugin이 제공하는 Domain 사실을 임의로 변경하지 않는다.

## 8. 검증 기준

- 공개 문서/API에서 Agent/Worker/Core/Bot 용어가 혼용되지 않는다.
- Routine 생성/삭제가 Bot count나 Core count를 바꾸지 않는다.
- Provider 제거가 Capability 계약 또는 Domain type 삭제를 요구하지 않는다.
- Plugin 비활성화가 Plugin-owned resource만 차단하고 Core Domain state를 손상시키지 않는다.

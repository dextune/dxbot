---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 목적

Bot/Brain/Core/Conversation/Thread/Session/Task/Execution/Capability/Provider/Host/Plugin/Control 용어를 수명과 Canonical Owner까지 포함해 하나의 의미로 고정한다.

## 2. 핵심 용어

| 용어 | 정의 | 수명 | Canonical Owner |
|---|---|---|---|
| Bot | 지속 Identity·Memory·Goal·Permission·State를 가진 논리 엔티티 | 장기 | Bot Aggregate |
| Identity | Bot 고유성·Persona·역할·계보·버전 | Bot과 동일 | Bot |
| Brain | 한 Bot의 판단·조정·Context 정책 | Bot과 동일 | Bot Runtime |
| Main Conversation | Bot당 하나의 영속 Communication + Supervisor Control Surface | Bot 수명 | Conversation Domain |
| Thread | DXBOT 소유의 영속 작업·문맥 경계와 lineage | 장기 | Conversation/Thread Domain |
| Thread Lineage | branch/fork의 source Thread/revision 관계 | Thread 보존 기간 | Conversation/Thread Domain |
| Conversation Message | Main Conversation/Thread 안의 사용자·Bot 발화/기록 | retention 정책 | Conversation Domain |
| Interface Session | WebSocket/browser/CLI/TUI 접속 View | 임시 | Interface |
| Provider Session | 외부 Provider conversation/session/resume handle | 임시/Derived | Provider/Adapter |
| Memory | 출처·scope·수명 정책을 가진 정규화 기억 | 임시~장기 | Memory subsystem |
| Bot-global Memory | Bot 전체에서 재사용되는 Canonical Memory | 장기 | Memory subsystem |
| Thread-scoped Memory | 특정 Thread 문맥에 한정된 Canonical Memory | Thread/retention | Memory subsystem |
| Working Memory | Execution/Core 임시 상태 | Execution/Core | Core Runtime |
| Goal | 장기 의도와 성공 조건 | 중장기 | Goal Aggregate |
| Task | 실행 가능한 지속 작업/state/revision | 완료 후 보존 | Task Aggregate |
| Task Specification Revision | Task 목표/지시의 immutable revision | Task history | Task Aggregate |
| Execution | 특정 Task revision 수행의 한 attempt | 유한/감사 보존 | Execution Aggregate |
| Core Lease | Execution 수행을 위해 Scheduler가 발급한 일시 권한 | 짧음 | Scheduler |
| Control Directive | redirect/suspend/reprioritize 등 live control의 durable 지시 | 완료 후 감사 보존 | Control/Task Domain |
| Runtime Control Channel | Work Queue와 분리된 bounded out-of-band control 경로 | Runtime 수명 | Live Control Runtime |
| Suspension | operator/user control로 실행 상태를 보존하고 admission을 중단한 의미 | resume/cancel까지 | Task/Live Control |
| Routine | Bot 소유의 영속 Trigger/Schedule 정의 | 장기 | Routine Aggregate/Service |
| Task Continuation | Waiting/Suspended 재개 최소 영속 상태 | 대기/정지 기간 | Task Aggregate |
| Side Effect Ledger | 외부 변경 Intent·Key·Outcome·Reconciliation 상태 | 정책 기반 | Application/Execution Ledger |
| Harness Run | 모델·도구 실행 trajectory | 유한 | Harness Adapter |
| Bot Message | Bot 간 의도를 전달하는 durable envelope | 보존 정책 | Bot Network |
| Artifact | 큰 결과물 content reference | 정책 기반 | Artifact Store |
| Command | 상태 변경 의도 | 순간 | Application Layer |
| Domain Event | Commit된 도메인 사실 | 장기 | Event Journal |
| Projection | Canonical source에서 재생성 가능한 조회 모델 | 재생성 가능 | Projection Owner |
| Optional Capability | 교체 가능한 기능의 안정 semantic contract | 버전 기반 | Capability Definition |
| Provider | Capability의 구체 구현 | 배포/등록 수명 | Provider package |
| Provider Host | 모든 Provider invocation의 공통 enforcement 경계 | Runtime 수명 | Common Framework |
| Provider Call Context | 한 Provider call에 필요한 최소 승인 handle | 호출 수명 | Provider Host |
| Provider SDK | Provider에 허용된 stable helper/test surface | SDK version | Common Framework |
| Conformance Suite | 동등 Provider semantics 자동 검증 | Contract version | Capability/Testkit |
| Reference Provider | deterministic 기준 구현 | Contract version | Capability/Common team |
| Interface | Runtime 접근 표현 계층 | 배포 단위 | Interface package |
| Plugin | 안정된 외부 확장 계약으로 설치되는 package | 설치 수명 | Plugin Runtime/Manager |
| Policy | 허용·제한·선택·예산 versioned 규칙 | 버전 기반 | Policy Owner |
| Quality Tier A | Domain/Common Contract/Runtime invariant 변경 | 변경 단위 | Common owners |
| Quality Tier B | 기존 Stable Contract Provider/Adapter 구현 | 변경 단위 | Extension owner |

## 3. Session / Thread 분리

`Session`이라는 단어를 아래 세 의미로 혼합하지 않는다.

```text
Interface Session = client connection, Ephemeral
Provider Session  = provider-private optimization, Derived/Ephemeral
Thread            = DXBOT-owned persistent context boundary, Canonical
```

문서/API에서 단순 `Session`이 나오면 Interface Session인지 Provider Session인지 scope를 명시한다. Thread를 Session이라고 부르지 않는다.

## 4. Main Conversation / Thread / Task 구분

- Main Conversation은 Bot 자체와의 장기 Communication/Control Surface다.
- Thread는 작업 문맥과 history/memory scope의 영속 경계다.
- Task는 실행 state machine이다.
- Execution은 Task revision의 attempt다.
- Core Lease는 Execution의 일시 자원이다.

하나의 Thread에 여러 Task가 있을 수 있고 Task 완료 후 Thread가 지속한다.

## 5. Conversation History / Memory 구분

Conversation History는 “무슨 일이 있었는가”를 보존한다. Memory는 “무엇을 기억해야 하는가”를 검증·정규화해 보존한다. Conversation Message 저장을 Memory commit으로 취급하지 않는다.

## 6. Live Control 용어

- `inspect`: non-mutating query
- `report`: 최신 진행 보고 요구
- `redirect`: Task 목표/지시 revision 변경 + old Execution yield + new Execution
- `suspend`: 상태 보존 후 실행 중단
- `resume`: 보존 상태를 한 번 소비해 새 Execution
- `cancel`: 작업 폐기 방향 terminal control
- `reprioritize`: Scheduler가 읽는 priority/resource hint revision 변경
- `fork`: source state를 보존하고 독립 Thread/Task lineage 생성

`redirect`를 단순 cancel+무관한 새 Task로 축약하지 않으며 running Execution mutation으로 구현하지 않는다.

## 7. Capability / Provider / Plugin 구분

Capability는 무엇을 의미하는지, Provider는 그 의미의 구현, Provider Host는 공통 enforcement, Plugin은 외부 package lifecycle을 의미한다. Plugin이 Provider를 제공해도 invocation은 일반 Provider와 동일 Host/Conformance를 따른다.

## 8. 소유권 원칙

- Bot Identity는 Bot Aggregate
- Main Conversation/Thread/lineage/history는 Conversation Domain
- Memory Record/Promotion은 Memory subsystem
- Task state/spec revision/Continuation은 Task Aggregate
- Core Lease/fairness는 Scheduler
- Control Directive runtime delivery/safe preemption은 Live Control Runtime
- Provider lifecycle/selection/call enforcement는 Common Provider Framework
- Plugin install/enable/data lifecycle은 Plugin Manager

서로의 Canonical state를 직접 mutate하지 않는다.

## 9. 검증 기준

- 공개 문서/API에서 Interface Session/Provider Session/Thread가 혼용되지 않는다.
- Main Conversation을 Thread 0로 표현하지 않는다.
- Conversation Message와 Bot Network Message wire/storage semantic이 구분된다.
- Provider 제거가 Thread/Memory/Task type 변경을 요구하지 않는다.
- redirect/suspend/reprioritize 의미가 Control/API/Testing에서 동일하다.

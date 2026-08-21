---
title: "용어집과 도메인 모델"
document_id: "DXB-GOV-002"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# 용어집과 도메인 모델

## 1. 목적

v0.6의 Bot/Project/Channel/Thread/Memory/Task/Core/Durable Process/Security/Runtime Memory 용어를 그대로 유지하고, v0.7의 Headless Application Contract와 CLI process/bootstrap boundary에 필요한 용어만 additive하게 고정한다.

## 2. v0.7 신규 핵심 용어

| 용어 | 정의 | 수명 | Canonical Owner |
|---|---|---|---|
| Application Contract | Backend use-case를 Domain internal API 없이 노출하는 public Command/Query/Subscription 계약 | protocol/schema version 수명 | `DXB-IFC-040` |
| Command | Canonical mutation intent. expected revision, idempotency, principal/action/target 의미를 가질 수 있음 | 요청/receipt/outcome 수명 | `DXB-IFC-040` + target Domain owner |
| Query | Canonical read facade 또는 Projection을 읽는 bounded read request | 요청/response 수명 | `DXB-IFC-040`, source truth는 기존 owner |
| Subscription | durable state/event 변화를 cursor/watermark로 관찰하는 bounded observation contract | connection/resume window 수명 | `DXB-IFC-040` |
| Runtime Host | CLI와 독립적으로 Bot/Task/Process/Provider Runtime을 장기 실행하는 process/composition boundary | Runtime process 수명 | Runtime composition |
| Host Lifecycle Adapter | Runtime이 없을 때 OS service manager/launcher를 통해 process/service start/status/stop/readiness만 다루는 bootstrap adapter. Domain API가 아님 | host operation 수명 | Runtime composition/infrastructure |
| Host Lifecycle Client | CLI가 Host Lifecycle Adapter를 호출하는 narrow client. Domain/Task/Memory/Scheduler/Provider state 접근 권한 없음 | CLI invocation 수명 | Control/Infrastructure boundary |
| Control Endpoint | 실행 중 Runtime의 Application Contract를 transport로 노출하는 authenticated/bounded adapter | Runtime/endpoint 수명 | Control adapter |
| Control Client | Control Endpoint와 version negotiation/request/stream을 수행하는 Interface-neutral client boundary | client process 수명 | Control layer |
| CLI Process | `dxb` 실행 파일의 한 process instance | command 실행 수명 | `DXB-IFC-041` |
| CLI Session | 한 CLI invocation 또는 watch/follow interaction의 ephemeral local 상태 | invocation/stream 수명 | `DXB-IFC-041` |
| Protocol Version | 요청/응답/stream wire interaction compatibility를 판정하는 버전 | protocol 수명 | `DXB-IFC-040` |
| Schema Version | public DTO/field semantics compatibility를 판정하는 버전 | schema 수명 | `DXB-IFC-040` |
| Capability Discovery | 현재 Runtime이 지원하는 public feature/capability/version/degraded 상태를 read-only로 조회하는 contract | response 수명 | `DXB-IFC-040`; authority 생성 없음 |
| Event Cursor | Subscription resume/gap detection을 위한 stable observation position | retention window 수명 | `DXB-IFC-040` |

## 3. v0.6 용어 유지

Durable Process, Process Definition Version, Activity, Epistemic Kind, Assertion State, Evidence Relation, Retraction, Revalidation, Information Label, Declassification, Authorization Decision, Security Domain, ActionGrant, Collaboration Run, Terminal Reason, Runtime Memory Envelope, Memory Reservation, Safety Headroom, Memory Pressure State, Accounted/Retained Bytes, Resident RSS, Spill의 v0.6 정의를 그대로 상속한다.

v0.5/v0.4의 Bot, Brain, Project, Membership, Channel, Role, Authority, Main/Channel Conversation, Thread, Memory Scope, Shared Scope Memory, Working Context, Context Plan, SupervisorRef, Control Directive, Provider Session, Interface Session, Core Lease, Goal/Task/Execution/Continuation/Side Effect/Artifact/Capability/Provider/Plugin/Policy 정의도 유지한다.

## 4. 구분 규칙

```text
CLI Session ≠ Interface Domain Identity
CLI Process ≠ Runtime Host Process
Host Lifecycle Operation ≠ Domain Command
Host Process Handle ≠ Task/Execution Handle
Control Connection ≠ Bot/Conversation/Thread/Task/Process lifecycle
Command ≠ Query ≠ Subscription ≠ Domain Event
Subscription Stream ≠ Runtime Work Queue ≠ Control Channel
Protocol Version ≠ Schema Version ≠ Runtime Binary Version ≠ Data Schema Version
Application Contract DTO ≠ Domain Struct ≠ Persistence Row
Capability Discovery ≠ Authorization Grant
Durable Process ≠ Brain ≠ Agent ≠ Task ≠ Execution
Knowledge Memory ≠ Runtime Memory
Core Lease count ≠ Runtime Memory capacity
```

## 5. Session / Bootstrap 의미

`Interface Session`은 persistent Domain identity가 아니다. v0.7의 실제 Reference Interface에서는 CLI invocation/control connection이 종료되어도 Bot/Main Conversation/Thread/Task/Durable Process가 유지된다. Provider Session 역시 동일 identity의 소유자가 아니다.

Runtime bootstrap은 Host Lifecycle Adapter가 process/service를 기동하고 readiness를 찾는 Infrastructure 작업이다. Runtime Ready 이후 Bot/Task/Memory/Process 상태를 읽거나 변경하는 모든 작업은 Application Contract를 사용한다.

## 6. 검증 기준

- CLI/control/host lifecycle 용어가 Bot/Thread/Task/Process identity와 혼용되지 않는다.
- public Contract와 Capability/Provider Contract를 하나의 generic API로 합치지 않는다.
- Host Lifecycle Client가 Domain fallback API로 정의되지 않는다.
- Runtime/Protocol/Schema/Data 버전을 하나의 숫자로 암묵 결합하지 않는다.
- v0.6 Domain/Security/Memory/Resource 용어가 v0.7 Interface 편의를 위해 재정의되지 않는다.

---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# 초기 아키텍처 결정 기준선

## 1. 목적

v0.4까지 확정된 ADR 후보를 유지하고 v0.5 Project/Channel/Scope-Aware Memory의 구현 전 기본 결정을 추가한다. 자료구조·DB polymorphism·정확한 numeric threshold는 증거 없이 고정하지 않는다.

## 2. 기존 결정 유지

ADR-0001~ADR-0046은 유지한다. 특히 Persistent Main Conversation, Thread/Session 분리, History≠Memory, Thread→Bot explicit promotion, bounded Context, Work/Control queue 분리, immutable redirect, durable suspend/resume, cooperative preemption, Scheduler Core ownership, Provider Session non-ownership, Thread branch lineage는 비회귀 기준이다.

## 3. v0.5 신규 결정 후보

| ADR | 기본 결정 | 재검토 트리거 |
|---|---|---|
| ADR-0047 | Project는 Brain 없는 Persistent Collaboration/Knowledge/Resource Domain | Project가 자체 판단 주체여야 한다는 제품 요구 |
| ADR-0048 | Channel은 Project-scoped Persistent Multi-Bot Collaboration Space | 독립 Project-less Channel 요구가 P0 제품 요구로 확정 |
| ADR-0049 | Project Membership과 Channel Membership은 분리하며 Channel 권한은 Project ceiling을 초과하지 않음 | 별도 ACL 모델 증거 |
| ADR-0050 | Channel Role과 Runtime Authority를 분리 | 비협상 |
| ADR-0051 | Memory Scope를 Bot/Project/Channel/Thread로 일반화하고 Execution Working은 Runtime-only | 비협상 |
| ADR-0052 | Scope promotion은 in-place move가 아니라 provenance-preserving new revision/reference | 비협상 |
| ADR-0053 | Thread identity는 Bot 전용 owner pair가 아니며 Conversation Parent relation을 가짐 | 비협상 |
| ADR-0054 | Channel participant activation은 Coordinator가 bounded routing하고 실제 실행은 Bot Brain→Scheduler/Core Lease | 비협상 |
| ADR-0055 | Shared Memory는 participating Bot Memory 복제가 아니라 independent scope-owned Canonical Memory | 비협상 |
| ADR-0056 | Recall candidate/index 뒤 final current Canonical authorization을 적용 | 비협상 |
| ADR-0057 | Membership revoke는 신규 read/write/control을 차단하되 이미 생성된 immutable Context를 retroactive mutation하지 않음 | stronger termination policy가 security ADR로 채택 |
| ADR-0058 | Channel Message와 Runtime Command/Directive를 별도 사건으로 유지 | 비협상 |
| ADR-0059 | P0 Task는 authoritative SupervisorRef 1명을 기본으로 함 | multi-supervisor merge semantic이 검증됨 |

## 4. ADR로 반드시 닫을 항목

다음은 의미는 고정하지만 physical form은 ADR 대상이다.
- `MemoryScopeRef` Rust enum/DB representation/discriminant
- `ConversationParentRef` physical union/foreign-key representation
- Project/Channel lifecycle enum
- Membership lifecycle 및 grant generation
- Role/Authority binding storage
- Shared Memory promotion/publication 용어
- Memory Class별 authority/conflict matrix
- Channel Coordinator routing policy와 per-message activation cap
- Manager absence/failover
- SupervisorRef 변경 가능성
- Channel archive와 running Task 관계
- revoke와 running Execution/high-risk Side Effect 관계
- scope-aware cache invalidation

## 5. ADR 필수 증거

Context/Decision/Alternatives, Canonical Owner, invariants, compatibility/migration/removal, security/permission, resource/memory/cache, concurrency/race, recovery, API/schema, Provider independence, benchmark/model/fault evidence, rollout/rollback을 포함한다.

## 6. Dependency 원칙

상위 Architecture/Governance 문서가 후행 Runtime 구현 문서에 `depends_on`하여 cycle을 만들지 않는다. Domain owner 간 참조는 의미 관계와 문서 dependency를 구분한다.

## 7. 검증 기준

- 신규 ADR 후보가 Acceptance/Risk/Open Question 중 하나 이상에 연결된다.
- physical representation 미결정을 이유로 Canonical semantic을 구현팀이 임의 결정하지 않는다.
- v0.4 ADR 의미를 v0.5 편의 기능이 약화하지 않는다.

---
title: "Project Scope 모델"
document_id: "DXB-DOM-028"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# Project Scope 모델

## 1. 목적

v0.5 Project identity/lifecycle/membership/resource boundary를 그대로 유지하고 v0.6 Information Flow와 Durable Process relation을 **reference 수준으로만** 보강한다. Project는 Brain/Process/Memory engine이 아니다.

## 2. Canonical 의미 비회귀

`Project ≠ Bot ≠ Brain ≠ Channel ≠ Thread ≠ Session`을 유지한다. Project는 LLM을 호출하거나 판단하지 않는다.

기존 ProjectId/Revision, Membership, Policy/Resource/Artifact refs, Project Memory Scope relation, Channel refs, lifecycle/retention/audit 의미를 유지한다.

## 3. v0.6 Reference 확장

필요 시 Project는 다음 relation/index를 가질 수 있다.
- active/recent Durable Process refs
- Collaboration Run refs
- information-flow/declassification policy refs
- Project-level Runtime resource policy ref

이 relation이 Process/Memory/Resource state를 Project Aggregate에 복제하지 않는다.

## 4. Project Memory Publication

Project Memory read/write는 기존 Project authorization과 Memory owner를 모두 통과한다. v0.6에서 추가로 source label→Project target의 Information-Flow/Declassification policy를 적용한다.

Private Bot/Thread/Channel information이 source read + Project write 권한만으로 자동 publication되지 않는다.

## 5. Lifecycle / Process

Project archive는 running Durable Process/Task를 암묵 cancel하지 않는다. archive 후 신규 Project-scoped step/message/memory admission은 policy에 따라 제한하며 필요한 중지는 Process/Task/Live Control owner를 통해 명시적으로 수행한다.

## 6. Resource Boundary

Project-level quota는 `DXB-RUN-031` hierarchy의 입력/ceiling이 될 수 있지만 Project가 MemoryReservation을 발급하거나 current RSS truth를 소유하지 않는다.

## 7. 검증 기준

- 기존 AT-PROJECT-001 유지.
- AT-SEC-003에서 Project target publication이 declassification 없이 Private source를 받지 않음.
- Project archive가 child Process/Task state를 직접 terminal로 변경하지 않음.
- Bot-only path에 Project 생성 강제 0.

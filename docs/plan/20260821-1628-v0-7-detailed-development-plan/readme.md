---
title: "DXBOT 상세 개발 기획 문서 집합"
document_id: "DXB-INDEX"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000"]
---

# DXBOT 상세 개발 기획 문서 집합 v0.7

이 디렉터리는 `docs/plan/20260821-1544-v0-6-detailed-development-plan`을 Backend normative baseline으로 고도화한 Tier A 문서 패키지다. v0.6의 Persistent Runtime/Foundation을 그대로 유지하면서 v0.7의 제품 Interface 개발 범위를 **Headless Application Contract + CLI**로 제한하고, CLI를 실제 운영 가능한 Reference Interface로 완성하는 방향을 고정한다.

> Runtime 의미는 Backend가 소유하고, Application Contract는 그 의미를 안정적으로 노출하며, CLI는 그 계약을 소비할 뿐 재해석하지 않는다.

## 0. 버전 상속 / Interface Scope

v0.7은 v0.6 상세 개발기획 전체를 참조 상속한다. 동일 `document_id`의 v0.7 문서는 명시적으로 바꾼 의미만 supersede하며 나머지 Backend 계약은 계속 유효하다.

`DXB-IFC-042`과 `DXB-IFC-043`은 v0.7 active implementation baseline에서 **명시적으로 superseded**된다. 두 Interface의 구현 milestone/release gate/interface-specific Acceptance/Risk/OQ는 상속하지 않는다.

이 scope withdrawal은 Backend 기능 제거가 아니다. TUI/Web은 v0.7 Runtime Host, Application Contract, CLI, compatibility/resource/security evidence가 충분히 안정화된 뒤 **별도 후속 버전에서 재계획**한다.

## 1. v0.7 핵심 구조

```text
                 ┌─ Host Lifecycle Client ─→ service manager / launcher
                 │                         (bootstrap only)
dxb CLI ─────────┤
                 │
                 └─ Control Client
                        ↓
                 Control Endpoint
                        ↓
             Headless Application Contract
                        ↓
                  Application Layer
                        ↓
              Persistent DXBOT Runtime
```

Runtime Ready 이후의 모든 Domain use-case는 Application Contract를 통과한다. Host Lifecycle Client는 Runtime process/service start/status/stop/readiness에만 사용하며 Domain state를 읽거나 변경하지 않는다.

## 2. 비협상 경계

- v0.6 Persistent Bot/Single Brain/Dynamic Core Lease/Memory/Durable Process/Security/Resource 의미 유지
- CLI→Domain Store/Scheduler/Provider direct access 금지
- Host Lifecycle Client→Bot/Task/Memory/Scheduler/Provider state 접근 금지
- CLI Process ≠ Runtime Host Process
- CLI SIGINT/disconnect/crash ≠ Task/Process cancel
- mutation idempotency/lost-response reconciliation
- paginated/bounded Query
- cursor/watermark/gap/resync Subscription
- protocol/schema/runtime/client/data-schema version 분리
- human output와 JSON/JSONL machine output 분리
- large result/watch full materialization 금지
- Common Authorization/Resource policy client-side duplicate 금지
- v0.7 active product Interface는 CLI만

## 3. Canonical Owner

| 의미 | Owner |
|---|---|
| 기존 Backend Domain/Runtime 의미 | v0.6 Canonical Owners |
| Application use-case orchestration | Application Layer |
| public Command/Query/Subscription | `DXB-IFC-040` |
| Runtime process bootstrap | Host Lifecycle Adapter/Client |
| CLI parsing/rendering/scripting | `DXB-IFC-041` |
| Runtime Host lifecycle/composition | Runtime Host/Application composition |
| Control connection/session | Control adapter |
| Testing | `DXB-ENG-052` |
| Acceptance | `DXB-DEL-061` |

## 4. 문서 구성

- Governance: 6
- Architecture: 8
- Domains: 10
- Runtime: 9
- Interfaces: **2**
- Engineering: 5
- Delivery: 5
- Normative/plan documents: **45**
- `readme.md` + `manifest.md`: 2
- Total Markdown: **47**

## 5. v0.7 신규 Acceptance

- AT-APP-001 Headless Contract Completeness
- AT-APP-002 Contract Version Compatibility
- AT-APP-003 Command Idempotency / Lost Response
- AT-APP-004 Subscription Resume / Gap
- AT-CLI-001 CLI End-to-End Reference Operation
- AT-CLI-002 CLI Process Independence
- AT-CLI-003 Machine Output Stability
- AT-CLI-004 Bounded Large Output
- AT-CLI-005 No Client Policy Duplication
- AT-CLI-006 Runtime Recovery / Reconnect
- AT-CLI-007 Active Interface Scope

기존 v0.6 Backend Acceptance는 삭제·축소하지 않는다.

## 6. 구현 순서

```text
M0 Scope Cleanup
→ M1 Application Contract
→ M2 Runtime Host / Control Endpoint / Bootstrap
→ M3 CLI Core
→ M4 Automation / Streaming / Recovery
→ M5 CLI Hardening / Packaging
```

v0.7 Release 목표는 CLI scaffold가 아니라 **CLI operational completeness**다. 핵심 Backend use-case, scripting/machine output, streaming/recovery, security/resource/compatibility까지 M5 Gate를 통과해야 한다.

TUI/Web 구현 milestone은 v0.7 active roadmap에 없다.

## 7. 완료 정의

1. v0.6 Backend Domain/Runtime 의미 회귀 0.
2. active Interface = Headless Application Contract + CLI.
3. IFC-042/043 explicit supersession 및 active file/dependency/gate 0.
4. IFC-040이 Command/Query/Subscription/version/error/backpressure owner.
5. IFC-041이 CLI output/exit/signal/streaming owner.
6. Runtime bootstrap은 narrow Host Lifecycle Boundary만 사용하고 Domain mutation 0.
7. CLI direct Domain/Store/Scheduler/Provider dependency 0.
8. CLI exit/crash/disconnect가 Bot/Task/Process lifecycle을 변경하지 않음.
9. mutation retry/lost response duplicate semantic effect 0.
10. large output/watch가 Runtime Memory Safety를 우회하지 않음.
11. JSON/JSONL + stable exit class로 automation 가능.
12. CLI만으로 representative Backend E2E 재현.
13. Structural/Consistency + Cross-Layer Executability/Compatibility 2회 독립 재검수 PASS.

검수 이력과 inventory evidence는 `manifest.md`가 소유한다.

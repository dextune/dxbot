---
title: "DXBOT v0.5 문서 매니페스트와 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.5.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-ENG-052"]
---

# DXBOT v0.5 문서 매니페스트와 검수 기록

## 1. Baseline

- Source baseline: `docs/plan/20260821-1200-v0-4-detailed-development-plan`
- Target: `docs/plan/20260821-1409-v0-5-detailed-development-plan`
- Upgrade theme: **Project · Channel · Scope-Aware Memory · Persistent Multi-Bot Collaboration**
- Quality Tier: **Tier A — Core Domain / Common Contract**
- Compatibility: **v0.4 strict superset**

`DXB-BASE-000`의 v0.4 Normative Inheritance 규칙에 따라 v0.5에서 명시적으로 변경하지 않은 v0.4 세부 계약은 계속 규범적이다. 문장 생략이나 요약은 deprecation이 아니다.

## 2. Inventory

| Group | Count |
|---|---:|
| Governance | 6 |
| Architecture | 8 |
| Domains | 10 |
| Runtime | 8 |
| Interfaces | 4 |
| Engineering | 5 |
| Delivery | 5 |
| **Plan documents** | **46** |
| readme + manifest | 2 |
| **Total Markdown** | **48** |

### Governance
- `00-governance/00-normative-baseline.md`
- `00-governance/00-source-concept-original.md`
- `00-governance/01-document-governance.md`
- `00-governance/02-glossary-domain-model.md`
- `00-governance/03-architecture-decision-baseline.md`
- `00-governance/04-document-template.md`

### Architecture
- `10-architecture/10-system-architecture.md`
- `10-architecture/11-rust-workspace-modules.md`
- `10-architecture/12-shared-contracts-extension-model.md`
- `10-architecture/13-harness-integration.md`
- `10-architecture/14-event-state-model.md`
- `10-architecture/15-storage-data-model.md`
- `10-architecture/16-plugin-system.md`
- `10-architecture/17-extension-framework.md`

### Domains
- `20-domains/20-bot-identity-lifecycle.md`
- `20-domains/21-brain-context.md`
- `20-domains/22-memory-system.md`
- `20-domains/23-goal-task-execution.md`
- `20-domains/24-dynamic-core-scheduler.md`
- `20-domains/25-multi-bot-network.md`
- `20-domains/26-control-plane.md`
- `20-domains/27-conversation-thread-model.md`
- `20-domains/28-project-scope-model.md`
- `20-domains/29-channel-collaboration-model.md`

### Runtime
- `30-runtime/30-concurrency-consistency.md`
- `30-runtime/31-resource-governance.md`
- `30-runtime/32-security-permissions-sandbox.md`
- `30-runtime/33-resilience-recovery.md`
- `30-runtime/34-observability-audit.md`
- `30-runtime/35-configuration-deployment.md`
- `30-runtime/36-live-control-preemption.md`
- `30-runtime/37-channel-orchestration.md`

### Interfaces
- `40-interfaces/40-api-protocols.md`
- `40-interfaces/41-cli.md`
- `40-interfaces/42-tui.md`
- `40-interfaces/43-web-control-center.md`

### Engineering
- `50-engineering/50-rust-engineering-rules.md`
- `50-engineering/51-performance-memory-cache.md`
- `50-engineering/52-testing-verification.md`
- `50-engineering/53-versioning-migration.md`
- `50-engineering/54-repository-ci-release.md`

### Delivery
- `60-delivery/60-implementation-roadmap.md`
- `60-delivery/61-acceptance-traceability.md`
- `60-delivery/62-risk-register.md`
- `60-delivery/63-open-questions.md`
- `60-delivery/64-external-reference-snapshot.md`

## 3. Change Classification

### New Canonical Documents — 3
- `DXB-DOM-028` Project Scope Model
- `DXB-DOM-029` Channel Collaboration Model
- `DXB-RUN-037` Channel Runtime Orchestration

### v0.5 Upgraded Existing Documents — 33
Governance 3, Architecture 4, Domains 8, Runtime 7, Interfaces 4, Engineering 3, Delivery 4.

### Baseline-Reused Documents — 10
다음 문서는 v0.5 변경 의미와 직접 충돌하지 않아 기존 blob을 재사용한다.
- `00-governance/00-source-concept-original.md`
- `00-governance/01-document-governance.md`
- `00-governance/04-document-template.md`
- `10-architecture/12-shared-contracts-extension-model.md`
- `10-architecture/13-harness-integration.md`
- `10-architecture/16-plugin-system.md`
- `10-architecture/17-extension-framework.md`
- `50-engineering/50-rust-engineering-rules.md`
- `50-engineering/54-repository-ci-release.md`
- `60-delivery/64-external-reference-snapshot.md`

재사용은 문서 미검토를 뜻하지 않는다. Project/Channel/Scope 변경과 Canonical Owner/Provider independence가 충돌하지 않는지 관계성 검수에 포함했다.

## 4. Canonical Ownership Freeze

| Semantic | Canonical Owner |
|---|---|
| Bot Identity/Lifecycle | DXB-DOM-020 |
| Brain / Scope-Aware Context | DXB-DOM-021 |
| Memory Scope/Record/Recall/Promotion | DXB-DOM-022 |
| Task/Execution/SupervisorRef | DXB-DOM-023 |
| Core Lease/Scheduler | DXB-DOM-024 |
| Durable Bot Network | DXB-DOM-025 |
| Control Plane | DXB-DOM-026 |
| Conversation/Thread/ParentRef | DXB-DOM-027 |
| Project | DXB-DOM-028 |
| Channel | DXB-DOM-029 |
| Live Control/Preemption | DXB-RUN-036 |
| Channel routing/admission/backpressure/presence | DXB-RUN-037 |
| Common Provider Framework/Host/SPI | DXB-ARC-017 |

Project/Channel Domain은 Memory/Thread/Task/Scheduler를 직접 소유하지 않고 relation만 가진다. Runtime Orchestration은 Channel Canonical State나 Brain을 소유하지 않는다.

## 5. v0.4 Compatibility Matrix

| v0.4 Contract | v0.5 Result |
|---|---|
| Persistent Bot Identity | 보존 |
| one logical Brain semantics | 보존 |
| Main Conversation per Bot | 보존, Project 비필수 |
| Thread ≠ Task ≠ Execution ≠ Core Lease | 보존 |
| Interface Session ≠ Provider Session ≠ Thread | 보존 |
| Conversation History ≠ Memory | 보존 |
| Bot-global Memory | `Bot(BotId)` Scope로 의미 보존 |
| Thread Memory | same ThreadId의 `Thread(ThreadId)` Scope로 일반화 |
| Thread→Bot promotion | Generic Promotion 특수 사례로 보존 |
| immutable Execution | 보존 + selected Scope revisions pin |
| Scheduler-owned Core Lease | 보존 |
| Live Control/Directive | 보존 + Channel Authority/Supervisor fencing |
| Provider Session optional optimization | 보존 |
| Stable Capability/Provider Host/SPI | 보존 |
| Side Effect write-ahead/recovery | 보존 |
| bounded queue/cache | 보존 + Channel fan-out/retrieval 확장 |
| existing Acceptance/Risk/OQ | Normative baseline 상속, 삭제·축소 없음 |

## 6. Review 1 — Structural / Canonical Ownership

### 범위
- 실제 pending Git tree inventory
- lowercase kebab-case file/directory naming
- 신규/기존 document ID 배치
- Canonical Owner duplication
- dependency direction/cycle
- Project/Channel/Memory/Thread/Orchestration responsibility boundary

### 결과
- pre-manifest tree에서 **46 plan docs + readme = 47 Markdown** 확인.
- manifest 추가 후 target inventory **48 Markdown**으로 확정.
- 신규 IDs `DXB-DOM-028`, `DXB-DOM-029`, `DXB-RUN-037` 충돌 없음.
- Project semantic=`DOM-028`, Channel semantic=`DOM-029`, Memory scope=`DOM-022`, Thread=`DOM-027`, Channel Runtime=`RUN-037`로 owner 분리.
- Scope를 Project→Channel→Thread→Bot의 저장 상속 계층으로 취급하지 않고 독립 Scope Graph로 유지.
- Domain→Provider/UI 신규 dependency 없음.
- `depends_on` 관계 검토에서 cycle 없음.
- naming 위반 없음.

**Review 1: PASS**

## 7. Review 2 — v0.4 Compatibility / Contract Traceability

### 추적 경로
```text
Bot
→ Main Conversation
→ Thread
→ Memory / Context Plan
→ Task / Execution
→ Scheduler / Core Lease
→ Provider Host
→ Recovery / API / Acceptance
```

### 발견
v0.5의 일부 고도화 문서는 변경점 중심으로 압축되어 있어, 독자가 문장 생략을 기존 v0.4 상세 계약의 deprecation으로 오해할 여지가 있었다. 이는 strict-superset 원칙과 충돌할 수 있다.

### 수정
- `DXB-BASE-000`에 **v0.4 전체 Normative Inheritance** 규칙 추가.
- 동일 `document_id`의 v0.5 문서는 명시적 변경만 supersede하고 미변경 v0.4 상세 계약은 계속 유효하도록 고정.
- omission≠deprecation을 명시.
- `readme.md`에 동일 버전 상속 규칙을 사용자 진입점으로 노출.
- 기존 Acceptance/Risk/OQ 상세는 v0.4 baseline을 상속하고 v0.5 신규 항목만 추가하는 SSOT 방식으로 정리.

### 호환성 확인
- 기존 Bot에 Project 생성 강제 없음.
- ThreadId/MemoryId/revision/provenance 재발급·재생성 없음.
- Bot Global Memory semantic 유지.
- Provider Session을 migration/Thread identity로 사용하지 않음.
- Main Conversation을 Channel/Thread 0로 치환하지 않음.
- v0.4 Thread→Bot promotion 유지.

**Review 2: PASS after correction**

## 8. Review 3 — Cross-Layer Executability / Security / Resource

### E2E
```text
User
→ Project/Channel authorization
→ Coordinator bounded routing
→ Manager Bot Brain
→ Scope-Aware immutable Context
→ durable Task delegation A/B
→ independent Scheduler/Core Leases
→ parallel results
→ Reviewer
→ MemoryProposal
→ Thread→Channel→Project promotion
→ final synthesis
```

동시에 다음 fault/race를 적용해 문서 관계를 대조했다.
- Researcher membership revoke
- Authority downgrade
- runtime crash/restart
- Provider Session loss
- stale semantic index/cache
- duplicate Manager redirect
- Channel fan-out pressure
- long history/shared Memory
- promotion conflict/source update
- v0.4 Bot-only Thread 병행

### 확인
- candidate/index 뒤 canonical fetch + **FINAL current authorization** 존재.
- routing snapshot 뒤 dispatch/control/write 전에 current Membership/Authority generation 재검증.
- immutable Context를 revoke 때문에 retroactive mutation하지 않음.
- high-risk Side Effect는 policy에 따라 execution 직전 current authorization 재검증 가능.
- Coordinator는 LLM reasoning/Brain/Task/Memory/Core owner가 아님.
- Manager는 durable Bot Network/Task/Directive를 사용하고 다른 Bot Brain/Core/Memory를 직접 조작하지 않음.
- Channel activation/fan-out/response/retrieval/subscriber/cache 모두 bounded.
- Project/Channel state 증가를 전체 heap materialization으로 처리하지 않음.
- cache/index/presence/summary는 rebuildable Derived state.
- restart 시 Project/Channel/Membership/Scoped Memory/Task/Directive를 Canonical State에서 복구하고 Provider Session은 optional optimization으로 유지.
- v0.4 Bot-only path와 동시에 성립.

추가 교정 필요 사항 없음.

**Review 3: PASS**

## 9. Acceptance / Risk / OQ Inventory

### 신규 Acceptance — 10
- AT-PROJECT-001
- AT-CHANNEL-001
- AT-CHANNEL-002
- AT-CHANNEL-003
- AT-MEM-005
- AT-MEM-006
- AT-CTX-003
- AT-SEC-002
- AT-COLLAB-001
- AT-MIG-001

### 신규 Risk — 11
- R-050 ~ R-060

### 신규 P0 Open Question — 20
- OQ-031 ~ OQ-050

기존 v0.4의 Acceptance/R-001~049/R-SPI-*/OQ inventory는 Normative Inheritance에 따라 유지한다.

## 10. Final Definition of Done

- [x] Project/Channel Canonical Owner 각각 하나
- [x] Memory Scope semantic은 DXB-DOM-022 단독 소유
- [x] Channel Orchestration이 Domain semantic/Brain을 복제하지 않음
- [x] Bot-only v0.4 path 유지
- [x] 기존 Acceptance 의미 유지
- [x] Thread identity/revision migration 보존
- [x] Bot Global Memory 의미 유지
- [x] Provider Session independence 유지
- [x] Scheduler Core Lease ownership 유지
- [x] Bot Network/Live Control 재사용
- [x] Shared Scope read/write current authorization
- [x] Channel fan-out/queue/cache bounded
- [x] 전체 transcript/Memory materialization 금지
- [x] cache/index/presence rebuild 가능
- [x] v0.4→v0.5 migration/rollback 계획 존재
- [x] Acceptance/Test/Risk/OQ 추적
- [x] dependency cycle 없음
- [x] lowercase kebab-case 준수
- [x] AGENTS/docs-agent 규칙과 모순 없음
- [x] 3회의 독립 재검수 통과

**최종 검수 결과: 3 / 3 PASS**

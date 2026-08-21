---
title: "DXBOT v0.6 문서 매니페스트와 검수 기록"
document_id: "DXB-MANIFEST"
version: "0.6.0"
status: "Reviewed Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-DEL-061", "DXB-DEL-062", "DXB-DEL-063", "DXB-ENG-052"]
---

# DXBOT v0.6 문서 매니페스트와 검수 기록

## 1. Baseline

- Source baseline: `docs/plan/20260821-1409-v0-5-detailed-development-plan`
- Target: `docs/plan/20260821-1544-v0-6-detailed-development-plan`
- Upgrade theme: **Durable Process + Epistemic/Secure Memory + Runtime Memory Safety + Bounded/Measurable Collaboration**
- Quality Tier: **Tier A — Core Domain / Common Contract**
- Compatibility: **v0.5 strict superset**
- Change type: **development-plan/documentation only** — 본 패키지는 구현 계약·Gate·Acceptance를 정의하며 Runtime 코드 구현 완료를 의미하지 않는다.
- Code changes in this package: **none**. 실제 구현 완료 판정은 `DXB-ENG-052` 테스트와 `DXB-DEL-061` Acceptance evidence를 통과한 구현 커밋에서 별도로 수행한다.

## 2. Inventory

| Group | Count |
|---|---:|
| Governance | 6 |
| Architecture | 8 |
| Domains | 10 |
| Runtime | 9 |
| Interfaces | 4 |
| Engineering | 5 |
| Delivery | 5 |
| **Plan documents** | **47** |
| readme + manifest | 2 |
| **Total Markdown** | **49** |

## 3. Change Classification

### New Canonical Documents — 1
- `DXB-RUN-038` — `30-runtime/38-durable-process-orchestration.md`

### v0.6 Upgraded Existing Documents — 29
Governance 3, Architecture 4, Domains 7, Runtime 7, Interfaces 1, Engineering 3, Delivery 4.

### Baseline-Reused Plan Documents — 17
- `00-governance/00-source-concept-original.md`
- `00-governance/01-document-governance.md`
- `00-governance/04-document-template.md`
- `10-architecture/12-shared-contracts-extension-model.md`
- `10-architecture/13-harness-integration.md`
- `10-architecture/16-plugin-system.md`
- `10-architecture/17-extension-framework.md`
- `20-domains/20-bot-identity-lifecycle.md`
- `20-domains/26-control-plane.md`
- `20-domains/27-conversation-thread-model.md`
- `30-runtime/35-configuration-deployment.md`
- `40-interfaces/41-cli.md`
- `40-interfaces/42-tui.md`
- `40-interfaces/43-web-control-center.md`
- `50-engineering/50-rust-engineering-rules.md`
- `50-engineering/54-repository-ci-release.md`
- `60-delivery/64-external-reference-snapshot.md`

재사용은 미검토를 뜻하지 않으며 v0.6 owner/compatibility와 충돌하지 않는지 Review 1/2에 포함했다.

## 4. Canonical Ownership Freeze

| Semantic | Canonical Owner |
|---|---|
| Bot Identity/Lifecycle | DXB-DOM-020 |
| Brain/Context | DXB-DOM-021 |
| Memory Scope/Epistemic/Retraction | DXB-DOM-022 |
| Task/Execution/Supervisor | DXB-DOM-023 |
| Core Lease/Scheduler | DXB-DOM-024 |
| Durable Bot Network | DXB-DOM-025 |
| Project | DXB-DOM-028 |
| Channel | DXB-DOM-029 |
| Concurrency/Fencing | DXB-RUN-030 |
| Resource/Runtime Memory | DXB-RUN-031 |
| Authorization/Information Flow | DXB-RUN-032 |
| Recovery | DXB-RUN-033 |
| Observability | DXB-RUN-034 |
| Live Control | DXB-RUN-036 |
| Channel Routing/Termination | DXB-RUN-037 |
| Durable Cross-Aggregate Process | DXB-RUN-038 |
| Performance/Memory Benchmark | DXB-ENG-051 |
| Testing | DXB-ENG-052 |
| Acceptance | DXB-DEL-061 |

## 5. v0.5 Compatibility Matrix

- Persistent Bot/Single Brain: **PASS — 보존**
- Main Conversation/Thread identity: **PASS — 보존**
- Project/Channel Brain 금지: **PASS — 보존**
- Scope-Aware Memory: **PASS — 보존 + epistemic lifecycle additive**
- Shared Memory independent scope: **PASS — 보존**
- immutable Execution: **PASS — 보존**
- Scheduler-owned Core Lease: **PASS — 보존**
- Live Control/Side Effect: **PASS — 보존**
- Provider Session independence/SPI: **PASS — 보존**
- Bot-only path: **PASS — 보존**
- Existing Acceptance/Risk/OQ: **PASS — baseline inheritance로 보존**

## 6. Review 1 — Structural / Consistency

**Result: PASS**

검수 범위:
- 문서/파일 inventory와 lowercase kebab-case 경로
- `document_id` / Canonical Owner 중복
- Durable Process와 Task/Channel Coordinator/Memory 역할 중복
- Authorization과 Domain permission 중복
- Resource Governance와 Scheduler/Core ownership 중복
- Event/Storage/API/Test/Acceptance/Risk/OQ 연결
- dependency cycle
- v0.5 strict-superset inheritance
- baseline-reused 17개 문서의 v0.6 owner 충돌 여부

Evidence:
- Plan documents: **47**, readme/manifest 포함 **49 Markdown**
- New Canonical Owner: **1 (`DXB-RUN-038`)**
- Canonical Owner duplicate: **0**
- Dependency cycle: **0**
- v0.5 동일 기능/ID의 암묵 제거: **0**
- Process가 child Task/Memory/Directive 상태를 복제하는 경로: **0**
- MemoryReservation을 persistent Task/Core state로 만드는 경로: **0**
- Project/Channel/Process를 Brain으로 승격하는 경로: **0**

발견/조치:
- v0.5 테스트 문서의 과거 3-review 기록과 현재 Repository/v0.6의 2-review 규칙이 혼동될 수 있어, `DXB-BASE-000`/`DXB-ENG-052`에 **과거 evidence는 보존하되 v0.6은 두 독립 Review로 통합하며 검사항목은 축소하지 않는다**고 명시했다.
- 조치 후 동일 Structural/Consistency 범위를 재확인하여 PASS.

## 7. Review 2 — Cross-Layer Executability / Compatibility

**Result: PASS**

End-to-end 추적:

```text
User request
→ Single-Bot 또는 Collaboration admission (RUN-037)
→ 필요 시 Durable Process 생성 (RUN-038)
→ Manager delegation / Researcher Task (DOM-025/023)
→ Activity/Execution / Provider Host
→ result commit 후 process advance 전 crash
→ committed outcome ref로 restart/replay, Provider 재호출 없음 (RUN-038/033)
→ Reviewer evidence validation (RUN-037 + DOM-022)
→ Claim/Evidence commit
→ Thread→Channel→Project publication (DOM-022 + RUN-032)
→ source evidence retraction
→ dependent Memory revalidation
→ Private→Shared publication attempt denied without declassification
→ ActionGrant issue/consume
→ duplicate retry/replan/delegation bounded by grant
→ Collaboration Cycle explicit terminal reason
→ many Bot/Core burst → global Runtime Memory admission (RUN-031/DOM-024)
→ Constrained/Critical/Emergency degradation
→ large Provider stream bounded/backpressured/spilled-or-cancelled by policy
→ cancel/suspend/reconnect releases reservation/task/subscriber resources
→ staged/lazy recovery avoids hot-load storm
→ RUN-034 observability / IFC-040 API / ENG-052 tests / DEL-061 Acceptance
```

검증 결과:
- duplicate Task/Activity/Side Effect semantic path: **0**
- replay-time completed LLM/Tool/Provider call: **0**
- Model/Tool output의 direct false-Verified path: **0**
- retraction dependency loss/blind delete contract: **0**
- Private→Shared authorization-only bypass: **0**
- ActionGrant over-consumption path: **0**
- terminal reason 없는 unbounded collaboration contract: **0**
- process-wide Runtime Memory budget bypass: **0**
- allocator OOM을 기다리는 정상 control path: **0**
- unbounded large-payload copy/buffer contract: **0**
- eager recovery hot-load contract: **0**
- Scheduler/Core ownership regression: **0**
- v0.5 Bot-only/Provider/Side Effect/Live Control semantic regression: **0**

발견/조치:
- 문서가 구현 완료로 오인될 가능성을 제거하기 위해 본 Manifest에 **documentation-only / code changes none / 구현 완료는 향후 Acceptance evidence로 판정**을 추가했다.
- 조치 후 동일 end-to-end 시나리오와 v0.5 compatibility matrix를 재추적하여 PASS.

## 8. Final Review Status

- Review 1 — Structural / Consistency: **PASS**
- Review 2 — Cross-Layer Executability / Compatibility: **PASS**
- 발견 사항 조치 후 동일 범위 재검수: **PASS**
- v0.5 compatibility regression found: **0**
- Canonical Owner duplication found: **0**
- Documentation package status: **Reviewed Draft — implementation-ready planning baseline**

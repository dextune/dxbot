---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.4.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-ARC-017", "DXB-DOM-027", "DXB-RUN-036", "DXB-DEL-060"]
---

# 미결정 사항과 권장 기본값

## 1. 목적

v0.4의 제품 의미는 고정하되 lifecycle enum, physical schema, Provider별 interruption 최적화처럼 구현 전에 증거가 필요한 세부를 Open Question으로 관리한다.

## 2. v0.3 기존 질문 유지

OQ-001~021, OQ-101~117과 P2 질문은 유효하다. DeepSeek optional Provider, DB/Event/ID/async/control transport, Memory delete, Sandbox, Side Effect enum, Routine policy, Provider selector/Host/SDK/Contract/Lifecycle/CallContext, fairness, vector index, Bot transport, TUI/remote API, policy DSL, Artifact, Plugin isolation/package, Memory threshold, Scaffold, Remote Provider 등의 결정은 해당 기존 Gate에서 닫는다.

## 3. v0.4 P0 Open Questions

| ID | 질문 | 권장 기본값 | Gate |
|---|---|---|---|
| **OQ-022** | Thread 정확한 lifecycle enum | create/active/archive/restore 의미만 P0 고정, enum 최소화 | M1 schema ADR |
| **OQ-023** | Thread branch/merge의 P0/P1 범위 | P0 branch+lineage+isolation, semantic merge는 P1 이후 | M1/M3 |
| **OQ-024** | Suspended Task continuation/checkpoint 표현 | Waiting과 storage primitives 일부 재사용 가능, semantic reason/state는 분리 | M2 storage ADR |
| **OQ-025** | Harness별 safe interruption point | Capability feature metadata + Host/Supervisor contract, provider-specific details Adapter에 격리 | M2 Conformance |
| **OQ-026** | Provider native steering 활용 범위 | Canonical Directive/Task revision 후 responsiveness optimization으로만 사용 | M2 Harness ADR |
| **OQ-027** | report freshness/SLA 정책 | observed_at + last committed + heartbeat + stale flag 필수, 수치는 benchmark/Provider capability 기반 | M3/M5 |
| **OQ-028** | Thread-local Memory retention | Bot-global과 별도 policy, default bounded hot/cache + durable retention class | M2 Memory ADR |
| **OQ-029** | 여러 Control Directive precedence | security/safety/terminal state 우선 + expected revision; 세부 precedence table은 model test로 확정 | M3 concurrency ADR |
| **OQ-030** | Conversation raw history compaction/archive | canonical history와 Memory 분리, cold/archive + provenance-preserving compaction 후보 | M1/M2 storage benchmark |

## 4. 추가 P1 Open Questions

| ID | 질문 | 권장 기본값 | Gate |
|---|---|---|---|
| OQ-118 | Thread semantic merge | explicit user/operation + conflict-preserving merge; automatic hidden merge 금지 | P1 |
| OQ-119 | Thread Graph large-scale query/index | adjacency/summary projection + neighborhood pagination; Canonical lineage 별도 | perf benchmark |
| OQ-120 | control coalescing | report/reprioritize 등 command-kind별 명시 semantics, cancel/redirect silent drop 금지 | M3 load |
| OQ-121 | control reserve sizing | Policy SSOT + workload benchmark, 문서 상수 금지 | M3 perf |
| OQ-122 | suspended checkpoint expiry | retention/policy + explicit expired/recovery-required, silent restart 금지 | M2/M3 |
| OQ-123 | Provider progress heartbeat normalization | Capability optional event → Host stable observation, unsupported 추측 금지 | M2/M5 |
| OQ-124 | cross-Thread reference/context import | explicit source refs + permission + bounded retrieval, mutable context share 금지 | P1 |

## 5. v0.4에서 이미 고정된 항목

다음은 Open Question이 아니다.
- Bot당 Persistent Main Conversation 하나
- Main Conversation은 Thread 0가 아님
- Interface Session / Provider Session / Thread 분리
- Thread는 Persistent Canonical Domain
- Conversation History ≠ Memory
- Thread-local Memory automatic global promotion 금지
- transcript size와 Context size 분리
- Provider Session loss independence
- Work Queue / bounded Control Channel 분리
- immutable Execution + redirect new Task revision/Execution
- cooperative preemption, Hard Real-Time 미보장
- suspend/resume crash-safe meaning
- Scheduler Core Lease ownership
- Prompt/Provider output의 Control authority 획득 금지

## 6. 결정 증거

각 질문은 use/non-use case, invariants, security/privacy, benchmark/RSS/allocation, concurrency model, fault/crash window, migration/rollback, API compatibility, Provider inventory/Conformance, prototype, owner, ADR link를 사용한다.

## 7. 검증 기준

- OQ-022~030이 해당 Milestone 전에 ADR/실험으로 닫힘.
- reversible safe default가 있어 질문 때문에 unsafe workaround를 만들지 않음.
- Tier B Provider 구현이 OQ-025/026을 이유로 Core semantic을 임의 결정하지 않음.
- exact enum/threshold/physical schema를 근거 없이 Normative 문서에 고정하지 않음.

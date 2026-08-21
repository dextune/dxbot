---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.5.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-037", "DXB-DEL-060"]
---

# 미결정 사항과 권장 기본값

## 1. 목적

v0.5의 Canonical semantic은 고정하되 enum, physical representation, numeric threshold, routing/failover algorithm처럼 증거가 필요한 세부를 ADR/benchmark 전에 임의 확정하지 않는다.

## 2. 기존 Open Question 유지

v0.4까지의 OQ-001~030, OQ-101~124 및 P2 질문은 유효하다. Thread lifecycle/merge, suspended checkpoint, safe-point, Provider steering, report freshness, Memory retention, Control precedence, history compaction 등 기존 결정 Gate를 축소하지 않는다.

## 3. v0.5 P0 Open Questions

| ID | 질문 | 권장 기본 방향 | Gate |
|---|---|---|---|
| **OQ-031** | Project lifecycle enum | create/active/archive/restore/delete 의미만 먼저 고정, enum 최소화 | M0/M1 ADR |
| **OQ-032** | Channel lifecycle enum | create/active/archive/restore 의미 고정, running Task와 분리 | M0/M1 ADR |
| **OQ-033** | Channel Membership lifecycle/generation | durable revision+generation, Presence와 분리 | M1 ADR |
| **OQ-034** | Project→Channel membership inheritance | 기본은 자동 grant 아님; Project ceiling + explicit Channel membership | M1 Security ADR |
| **OQ-035** | MemoryScopeRef physical representation | semantic enum은 고정, Rust/DB discriminant/FK는 benchmark+schema ADR | M0/M2 |
| **OQ-036** | promotion/publication/internalization 최종 용어 | source-preserving explicit transition 의미는 유지, Glossary/ADR에서 하나로 freeze | M0/M2 |
| **OQ-037** | Project/Channel Memory retention | Bot/Thread와 별도 policy class, 숫자는 workload/retention 요구 기반 | M2 |
| **OQ-038** | Shared Memory write approval | read/write/promotion 권한 분리, sensitive class는 approval 가능 | M2 Security |
| **OQ-039** | Memory Class별 conflict authority matrix | scope-depth가 아니라 class/authority/provenance/verification 기반 | M2 ADR |
| **OQ-040** | Channel Coordinator routing policy | explicit mention/manager-first/role subset 지원, pluggable policy | M3 |
| **OQ-041** | per-message maximum Bot activation | Policy SSOT + load/latency/RSS benchmark, 문서 상수 금지 | M3 Perf |
| **OQ-042** | Manager 부재/failover | implicit random manager 금지, explicit policy/role election | M4 |
| **OQ-043** | authoritative SupervisorRef 변경 가능 여부 | P0 one-authoritative-supervisor, change는 explicit revision | M4 ADR |
| **OQ-044** | Channel archive와 running Task | auto-cancel 금지; reject/pending/explicit control 중 정책 결정 | M1/M4 |
| **OQ-045** | revoked Membership과 running Execution | new access/control 차단 + immutable Context; high-risk reauth class 결정 | M2/M3 Security |
| **OQ-046** | Channel Thread branch participant inheritance | implicit full inheritance 금지, explicit/default-deny policy 검토 | M3/M4 |
| **OQ-047** | Project/Channel export/delete와 Bot-internalized Memory 관계 | provenance 유지 + independent retention/forget policy | M2/M5 |
| **OQ-048** | large Channel history/index 구조 | canonical history + paginated/cold projection/index, exact structure benchmark | M3/M6 |
| **OQ-049** | scope-aware cache invalidation | revision/generation key + selective invalidation, global flush 남발 금지 | M2/M3 Perf |
| **OQ-050** | user/principal의 Channel participation physical model | Bot runtime membership과 user access grant semantic은 분리 유지 | M1/API ADR |

## 4. v0.5에서 이미 고정된 항목

Open Question이 아니다.
- Project/Channel은 Brain이 아님
- Channel은 P0에서 Project-scoped
- Project Membership ≠ Channel Membership
- Role ≠ Authority
- Bot-only Main Conversation path 유지
- Thread identity 보존 + Conversation Parent 일반화
- Memory Scope = Bot/Project/Channel/Thread independent scopes
- Shared Memory ≠ Bot Memory replica
- promotion은 source-preserving explicit semantic
- final Canonical authorization after candidate/index
- Channel Coordinator는 Brain이 아님
- participant fan-out은 bounded
- Dynamic Core/Scheduler ownership 유지
- natural-language Message ≠ Control Directive
- Manager가 다른 Bot Brain/Core/Memory 직접 mutation 금지

## 5. 결정 증거

각 OQ는 use/non-use case, invariants, security/privacy, RSS/allocation/cache, concurrency model, fault/recovery, migration/rollback, API compatibility, Provider independence, benchmark/prototype, owner, ADR link를 사용한다.

## 6. 검증 기준

- OQ-031~050이 해당 Milestone 전에 ADR/실험/benchmark로 닫힘.
- 질문이 남아 있다는 이유로 unsafe implicit inheritance/all-member wake/direct Core control을 만들지 않음.
- exact enum/threshold/physical schema를 증거 없이 Normative 문서에 고정하지 않음.

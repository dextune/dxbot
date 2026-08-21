---
title: "미결정 사항과 권장 기본값"
document_id: "DXB-DEL-063"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-GOV-003", "DXB-DEL-060"]
---


# 미결정 사항과 권장 기본값

## 1. 목적

구현 중 암묵적으로 결정될 수 있는 쟁점을 공개하고, 결론 전까지 사용할 안전한 기본값과 결정 Gate를 제시한다.

## 2. 책임 범위

- open question
- recommended default
- decision evidence
- deadline milestone
- owner/ADR
- 영향 범위

질문을 이유로 개발을 멈추지 않으며, 권장 기본값으로 진행하되 비가역 변경 전 ADR을 확정한다.

## 3. P0 결정 필요

| ID | 질문 | 권장 기본값 | 결정 Gate |
|---|---|---|---|
| OQ-001 | DeepSeek Harness를 sidecar로 실제 사용하거나 철학만 차용할 것인가 | Port+Fake 우선, sidecar는 optional adapter | M0 exit |
| OQ-002 | P0 embedded DB 구현은 무엇인가 | transactional embedded SQL 계열, domain-specific store | M0 storage spike |
| OQ-003 | Event payload 형식 | versioned structured JSON + normalized hot columns | M0 perf/migration |
| OQ-004 | ID 형식 | time-sortable 128-bit newtype, exact format ADR | schema freeze |
| OQ-005 | Bot별 Coordinator activation 정책 | lazy activation + idle eviction | M1 soak |
| OQ-006 | Model/tool streaming event 보존 범위 | trajectory Artifact, Domain에는 summary/ref | M2 privacy/cost |
| OQ-007 | Rust async runtime/library stack | 하나의 표준 async runtime과 tracing stack | workspace freeze |
| OQ-008 | local control transport | Unix socket/named pipe abstraction | M1 CLI |
| OQ-009 | CLI/daemon binary 분리 | 단일 package, logical binaries 허용 | M1 packaging |
| OQ-010 | Memory 삭제와 backup 적용 | immediate query tombstone + backup retention ledger | M2 policy |
| OQ-011 | Sandbox 최소 지원 OS | Linux 강한 enforcement, 타 OS capability matrix/fail-closed | M2 release |
| OQ-012 | 실제 모델 E2E를 release gate로 둘 범위 | canary, core gate는 Fake | M2 CI |

## 4. P1 결정 필요

| ID | 질문 | 권장 기본값 | 결정 Gate |
|---|---|---|---|
| OQ-101 | 자동 Task 분해를 누가 수행하는가 | Brain proposal + deterministic scheduler validation | M3 |
| OQ-102 | Core fairness 알고리즘 | per-Bot round-robin + priority aging | M3 load |
| OQ-103 | Memory vector index | optional derived provider | M2/M3 benchmark |
| OQ-104 | Bot capability discovery 저장소 | projection catalog, authorization 아님 | M4 |
| OQ-105 | Bot 간 transport | local durable DB first | M4 |
| OQ-106 | TUI framework | event-driven, Control client only | M5 spike |
| OQ-107 | Remote API 방식 | HTTP+SSE 우선, worker RPC 별도 | M5 |
| OQ-108 | Approval policy DSL | typed policy config, 임의 script 금지 | M2/M5 |
| OQ-109 | Audit store 분리 | logical separate, physical embedded 가능 | M5 security |
| OQ-110 | Artifact content addressing | digest 기반, encryption/retention metadata | M2 |

## 5. P2 결정 필요

| ID | 질문 | 권장 기본값 | 결정 Gate |
|---|---|---|---|
| OQ-201 | Web frontend 기술 | API contract와 독립 선택 | M6 |
| OQ-202 | external DB | 측정된 single-writer 병목 후 선택 | M7 entry |
| OQ-203 | remote worker protocol | CoreExecutor wire contract | M7 |
| OQ-204 | multi-tenancy | P0에서는 single trusted owner, 타입 확장 여지 | M7 |
| OQ-205 | Bot ownership/sharding | BotId single-writer lease | M7 |
| OQ-206 | Plugin SDK | 안정 wire protocol, Rust ABI 직접 공개 금지 | post-M6 |
| OQ-207 | shared cross-Bot Memory | explicit shared namespace+ACL, 기본 private | post-M4 |
| OQ-208 | HA와 RPO/RTO | deployment tier별 | M7 |

## 6. 결정 템플릿

각 질문은 다음 증거로 닫는다.
- 사용 사례와 비사용 사례
- product invariant 영향
- security/privacy
- performance/memory benchmark
- operational complexity
- migration/rollback
- alternatives
- prototype/conformance
- owner
- ADR link

## 7. 권장 결론 상세

### Harness
DXBOT Domain과 Runtime을 DeepSeek 구현에 직접 맞추지 않는다. sidecar가 성공하면 초기 실행 Provider로 활용하고, 실패해도 Fake/native minimal 경로로 제품 핵심을 개발한다.

### Storage
범용 Repository를 만들지 않고 Bot/Task/Memory Store 계약을 둔다. embedded DB 선택이 미래 external DB의 lowest-common-denominator API를 강요하지 않게 한다.

### Memory Index
검색 Index는 derived다. vector provider 선택이 Memory Canonical schema와 삭제 semantics를 바꾸지 않게 한다.

### UI
CLI/TUI/Web 기술 선택은 Control API 이후 결정한다. UI 기술이 Runtime crate, DB, Harness 타입에 영향을 주지 않아야 한다.

## 8. 예외상황

- benchmark 없이 성능을 이유로 결정을 닫지 않는다.
- upstream 문서만으로 DXBOT 요구 충족을 가정하지 않는다.
- "나중에" 상태의 P0 질문은 M0/M1 gate를 넘지 못한다.
- reversible default로 진행 중에도 wire/data schema freeze 전 ADR이 필요하다.
- 결정이 변경되면 Migration과 traceability를 갱신한다.

## 9. 확장성

질문 수가 늘어나면 상태(`Open/Experimenting/Decided/Deferred`)와 milestone filter를 Control Plane 또는 issue tracker에 연결한다. Decided 항목은 ADR로 이동하고 이 문서에는 링크와 요약만 남긴다.

## 10. 구현 우선순위

- **P0:** OQ-001~012
- **P1:** OQ-101~110
- **P2:** OQ-201~208

## 11. 검증 기준

- M0/M1 exit 전에 P0 질문이 ADR 또는 명시적 실험 결과로 닫힌다.
- 모든 Open 질문에 권장 기본값이 있어 개발이 불필요하게 중단되지 않는다.
- 비가역 schema/protocol 결정은 benchmark/migration/rollback 증거를 가진다.

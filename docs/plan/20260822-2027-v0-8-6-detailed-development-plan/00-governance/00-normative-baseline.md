---
title: "DXBOT v0.8.6 최상위 구현 기준선"
document_id: "DXB-BASE-000"
version: "0.8.6"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: []
---
# DXBOT v0.8.6 최상위 구현 기준선

## 1. 효력과 버전

본 문서는 active package v0.8.6의 최상위 제품·구현 기준선이다. 과거 package는 provenance이며 현재 의미를 직접 결정하지 않는다.

```text
plan_version              = active package 계약 버전
review_revision           = plan line의 단조 증가 교정 revision
adversarial_review_rounds = 이번 교정에 사용한 독립 검토 관점 수
final_rechecks            = 변경 완료 후 별도 재검수 수
document_version          = 개별 Canonical Owner 의미 버전
runtime_version           = 실행 바이너리 버전
```

## 2. 제품 불변조건

- `INV-001`: Bot은 Interface Session과 독립된 Persistent Identity·Memory·State를 가진다.
- `INV-002`: 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
- `INV-003`: Core는 Scheduler-owned Dynamic Lease이며 Bot 복제본이나 독립 Agent가 아니다.
- `INV-004`: Bot당 하나의 Persistent Main Conversation을 유지한다.
- `INV-005`: Conversation, Thread, Task, Execution, Core Lease, Provider Session, Interface Session은 서로 다른 identity다.
- `INV-006`: Project와 Channel은 협업·권한·지식·자원 Scope이며 Brain을 소유하지 않는다.
- `INV-007`: Conversation History와 Memory를 구분하고 Memory assertion은 scope·revision·provenance를 가진다.
- `INV-008`: running Execution의 Context Plan, Task revision, Provider binding은 immutable snapshot이다.
- `INV-009`: Durable Process는 cross-aggregate 진행만 소유하며 child canonical state를 복제하지 않는다.
- `INV-010`: Authorization Decision Owner와 Runtime Resource Owner는 각각 하나다.
- `INV-011`: Provider, Plugin, Interface는 Domain Identity와 Canonical State를 소유하지 않는다.
- `INV-012`: Bot-only path는 Project·Channel·Durable Process 없이 완전하게 동작한다.
- `INV-013`: 최소 하나의 실제 Harness Adapter 경로가 Provider Host를 통해 Task를 수행해야 한다.
- `INV-014`: Multi-Bot은 typed delegation과 membership boundary로 동작하며 anonymous sub-agent로 축약하지 않는다.
- `INV-015`: P0 Dynamic Core 병렬성은 서로 다른 admitted Execution 간 병렬성이다. 한 Execution 내부 anonymous fan-out은 P0가 아니다.

## 3. Active P0 Scope

```text
Linux user-scoped Runtime Instance
→ Domain/Application/Persistence
→ Provider Host + deterministic Reference Provider + one real Harness Adapter
→ typed Command / Query / Subscription / Host Action
→ authenticated Unix-domain Control Endpoint / Client
→ dxb CLI
```

TUI, Web, BFF, remote multi-tenant transport, distributed HA, generic Workflow/RPC/IDL framework, Plugin ecosystem 전체 CLI는 비범위다.

## 4. 비협상 구현 계약

1. public mutation은 typed operation만 사용하고 generic JSON dispatch를 금지한다.
2. client의 `RequestDigest`는 client가 아는 typed selector/payload만 포함한다.
3. server는 Principal, resolved target, authorization/policy generation을 `ResolvedBindingDigest`로 별도 결박한다.
4. Local Journal, Operation Receipt, Directive, Target Lifecycle을 하나의 enum으로 합치지 않는다.
5. `Committed`는 해당 Application operation 자체의 canonical mutation commit이며 target terminal을 뜻하지 않는다.
6. IdempotencyKey는 검증 가능한 issuance epoch를 포함하고 acceptance horizon 밖의 key를 신규 mutation으로 재해석하지 않는다.
7. Query는 bounded snapshot/cursor, Subscription은 explicit terminal/gap/resync를 사용한다.
8. stdout=result, stderr=diagnostic, error/exit/output schema는 active package 안에서 닫힌다.
9. OS peer에서 Principal을 server-side로 파생하며 profile·client payload·BotId는 Authority가 아니다.
10. 최초 Instance는 안전한 default policy generations와 owner authority binding을 원자적으로 생성한다.
11. Provider, queue, cache, cursor, receipt, local journal은 item과 byte 상한을 가진다.
12. Storage profile은 atomic commit, versioned snapshot, receipt recovery, bounded disk pressure를 executable spike로 증명한다.
13. Linux export는 descriptor-relative no-follow와 atomic no-replace를 사용한다.
14. graceful Runtime shutdown과 host process stop은 서로 다른 typed operation이다.
15. validator PASS는 문서 구조 증거이며 Rust/Runtime semantics PASS가 아니다.

## 5. Canonical Owner Registry

<!-- canonical-owner-registry:start -->
- `active-package` | `DXB-GOV-001` | `plan-governance` | `repository` | `DXB-INDEX`
- `product-invariants` | `DXB-BASE-000` | `baseline` | `repository` | `DXB-INDEX`
- `terminology` | `DXB-GOV-002` | `domain-language` | `repository` | `N/A`
- `application-operation-metadata` | `DXB-IFC-040` | `application-contract` | `contract-snapshot-store` | `application-contract`
- `cli-projection-and-local-journal` | `DXB-IFC-041` | `cli` | `cli-state-store` | `application-contract`
- `cli-input-grammar` | `DXB-IFC-042` | `application-contract` | `contract-snapshot-store` | `application-contract`
- `bot-identity` | `DXB-DOM-020` | `domain-bot` | `bot-store` | `application-contract`
- `context-plan` | `DXB-DOM-021` | `application-context-plan` | `execution-store` | `application-contract`
- `memory-assertion` | `DXB-DOM-022` | `domain-memory` | `memory-store` | `application-contract`
- `task-execution-side-effect` | `DXB-DOM-023` | `domain-task` | `task-side-effect-store` | `application-contract`
- `core-lease` | `DXB-DOM-024` | `runtime-scheduler` | `transient-runtime-state` | `application-contract`
- `delegation` | `DXB-DOM-025` | `application-delegation` | `task-store` | `application-contract`
- `use-case-catalog` | `DXB-DOM-026` | `application` | `operation-store` | `application-contract`
- `conversation-thread` | `DXB-DOM-027` | `domain-conversation` | `conversation-store` | `application-contract`
- `project-membership` | `DXB-DOM-028` | `domain-project` | `project-store` | `application-contract`
- `channel-membership` | `DXB-DOM-029` | `domain-channel` | `channel-store` | `application-contract`
- `storage-semantics` | `DXB-ARC-015` | `storage` | `canonical-storage` | `N/A`
- `storage-proof-profile` | `DXB-ARC-018` | `storage-spike` | `test-fixtures` | `N/A`
- `harness-capability` | `DXB-ARC-013` | `provider-host` | `provider-registry` | `capability-contract`
- `resource-admission` | `DXB-RUN-031` | `runtime-resource-governor` | `resource-ledger` | `application-contract`
- `principal-approval-authority` | `DXB-RUN-032` | `runtime-security` | `security-store` | `application-contract`
- `recovery` | `DXB-RUN-033` | `runtime-recovery` | `canonical-storage` | `application-contract`
- `audit` | `DXB-RUN-034` | `runtime-audit` | `audit-store` | `application-contract`
- `bootstrap-default-policy` | `DXB-RUN-035` | `runtime-bootstrap` | `instance-metadata-store` | `application-contract`
- `control-directive` | `DXB-RUN-036` | `application-control` | `directive-store` | `application-contract`
- `durable-process` | `DXB-RUN-038` | `runtime-process` | `process-store` | `application-contract`
<!-- canonical-owner-registry:end -->

## 6. 구현 진입 판정

Workspace bootstrap, M1A spike, M1B contract source 구현은 허용한다. 전체 parser/schema freeze는 `AT-STORAGE-001`, `AT-CONTRACT-001`, `AT-SUBMIT-001`, `AT-BOOT-001`의 executable PASS와 generated snapshot drift 0 이후에만 허용한다.

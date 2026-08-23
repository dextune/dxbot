---
title: "DXBOT v0.8.10 최상위 구현 기준선"
document_id: "DXB-BASE-000"
version: "0.8.10"
status: "Normative Baseline"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: []
---
# DXBOT v0.8.10 최상위 구현 기준선

## 1. 효력과 범위

본 문서는 active package v0.8.10의 최상위 제품·구현 기준선이다. 과거 package와 원문 보존본은 provenance이며 현재 의미를 직접 결정하지 않는다. 동일 개념의 세부 규칙은 Canonical Owner 문서가 소유하고, 본 기준선은 제품 불변조건과 구현 진입 조건을 소유한다.

```text
plan_version              = active package 계약 버전
review_revision           = plan line의 단조 증가 교정 revision
adversarial_review_rounds = 이번 교정에 사용한 독립 검토 관점 수
final_rechecks            = 변경 완료 후 별도 재검수 수
document_version          = 개별 Canonical Owner 의미 버전
runtime_version           = 실행 바이너리 버전
```

## 2. 제품 불변조건

<!-- product-invariants:start -->
- `INV-001` | Bot은 Interface Session과 독립된 Persistent Identity·Memory·State를 가진다.
- `INV-002` | 하나의 Bot은 하나의 logical Brain semantics를 유지한다.
- `INV-003` | Core는 Scheduler-owned Dynamic Lease이며 Bot 복제본이나 독립 Agent가 아니다.
- `INV-004` | Bot당 하나의 Persistent Main Conversation을 유지한다.
- `INV-005` | Conversation, Thread, Task, Execution, Core Lease, Provider Session, Interface Session은 서로 다른 identity다.
- `INV-006` | Project와 Channel은 협업·권한·지식·자원 Scope이며 Brain을 소유하지 않는다.
- `INV-007` | Conversation History와 Memory를 구분하고 Memory assertion은 scope·revision·provenance를 가진다.
- `INV-008` | running Execution의 Context Plan, Task revision, Provider binding은 immutable snapshot이다.
- `INV-009` | Durable Process는 cross-aggregate 진행만 소유하며 child canonical state를 복제하지 않는다.
- `INV-010` | Authorization Decision Owner와 Runtime Resource Owner는 각각 하나다.
- `INV-011` | Provider, Plugin, Interface는 Domain Identity와 Canonical State를 소유하지 않는다.
- `INV-012` | Bot-only path는 Project·Channel·Durable Process 없이 완전하게 동작한다.
- `INV-013` | 최소 하나의 실제 Harness Adapter 경로가 Provider Host를 통해 Task를 수행해야 한다.
- `INV-014` | Multi-Bot은 typed delegation과 membership boundary로 동작하며 anonymous sub-agent로 축약하지 않는다.
- `INV-015` | P0 Dynamic Core 병렬성은 서로 다른 admitted Execution 간 병렬성이다. 한 Execution 내부 anonymous fan-out은 P0가 아니다.
<!-- product-invariants:end -->

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
2. `in-*` metadata는 하나의 source에서 `CliInput`과 `CommandPayload` 두 projection을 생성한다. `@local`과 local source handle은 wire payload와 `RequestDigest`에 들어가지 않는다.
3. file/stdin source는 `Prepared` 이전 bounded bytes 또는 typed `ArtifactRef`로 materialize하며 path, file descriptor, source variant를 전송하지 않는다.
4. CLI는 `Prepared`를 durable create한 뒤 `Dispatching` append+fsync를 완료하고서만 첫 network byte를 보낸다.
5. Local Journal, Operation Receipt, Directive, Target Lifecycle을 하나의 enum이나 canonical store로 합치지 않는다.
6. `CommandId`는 Runtime Instance 전역 logical submission identity다. `IdempotencyKey`는 authenticated Principal과 issuance epoch에 결박되며 mismatch는 신규 operation을 만들지 않는다.
7. binding과 최소 tombstone은 nonterminal/recovery 및 key acceptance horizon을 침해하도록 compact하지 않는다. horizon 밖 retry는 expired로 거부하고 신규 mutation으로 재해석하지 않는다.
8. Runtime은 Principal 인증 직후 existing global CommandId와 principal-scoped Idempotency binding을 selector·authorization 재평가보다 먼저 조회한다.
9. `Committed`는 Application operation 자체의 canonical mutation commit이며 target terminal을 뜻하지 않는다.
10. PreAcceptError, Receipt `Rejected`, committed Domain outcome `Rejected/Deferred`를 구분한다.
11. approval-required operation은 durable intent와 Receipt `Accepted`를 유지하고 Approval decision이 동일 operation을 재평가해 종결한다.
12. Query는 bounded snapshot/cursor, Subscription은 explicit terminal/gap/resync를 사용한다.
13. stdout=result, stderr=diagnostic 원칙을 유지한다. SIGINT, broken pipe, local timeout은 관찰·출력만 종료하며 별도 typed cancel 없이는 Runtime operation을 취소하지 않는다.
14. OS peer에서 Principal을 server-side로 파생하며 profile, client payload, BotId는 Authority가 아니다.
15. 최초 Instance는 Principal, default policy generations, owner authority, HostGeneration, schema version을 원자적으로 생성한다.
16. Provider, queue, cache, cursor, receipt, local journal은 item과 byte 상한을 가진다.
17. Storage profile은 atomic commit, versioned snapshot, receipt/binding recovery, compaction tombstone, bounded disk pressure를 executable spike로 증명한다.
18. Linux export는 descriptor-relative no-follow와 atomic no-replace를 사용한다.
19. graceful Runtime shutdown과 host process stop은 서로 다른 typed operation이다.
20. Bot identity creation은 Provider readiness와 독립적이며 Provider availability는 activation/execution admission에서 판단한다.
21. local profile/Instance selector는 endpoint discovery일 뿐 Authority가 아니다. 선택 실패 시 다른 Instance로 silent fallback하지 않는다.
22. mutation의 canonical ID와 expected revision/generation은 사용자가 내부 저장소 값을 직접 운반하도록 강제하지 않는다. CLI가 `Prepared` 이전 bounded preflight로 materialize하고 Runtime이 독립적으로 검증한다.
23. user-facing command path의 target은 해당 도메인 selector여야 한다. `conversation send`는 Bot Main Conversation을, `channel send`는 Channel Conversation을 내부 ID 노출 없이 resolve할 수 있어야 한다.
24. `dxb`, `--help`, group help와 local client version은 Runtime 없이 동작한다. Runtime이 필요한 명령은 자동 start하지 않고 typed next action을 반환한다.
25. human/json/jsonl rendering, prompt, progress, color, pager는 local projection이며 CommandPayload·RequestDigest·Authority를 바꾸지 않는다.
26. error/result는 stable code와 typed next action을 제공하되 hidden resource 존재나 secret/content body를 노출하지 않는다.
27. Acceptance는 정확히 하나의 최초 요구 milestone을 가지며 Gate는 누적된다. command subset은 자신의 Acceptance와 모든 선행 Gate가 PASS한 뒤에만 freeze한다.
28. document validator PASS는 generated Rust contract, storage durability, authorization correctness, real Harness 실행 증거를 대신하지 않는다.

## 5. Canonical Owner Registry

<!-- canonical-owner-registry:start -->
- `active-package` | `DXB-GOV-001` | `plan-governance` | `repository` | `DXB-INDEX`
- `product-invariants` | `DXB-BASE-000` | `baseline` | `repository` | `DXB-INDEX`
- `terminology` | `DXB-GOV-002` | `domain-language` | `repository` | `N/A`
- `application-operation-metadata` | `DXB-IFC-040` | `application-contract` | `contract-snapshot-store` | `application-contract`
- `operation-receipt` | `DXB-ARC-014` | `application-operation` | `operation-store` | `application-contract`
- `cli-projection-and-local-journal` | `DXB-IFC-041` | `cli` | `cli-state-store` | `application-contract`
- `cli-input-grammar` | `DXB-IFC-042` | `application-contract` | `contract-snapshot-store` | `application-contract`
- `cli-user-journeys` | `DXB-IFC-043` | `cli-experience` | `cli-state-store` | `application-contract`
- `bot-identity` | `DXB-DOM-020` | `domain-bot` | `bot-store` | `application-contract`
- `context-plan` | `DXB-DOM-021` | `application-context-plan` | `execution-store` | `application-contract`
- `memory-assertion` | `DXB-DOM-022` | `domain-memory` | `memory-store` | `application-contract`
- `task-execution-side-effect` | `DXB-DOM-023` | `domain-task` | `task-side-effect-store` | `application-contract`
- `core-lease` | `DXB-DOM-024` | `runtime-scheduler` | `transient-runtime-state` | `application-contract`
- `delegation` | `DXB-DOM-025` | `application-delegation` | `task-store` | `application-contract`
- `use-case-catalog` | `DXB-DOM-026` | `application` | `N/A` | `application-contract`
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

Workspace bootstrap과 M1A storage spike는 GO다. M1B contract source prototype은 GO지만 public parser/schema freeze는 금지한다. M2/M3/M4/M5 command subset은 누적 Gate PASS 뒤에만 freeze하며 전체 63-operation freeze는 M6에서만 허용한다.

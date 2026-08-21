---
title: "테스트·검증·3회 전체 Review 전략"
document_id: "DXB-ENG-052"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ENG-050", "DXB-ENG-051", "DXB-IFC-040", "DXB-IFC-041", "DXB-RUN-033"]
---

# 테스트·검증·3회 전체 Review 전략

## 1. 목적

v0.8의 self-contained baseline, public contract, Runtime Instance, CLI security/automation을 deterministic test와 세 번의 독립적 전체-package Review로 검증한다. 문서에 PASS라고 적는 것과 실행 증거를 구분한다.

## 2. Test Layer

```text
Unit / State Machine
→ Property / Model
→ Concurrency / Fault Injection
→ Component / Contract
→ Provider Host / Conformance
→ Runtime Host / Control Integration
→ CLI E2E
→ Recovery / Security
→ Performance / Soak
→ Architecture / Docs / Schema CI
```

fake clock/ID/RNG/barrier/reference Provider를 사용한다. Core correctness를 실제 모델 응답이나 sleep timing에 의존시키지 않는다.

## 3. Deterministic Fixtures

- versioned Application Contract request/response/event/error schema
- operation registry/exit/help golden
- exact/same-name/alias/ambiguous selector set
- Operation Receipt store and RequestDigest canonicalization
- commit 직후 response drop failpoint
- same key/same digest 및 same key/different binding
- receipt nonterminal/terminal/expiry/reconcile
- Linux user Runtime Instance, fake user service manager, UDS endpoint
- 100-way concurrent start; stale lock/socket/descriptor; start-stop-restart races
- snapshot list with concurrent insert/update/delete
- foreign/expired/reauth/schema-major cursor
- at-least-once duplicate event, gap, slow consumer, reconnect, SIGINT, broken pipe
- complete/partial/failed JSONL terminal records
- local wait timeout while Runtime mutation continues
- endpoint permission/peer/symlink/hijack
- ANSI/OSC/C0/C1/bidi/confusable terminal input
- traversal/symlink/FIFO/device/existing-file/disk-full export
- Runtime restart with same Bot/Conversation/Thread/Task/Memory/Process IDs
- memory pressure/admission/decoded expansion
- Provider unavailable/drain/generation late callback

## 4. Acceptance Suites

### AT-BASE-001 — Self-Contained Effective Baseline
v0.8 active package 밖의 normative 문장을 읽지 않고 owner, dependency, P0 scope, acceptance를 판정한다. hidden inheritance와 duplicate owner 0.

### AT-CLI-008 — P0 Command Matrix Completeness
모든 P0 command가 one typed Application operation, kind, target, revision, idempotency, wait, machine schema, exit/security/resource, Acceptance에 연결된다.

### AT-APP-005 — Operation Receipt / Key Binding
commit-after-response-loss와 CLI restart에서 기존 outcome 복구; same key/different binding conflict; duplicate effect 0.

### AT-CLI-009 — Selector Ambiguity Safety
동일 이름·alias·scope fixture에서 wrong-target mutation 0; fuzzy/last-used mutation 0; bounded candidate error.

### AT-HOST-001 — Runtime Instance / Concurrent Start
same data root active HostGeneration/writer/endpoint 1개; stale descriptor/lock 안전 복구; instance mismatch fail.

### AT-APP-006 — Pagination Snapshot / Cursor Binding
stable sort/tie-breaker, concurrent mutation 중 snapshot duplicate/missing 0; foreign/expired/reauth cursor explicit.

### AT-APP-007 — Subscription Partial / Terminal / Resume
at-least-once duplicate 식별, gap/resync, partial/terminal/last safe cursor, local cancel independence.

### AT-CLI-010 — Machine Wait / Timeout / Partial
accepted/pending/committed/target terminal/local timeout/partial exit가 script에서 구분되고 timeout 후 duplicate retry 0.

### AT-SEC-005 — Local Endpoint / Terminal / Export Safety
endpoint hijack/permission/symlink, ANSI/OSC, traversal/symlink/special file/partial sensitive artifact fail closed.

### AT-SCHEMA-001 — Contract Schema SSOT / Drift
Rust source→deterministic schema/golden/help/operation/exit registry; manual divergent DTO 0.

### AT-CI-001 — Actual Gate Execution
active package를 자동 탐색해 inventory/ID/DAG/status/scope/traceability/schema/matrix/dependency Gate를 PR에서 실제 실행한다.

기존 AT-APP-001~004, AT-CLI-001~007과 Backend Bot/Memory/Task/Process/Security/Resource Acceptance를 regression suite로 유지한다.

## 5. Fault Matrix

각 representative vertical slice에 다음을 결합한다.

```text
Runtime absent / concurrent start / readiness timeout
stale endpoint / instance mismatch / permission deny
ambiguous selector / not-found / stale revision
approval required / authority revoke
commit after response loss / same key different digest
CLI crash / SIGINT / broken pipe / local timeout
Runtime restart / endpoint recreation / Provider loss
snapshot concurrent mutation / cursor expiry / schema mismatch
slow consumer / gap / partial JSONL
export traversal / symlink / disk full / cleanup failure
memory pressure / decoded expansion / resource reject
```

각 fault에서 duplicate effect, wrong target, identity drift, silent loss, unbounded allocation, secret/terminal/file bypass, resource leak을 검사한다.

## 6. Review 1 — Effective Baseline / Structural / Traceability

전체 47개 active Markdown을 검사한다.

- file/path lowercase kebab-case와 inventory
- unique document_id, depends_on 존재성/cycle
- active P0 status와 owner
- hidden previous-version inheritance
- Canonical Owner 중복과 same-rule duplication
- P0/P1/P2 Command Matrix completeness
- Requirement/ADR/Owner/Test/Acceptance/Risk/OQ 연결
- manifest resolution drift
- active TUI/Web/BFF residue

발견 사항을 수정한 뒤 동일 전체 범위를 처음부터 재실행한다.

## 7. Review 2 — Cross-Layer Executability / Fault / Compatibility

전체 경로를 추적한다.

```text
Shell
→ parser/global options/selector
→ Host Lifecycle or Control Client
→ Endpoint/version/principal
→ Application Contract/Receipt
→ Application/Domain/Persistence
→ Runtime/Scheduler/Provider Host
→ Result/Event/Projection
→ Page/Subscription
→ Human/Machine/File Output
```

Section 5 fault를 삽입하고 owner 없는 상태, shortcut, duplicate effect, identity drift, silent loss, unbounded resource, unsafe output이 없는지 검사한다. 수정 후 전체 경로를 재실행한다.

## 8. Review 3 — Adversarial Scope / Overengineering / Ambiguity

전체 package에서 질문한다.
- P0 use-case 없는 abstraction/framework/crate가 추가됐는가
- 미래 TUI/Web/remote를 이유로 generic layer를 만들었는가
- alias/DTO/schema가 같은 operation/policy를 복제하는가
- exact library/data structure/threshold를 증거 없이 고정했는가
- 반대로 selector/receipt/cursor/wait/security처럼 구현자가 정책을 발명해야 하는 공백이 남았는가
- Provider/Plugin/Core/Routine 관리가 P0에 과도하게 들어왔는가
- CLI 장식이 Runtime correctness/resource/security보다 앞섰는가

수정 후 전체 package를 처음부터 재실행한다.

## 9. Evidence

각 Review는 다음을 `manifest.md`와 CI artifact에 남긴다.
- 실행 시점/대상 package hash
- 검사 항목과 tool/command
- 발견 ID·영향 문서
- 수정 요약
- 재실행 결과
- 실행하지 못한 code/build/performance gate 구분

문서 작성 단계의 PASS는 문서 구조·계약 검수이며 실제 Rust build/E2E/performance/security 통과를 의미하지 않는다.

## 10. Release Gate

- Review 1/2/3 전체 재실행 PASS
- AT-BASE-001 문서 validator PASS
- 구현 단계에서 AT-APP/CLI/HOST/SEC/SCHEMA/CI deterministic suite PASS
- Backend identity/memory/task/process/security/resource regression PASS
- no active TUI/Web artifact/dependency
- Critical Risk owner/evidence 연결

## 11. 검증 기준

- 신규 11개 Acceptance 모두 owner/test/risk/roadmap 연결.
- 세 Review가 서로 다른 목적과 evidence를 가짐.
- 발견 수정 후 부분이 아닌 전체 범위 재실행.
- false-positive 검증용 intentionally broken fixture 존재.
- actual executable evidence와 self-authored manifest assertion 구분.

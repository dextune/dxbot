---
title: "v0.8.10 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---
# v0.8.10 아키텍처 결정 기준선

이 문서는 active package에서 유효한 ADR을 전부 열거한다. 과거 package와 Reference Snapshot의 문장은 이 active Baseline을 override하지 않는다.

<!-- architecture-decision-registry:start -->
- `ADR-0095` | 원문 보존본은 비규범 provenance이며 active Baseline을 override하지 않는다.
- `ADR-0096` | plan version, review revision, review pass count, document version을 분리한다.
- `ADR-0097` | mutation client는 per-command journal을 durable prepare한 뒤 송신한다.
- `ADR-0098` | P0 Principal은 `InstanceId + authenticated OS UID`에서 server-side로 파생한다.
- `ADR-0099` | 최초 Instance는 Principal, default policy, owner authority generation을 원자적으로 생성한다.
- `ADR-0100` | Storage는 `DXB-ARC-018` executable spike를 통과하기 전 public trait를 freeze하지 않는다.
- `ADR-0101` | deterministic Reference Provider와 별도로 one real Harness Adapter canary를 요구한다.
- `ADR-0102` | Project/Channel membership과 Multi-Bot delegation은 typed Application operation이다.
- `ADR-0103` | Linux export는 descriptor-relative no-follow와 atomic no-replace를 사용한다.
- `ADR-0104` | graceful Runtime shutdown과 host process stop은 서로 다른 operation이다.
- `ADR-0105` | operation metadata가 parser, help, schema, error, exit snapshot의 SSOT다.
- `ADR-0106` | read-only active plan validator를 PR/main에서 실행한다.
- `ADR-0107` | `RequestDigest`는 client-visible semantic만 포함하고 `ResolvedBindingDigest`는 server-only binding을 포함한다.
- `ADR-0108` | Local Journal, Receipt, Directive, Target Lifecycle 상태기계를 분리하고 wait predicate를 operation metadata에 제한한다.
- `ADR-0109` | IdempotencyKey는 issuance epoch를 포함하며 acceptance horizon 밖 key는 신규 mutation으로 취급하지 않는다.
- `ADR-0110` | Approval, AuthorityBinding, ActionGrant는 `DXB-RUN-032`, Side Effect Ledger는 `DXB-DOM-023`이 소유한다.
- `ADR-0111` | default policy는 deny-unknown, bounded resource, no auto declassification/promotion, no Reference Provider fallback을 의미한다.
- `ADR-0112` | stable output schema와 exit code 0, 2~18 mapping을 active package 안에 정의한다.
- `ADR-0113` | P0 Dynamic Core 병렬성은 서로 다른 admitted Execution 간 병렬성으로 제한한다.
- `ADR-0114` | review revision 숫자와 review evidence 수를 분리하고 Manifest가 5회 검토와 2회 재검수를 기록한다.
- `ADR-0115` | durable `Dispatching` journal record를 fsync한 이후에만 network send를 시작한다.
- `ADR-0116` | authenticated Principal 이후 existing idempotency/Command binding lookup을 selector·authorization 재평가보다 먼저 수행한다.
- `ADR-0117` | PreAcceptError, Receipt `Rejected`, committed Domain `Rejected/Deferred`를 서로 다른 상태로 취급한다.
- `ADR-0118` | approval-required operation은 pending durable intent를 유지하고 Approval decision이 원 operation을 자동 재평가·종결한다.
- `ADR-0119` | application-contract는 Receipt schema, application-operation은 Receipt lifecycle, operation-store는 persistence를 소유한다.
- `ADR-0120` | Membership row, AuthorityBinding generation/revoke, required AuditIntent는 하나의 Application Unit of Work에서 atomic commit한다.
- `ADR-0121` | P0 Application Command output은 `out-operation-v1`으로 수렴하고 `target-terminal` wait를 제거한다.
- `ADR-0122` | create command에서 global Instance revision CAS를 사용하지 않는다.
- `ADR-0123` | CLI contract freeze는 milestone별 subset으로 수행하며 M6 전 full 63-operation freeze를 금지한다.
- `ADR-0124` | Bot identity creation은 Provider readiness와 분리한다.
- `ADR-0125` | active package는 제품 불변조건, 모든 active ADR, fail-closed recovery/security 의미를 자체 열거하며 과거 package의 규범 문장을 상속하지 않는다.
- `ADR-0126` | 하나의 `in-*` source metadata에서 `CliInput`과 `CommandPayload`를 생성하고 `@local` 및 source handle을 wire와 digest에서 제거한다.
- `ADR-0127` | `CommandId`는 Instance 전역 unique identity이고 IdempotencyKey는 authenticated Principal과 issuance epoch에 결박한다.
- `ADR-0128` | binding compaction은 acceptance/recovery horizon 동안 최소 tombstone을 보존하고 expired retry를 신규 operation으로 재해석하지 않는다.
- `ADR-0129` | CLI journal은 command별 single writer를 강제하고 writer 종료 뒤 lock takeover를 허용하되 local interrupt를 Runtime cancel로 변환하지 않는다.
- `ADR-0130` | Acceptance는 정확히 하나의 최초 요구 milestone을 가지며 milestone Gate는 누적된다.
- `ADR-0131` | `version` command의 최소 호환성 조회 Acceptance와 M6 final schema freeze Acceptance를 분리한다.
- `ADR-0132` | validator negative fixture는 현재 문서 값을 동적으로 변이하며 과거 markdown count 같은 상수를 하드코딩하지 않는다.
- `ADR-0133` | Grok architecture adoption package는 비규범 Reference Snapshot이며 채택 결정은 active owner 문서와 executable evidence에만 반영한다.
- `ADR-0134` | Runtime composition은 `runtime-host`가 소유하고 private descriptor/graph로 시작한다. 새 crate, public extension schema, generic service locator를 만들지 않는다.
- `ADR-0135` | Diagnostic Event는 Durable Audit과 분리한다. Diagnostic/Health는 bounded derived path, Recovery Backstop은 canonical storage owner의 consistent snapshot path이며 silent restore를 금지한다.
- `ADR-0136` | Provider conformance는 HTTP streaming과 subprocess streaming 두 Reference Adapter가 동일 suite를 통과해 증명하며 Application/CLI provider-specific branch를 금지한다.
- `ADR-0137` | Sandbox는 Docker 비종속 contract로 정의하고 local subprocess prototype에서 ownership/generation/artifact/cancel/cleanup을 먼저 증명한다. Docker는 A4의 선행 조건이 아니다.
- `ADR-0138` | P0 production Provider config는 Runtime Host의 owner-only `provider-config.json`이 generation/capability/endpoint/model/credential reference를 소유하고, credential material은 Runtime Host process 환경에서 reference로만 resolve해 Common transport에 일시 전달한다. Provider registration/lifecycle은 동일 `ProviderHost`가 소유하며 secret 원문은 config/discovery/Application/journal/audit에 저장하지 않고 ReferenceProvider production fallback을 금지한다.
- `ADR-0139` | required `AuditIntent`는 security/Application producer UoW에서 대상 transition과 원자적으로 durable commit하며, `runtime-audit`의 owner-only bounded outbox가 redaction 후 `AuditSequence + previous digest + RecordDigest` canonical audit record를 소유한다. Intent→record projection은 deterministic key로 exactly-once 재구성 가능하고 projection integrity/capacity failure 시 새 Provider dispatch를 fail closed한다.
- `ADR-0140` | Linux P0 supported service profile은 packaged systemd user unit이 foreground Runtime Host packaging entrypoint를 소유하고 authenticated generation-fenced `ExecStop`으로 graceful drain을 시작한다. public 63-command CLI registry는 확장하지 않으며 admission stop → activity drain/Core Lease 0 → audit checkpoint → endpoint/discovery/PID unpublish → bounded single-writer lock handoff 순서를 강제한다.
<!-- architecture-decision-registry:end -->

## Grok adoption A0 decision lock

| 항목 | 결정 | Canonical Owner | 현재 gap | 비차용 경계 |
|---|---|---|---|---|
| Runtime composition | Close | `runtime-host` + `runtime-bootstrap` | graph validation, deterministic lifecycle, rollback/readiness aggregate 미구현 | 새 crate/public plugin schema 금지 |
| Diagnostics/backstop | Adopt/Close | `runtime-audit` + `runtime-host` + canonical storage owner | typed diagnostic family, bounded sink, backstop/quarantine evidence 미구현 | audit 중복 저장, raw DB upload, silent restore 금지 |
| Provider conformance | Close | `provider-host` | HTTP common path는 있으나 subprocess reference와 공통 conformance matrix 미구현 | 상용 provider 수 확대, Application/CLI provider branch 금지 |
| Sandbox lifecycle | Observe → Close | `runtime-security` + `runtime-host` | Docker 비종속 spec, ownership/generation/artifact fencing, subprocess isolation 미구현 | Docker 필수화, canonical store/credential directory direct mount 금지 |

A0 gate는 다음으로 잠근다.

- 위 네 항목의 owner는 서로 중복된 canonical state를 만들지 않는다.
- `docs/plan/20260825-0930-grok-architecture-adoption-plan/`은 설계 provenance와 acceptance reference이며 active command/schema를 직접 변경하지 않는다.
- A1~A4는 기존 crate 내부에서 구현하고 새 crate를 추가하지 않는다.
- A1/A2 private contract evidence 전에는 public extension/diagnostic/sandbox DTO를 freeze하지 않는다.
- A3는 provider 개수 확대가 아니라 서로 다른 transport/lifecycle 특성의 conformance 증명으로 제한한다.
- A4는 local subprocess prototype까지이며 Docker adapter는 필요성 증거가 생기기 전 추가하지 않는다.

구체 DB 제품, Harness 제품, timeout·retention·compatibility 수치는 executable evidence와 별도 freeze decision 전에는 고정하지 않는다.

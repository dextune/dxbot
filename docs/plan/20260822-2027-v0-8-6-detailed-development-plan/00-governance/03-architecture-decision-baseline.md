---
title: "v0.8.6 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---
# v0.8.6 아키텍처 결정 기준선

| ADR | 결정 |
|---|---|
| ADR-0095 | 원문 보존본은 비규범 provenance이며 active Baseline을 override하지 않는다. |
| ADR-0096 | plan version, review revision, review pass count, document version을 분리한다. |
| ADR-0097 | mutation client는 per-command journal을 durable prepare한 뒤 송신한다. |
| ADR-0098 | P0 principal은 `InstanceId + authenticated OS UID`에서 server-side로 파생한다. |
| ADR-0099 | 최초 Instance는 principal/default policy/owner authority generation을 원자적으로 생성한다. |
| ADR-0100 | Storage는 `DXB-ARC-018`의 executable spike를 통과하기 전 public trait를 freeze하지 않는다. |
| ADR-0101 | deterministic Reference Provider와 별도로 one real Harness Adapter canary를 요구한다. |
| ADR-0102 | Project/Channel membership과 Multi-Bot delegation은 typed Application operation이다. |
| ADR-0103 | Linux export는 descriptor-relative no-follow와 atomic no-replace를 사용한다. |
| ADR-0104 | graceful Runtime shutdown과 host process stop은 서로 다른 operation이다. |
| ADR-0105 | operation metadata가 parser/help/schema/error/exit snapshot의 SSOT다. |
| ADR-0106 | read-only active plan validator를 PR/main에서 실행한다. |
| ADR-0107 | `RequestDigest`는 client-visible semantic만 포함하고 `ResolvedBindingDigest`는 server-only binding을 포함한다. |
| ADR-0108 | Local Journal, Receipt, Directive, Target Lifecycle 상태기계를 분리하고 wait predicate를 operation metadata에 제한한다. |
| ADR-0109 | IdempotencyKey는 issuance epoch를 포함한다. acceptance horizon 밖 key는 binding 삭제 후에도 신규 key로 취급하지 않고 거부한다. |
| ADR-0110 | Approval, AuthorityBinding, ActionGrant는 `DXB-RUN-032`, Side Effect Ledger는 `DXB-DOM-023`이 소유한다. |
| ADR-0111 | default policy는 deny-unknown, bounded resource, no auto declassification/promotion, no Reference Provider fallback을 의미한다. |
| ADR-0112 | stable output schema와 exit code 0, 2~18 mapping을 active package 안에 정의한다. |
| ADR-0113 | P0 Dynamic Core 병렬성은 서로 다른 admitted Execution 간 병렬성으로 제한한다. |
| ADR-0114 | review revision 숫자와 review evidence 수를 분리하고 Manifest가 5회 검토와 2회 재검수를 기록한다. |

구체 DB 제품, Harness 제품, timeout·retention·compatibility 숫자는 executable evidence와 별도 freeze decision 전에는 고정하지 않는다.

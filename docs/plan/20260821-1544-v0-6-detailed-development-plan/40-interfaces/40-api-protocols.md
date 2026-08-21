---
title: "Control API와 프로토콜"
document_id: "DXB-IFC-040"
version: "0.6.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-DOM-026", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-RUN-038", "DXB-ARC-014", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032"]
---

# Control API와 프로토콜

## 1. 목적

CLI/TUI/Web/automation이 동일 schema로 v0.5 Bot/Project/Channel/Memory/Collaboration을 관리하면서 v0.6 Process/Cycle/Epistemic Memory/ActionGrant/Resource pressure 상태를 안전하게 관측·제어하게 한다. Interface가 자체 Process/Authorization logic을 재구현하지 않는다.

## 2. Durable Process / Collaboration Status

노출 후보:
- ProcessId / ProcessDefinitionVersion
- root intent/correlation refs
- current step/progress summary
- pending command/activity refs의 safe summary
- status / terminal reason / recovery-required
- CycleId / selected participant count / round-hop-activation budget summary
- observed_at / stale

내부 engine stack, provider secret, raw security token은 공개하지 않는다.

## 3. Memory API 확장

기존 `scope_ref`, MemoryId/revision/provenance/promotion API를 유지하고 권한상 허용되는 범위에서:
- Epistemic Kind
- Assertion State
- temporal validity/effective metadata
- evidence/dependency relation refs
- retraction/revalidation/quarantine status
- source trust/confidentiality label의 safe representation

을 노출한다.

API consumer가 `Verified`를 단순 score로 추론하지 않게 Canonical state를 명시한다.

## 4. Publication / Declassification

cross-scope publication/promotion 요청은 source revision, target ScopeRef, current policy generation과 필요한 approval/declassification status를 포함할 수 있다.

Private/Sensitive source는 read/write 권한만으로 Shared target commit되지 않는다. response는 `approval-required`, `denied`, `conflict`, `committed` 등 stable semantic을 구분한다. exact wire error registry는 ADR에서 freeze한다.

## 5. ActionGrant API

권한상 필요한 경우에만:
- GrantRef
- target action/resource summary
- expiry
- remaining bounded use/budget summary
- revoked/exhausted status

를 노출한다. secret/token 원문이나 내부 authorization material을 반환하지 않는다. Grant가 Membership/Role/Authority를 대체하는 것처럼 표현하지 않는다.

## 6. Resource / Memory Pressure Error Semantic

wire level은 exact code를 ADR에서 확정하되 최소 의미를 구분한다.
- resource exhausted
- memory pressure constrained/critical
- payload too large / decoded expansion rejected
- admission deferred/retryable
- budget exhausted
- stream terminated by cumulative byte cap
- recovery-required

`OOM`을 정상 retryable application error처럼 약속하지 않는다.

## 7. Streaming / Large Payload

- request/response cumulative byte cap
- in-flight/backpressure
- history pagination
- Artifact streaming/reference
- decoded/decompressed limit
- partial-result semantic이 있는 경우 명시적 상태
- slow client gap/resync

full Project/Channel history나 large process trace를 단일 response로 materialize하지 않는다.

## 8. Existing API Compatibility

- Bot-only v0.5 command에 ProjectId/ChannelId/ProcessId를 필수화하지 않는다.
- Message, Bot Network Message, Control Directive, Memory Proposal, Process Command/Status DTO를 동일 schema로 합치지 않는다.
- Role/Authority/Authorization Decision을 별도 의미로 유지한다.
- Interface Session/Provider Session을 persistent identity로 노출하지 않는다.

## 9. 검증 기준

- 신규 AT-PROC/MEM/SEC/COLLAB/RMEM Acceptance를 headless API로 재현 가능.
- Process status가 child Task/Memory state copy를 public truth로 노출하지 않음.
- revoked/declassified/grant state가 stale client mutation을 허용하지 않음.
- payload/pressure rejection이 stable semantic을 가짐.
- Bot-only v0.5 API regression 0.

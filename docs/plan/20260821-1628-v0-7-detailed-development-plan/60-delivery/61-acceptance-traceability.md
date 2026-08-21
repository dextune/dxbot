---
title: "수용 기준과 원칙 추적성"
document_id: "DXB-DEL-061"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-ARC-017", "DXB-DOM-022", "DXB-DOM-027", "DXB-DOM-028", "DXB-DOM-029", "DXB-RUN-036", "DXB-RUN-037", "DXB-RUN-038", "DXB-IFC-040", "DXB-IFC-041", "DXB-DEL-060", "DXB-ENG-052"]
---

# 수용 기준과 원칙 추적성

## 1. 목적

v0.6까지의 Backend Acceptance를 모두 regression 기준으로 유지하고, v0.7 Headless Application Contract/Runtime Host/CLI의 실행 가능성·독립성·boundedness를 Acceptance로 추가한다.

`DXB-IFC-042/043` 전용 과거 Acceptance가 있더라도 v0.7 active release gate에는 상속하지 않는다. Backend 의미를 검증하는 Interface-neutral Acceptance는 삭제하지 않고 CLI/Control Client 기준으로 재표현한다.

## 2. 기존 Acceptance 유지

v0.6의 AT-PROC-001, AT-MEM-007/008, AT-SEC-003/004, AT-COLLAB-002/003, AT-RMEM-001~003과 v0.5/v0.4/v0.3의 Project/Channel/Memory/Context/Security/Migration/Conversation/Thread/Control/Session/Bot/Brain/Core/Task/Side Effect/Routine/Network/Harness/Storage/Recovery/Observability/Module/Plugin/Repository/Policy/SPI Acceptance 의미를 삭제·축소하지 않는다.

단, 과거 Interface 이름 나열은 v0.7 active consumer 기준으로 supersede한다.

## 3. v0.7 Requirement Trace

| Requirement | Canonical Owner | Acceptance |
|---|---|---|
| FR-APP-001 Headless use-case completeness | IFC-040/Application | AT-APP-001 |
| FR-APP-002 Contract/version compatibility | IFC-040/ENG-053 | AT-APP-002 |
| FR-APP-003 Command idempotency/lost response | IFC-040/ARC-014 | AT-APP-003 |
| FR-APP-004 Subscription resume/gap | IFC-040/RUN-033 | AT-APP-004 |
| FR-CLI-001 Reference E2E | IFC-041 | AT-CLI-001 |
| NFR-CLI-002 Process independence | IFC-041/ARC-010 | AT-CLI-002 |
| NFR-CLI-003 Machine output stability | IFC-041 | AT-CLI-003 |
| NFR-CLI-004 Bounded large output | IFC-041/RUN-031/ENG-051 | AT-CLI-004 |
| NFR-CLI-005 No client policy duplication | IFC-041/RUN-032 | AT-CLI-005 |
| FR-CLI-006 Recovery/reconnect identity | IFC-041/RUN-033 | AT-CLI-006 |
| GOV-CLI-007 Active Interface scope | BASE-000/ENG-054 | AT-CLI-007 |

## 4. AT-APP-001 — Headless Contract Completeness

Given v0.6 representative Backend fixture,
When `DXB-IFC-040`만 사용하면,
Then Bot/Conversation/Thread/Project/Channel/Task/Process/Memory/Control/Runtime의 P0 use-case를 Domain internal API 없이 수행할 수 있다.

검증:
- direct store access 0
- direct Scheduler/Core mutation 0
- direct Provider call 0
- client-side Authorization/Memory policy mutation 0

## 5. AT-APP-002 — Contract Version Compatibility

Given compatible/incompatible CLI-Runtime pair,
When connect/query/command/subscribe하면,
Then:
- compatible pair semantic 유지
- incompatible pair explicit version error
- silent field reinterpretation/downgrade 0
- protocol/schema/runtime/client/data-schema version 혼합 0

## 6. AT-APP-003 — Command Idempotency / Lost Response

Given mutation이 Runtime에서 commit되었지만 response 전 connection이 끊길 때,
When 동일 logical idempotency identity로 retry하면,
Then duplicate Canonical effect가 생성되지 않고 기존 outcome/receipt를 조회 또는 reconcile할 수 있다.

## 7. AT-APP-004 — Subscription Resume / Gap

Given follow 중 disconnect 또는 cursor gap/expiry,
When reconnect하면,
Then resume 또는 explicit resync를 수행하며 silent event loss로 current state를 오판하지 않는다.

## 8. AT-CLI-001 — CLI End-to-End Reference Operation

CLI만 사용하여 최소 다음을 완결한다.
- Runtime start/status/doctor
- Bot create/chat/thread
- Task execution/control/watch
- Project/Channel collaboration
- Scoped Memory inspect/promotion
- Process/result/evidence observation
- Runtime restart/recovery

## 9. AT-CLI-002 — CLI Process Independence

Given Task/Process가 실행 중일 때,
When CLI 정상 종료/crash/SIGINT/disconnect하면,
Then explicit Runtime command가 없는 한 Bot/Task/Process lifecycle은 변경되지 않는다.

## 10. AT-CLI-003 — Machine Output Stability

Given `--json` 또는 streaming machine mode,
Then:
- stdout machine result 외 progress/log 혼입 0
- diagnostic/error는 stderr
- stable schema/exit class
- terminal width/color가 semantic을 변경하지 않음
- default redaction 유지

## 11. AT-CLI-004 — Bounded Large Output

Given long history, large Memory result, large Artifact, long-running watch,
When CLI list/export/follow를 수행하면,
Then:
- full result heap materialization 0
- paging/streaming bounded
- slow consumer unbounded Runtime queue 0
- CLI RSS가 total dataset size와 선형 동기 증가하지 않음

## 12. AT-CLI-005 — No Client Policy Duplication

Given stale Role/Authority, Memory conflict, Scheduler/Memory pressure,
When CLI command를 수행하면,
Then CLI가 local policy로 결과를 추측하지 않고 Runtime Canonical decision/error를 표시한다.

## 13. AT-CLI-006 — Runtime Recovery / Reconnect

Given Runtime restart 또는 Control Endpoint recreation,
When CLI 재접속하면,
Then same Bot/Conversation/Thread/Task/Memory/Process identity를 조회하고 Provider/CLI/control session을 Domain identity로 오인하지 않는다.

## 14. AT-CLI-007 — Active Interface Scope

v0.7 package/build/release gate에서:
- active Interface normative docs는 IFC-040/041만 존재
- IFC-042/043 파일 및 active dependency 0
- superseded Interface 전용 milestone/Acceptance/Risk/OQ가 release blocker로 남지 않음
- TUI/Web active crate/artifact 0
- Backend semantic regression 0

## 15. Cross-Layer Traceability

```text
FR-APP-001~004
→ ARC-010/014/015
→ RUN-030/031/032/033
→ IFC-040
→ ENG-052/053
→ AT-APP-001~004
→ R-078~089 / OQ-068~079

FR/NFR-CLI-001~007
→ IFC-041
→ ENG-050/051/052/054
→ DEL-060
→ AT-CLI-001~007
→ R-078~089 / OQ-068~079
```

## 16. 검증 기준

- 신규 11개 Acceptance가 Owner/Test/Risk/OQ/roadmap에 연결된다.
- 기존 Backend Acceptance semantic regression 0.
- superseded Interface 전용 gate가 active matrix에 없음.

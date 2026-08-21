---
title: "초기 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---

# 초기 아키텍처 결정 기준선

## 1. 목적

v0.8에서 구현자가 발명해서는 안 되는 P0 결정을 고정한다. 자료구조·library·세부 timeout은 측정 가능한 owner가 결정하되 public semantic과 실패 의미는 본 기준선을 따른다.

## 2. 확정 ADR

| ADR | 결정 | 상태 |
|---|---|---|
| ADR-0084 | active package는 self-contained Effective Baseline이며 과거 normative inheritance를 사용하지 않는다 | Accepted |
| ADR-0085 | P0 Reference Platform은 Linux user-scoped Runtime, data root당 active instance 1개다 | Accepted |
| ADR-0086 | local control은 user-owned Unix-domain endpoint와 same-user peer identity를 사용한다 | Accepted |
| ADR-0087 | ResourceSelector는 Canonical ID 우선, scoped exact name만 허용하고 ambiguity를 거부한다 | Accepted |
| ADR-0088 | mutation은 durable Operation Receipt와 key/digest binding으로 response-loss를 복구한다 | Accepted |
| ADR-0089 | paged Query 기본은 snapshot-consistent이며 cursor는 query digest·scope·principal·snapshot·schema에 결박된다 | Accepted |
| ADR-0090 | Subscription은 at-least-once, explicit gap/resync, JSONL terminal record를 사용한다 | Accepted |
| ADR-0091 | machine wait/timeout/partial/exit registry는 `DXB-IFC-041` 하나가 소유한다 | Accepted |
| ADR-0092 | terminal text는 기본 sanitize하고 file export는 no-follow, no-overwrite, temp+atomic rename을 사용한다 | Accepted |
| ADR-0093 | Rust Application Contract source가 schema SSOT이며 generated snapshot drift를 CI가 막는다 | Accepted |
| ADR-0094 | Bot-only Vertical Slice로 Contract kernel을 검증한 뒤 Project/Channel, Recovery/Streaming을 확장한다 | Accepted |

## 3. P0 결정 세부

- Runtime Instance directory와 lock/readiness는 `DXB-RUN-035`가 소유한다.
- receipt retention과 local journal은 `DXB-IFC-040`이 소유한다.
- exact command/exit/output는 `DXB-IFC-041`이 소유한다.
- authorization·endpoint·terminal·export 공격면은 `DXB-RUN-032`와 `DXB-IFC-041`이 분담한다.
- schema compatibility window는 `DXB-ENG-053`이 소유한다.

## 4. Deferred P1/P2 결정

다른 OS adapter, remote transport, TUI/Web, Plugin lifecycle 전체 CLI, Goal/Routine 전체 CLI, 범용 trace export, distributed Runtime은 P0 blocking decision이 아니다. P0 계약을 약화시키는 generic fallback은 허용하지 않는다.

## 5. ADR 변경 증거

변경 시 Context, alternatives, owner, invariant, compatibility, removal, security, resource, race, crash window, deterministic fixture, rollout/rollback을 기록한다. public mutation·selector·receipt·cursor·exit 의미를 바꾸는 결정은 breaking compatibility review 대상이다.

## 6. 검증 기준

- OQ-080~089의 P0 질문은 위 ADR과 owner 문서에서 `Resolved`다.
- exact library나 crate 수를 불필요하게 영구 고정하지 않는다.
- generic RPC/JSON mutation escape hatch 0

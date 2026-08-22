---
title: "v0.8.5 아키텍처 결정 기준선"
document_id: "DXB-GOV-003"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-BASE-000", "DXB-GOV-002"]
---
# v0.8.5 아키텍처 결정 기준선

| ADR | 결정 |
|---|---|
| ADR-0095 | 원문 보존본은 비규범 provenance이며 active Baseline을 override하지 않는다. |
| ADR-0096 | active plan version과 review revision, individual document version을 분리한다. |
| ADR-0097 | mutation client는 per-command durable journal을 fsync한 뒤 송신하고 `CommandId`/`IdempotencyKey`로 복구한다. `ClientRequestId`는 correlation 전용이다. |
| ADR-0098 | P0 local principal은 `InstanceId + authenticated OS UID`에서 server-side로 파생하며 client가 지정할 수 없다. |
| ADR-0099 | 최초 Instance initialization은 default Brain/Permission/Resource/Provider policy generation을 원자적으로 생성한다. |
| ADR-0100 | P0 Storage는 `DXB-ARC-018`의 single-writer atomic commit, versioned snapshot, bounded disk profile을 executable spike로 통과해야 한다. |
| ADR-0101 | deterministic Reference Provider와 별도로 one real Harness Adapter canary를 P0 release evidence로 요구한다. |
| ADR-0102 | Project/Channel membership과 Multi-Bot delegation은 typed Application operation으로 CLI에서 실행 가능해야 한다. |
| ADR-0103 | Linux safe export는 descriptor-relative no-follow, exclusive temp, atomic no-replace publication, parent fsync를 사용한다. |
| ADR-0104 | `runtime stop`은 Control graceful request가 기본이며 host-level stop은 명시적 escalation만 허용한다. |
| ADR-0105 | command path와 typed input grammar는 generated operation metadata에서 파생하며 수동 parser 계약 복제를 금지한다. |
| ADR-0106 | v0.1 one-shot write workflow를 제거하고 read-only active plan validator를 사용한다. |

세부 library, DB 제품, timeout 숫자는 위 semantic을 만족하는 executable evidence와 별도 ADR 없이 고정하지 않는다.

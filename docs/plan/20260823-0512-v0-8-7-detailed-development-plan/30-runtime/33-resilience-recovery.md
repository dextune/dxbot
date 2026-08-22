---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.8.7"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-030", "DXB-ARC-015"]
---
# 오류·복구·회복성

## Submission recovery

- `Prepared`: send가 시작되지 않았음이 durable하게 증명되며 explicit abandon 또는 original payload 재제공이 가능하다.
- `Dispatching`: send 시작 가능성이 있으므로 operation lookup 전 abandon/replay/evict 금지.
- `Observed/Terminal`: Runtime ReceiptRef가 source of truth다.

Runtime은 authenticated Principal 이후 key/Command binding lookup을 먼저 수행하므로 alias 변경, revoke, archive가 이미 commit된 retry receipt 조회를 가로막지 않는다. observation redaction/visibility는 current policy를 적용할 수 있다.

Approval pending operation은 restart 후 Approval/Operation binding을 재연결하고 decision이 있으면 재평가한다. wakeup 유실은 startup reconcile로 복구한다.

cursor/subscription/export/stale endpoint recovery는 v0.8.6 bounded/fail-closed 의미를 유지한다.

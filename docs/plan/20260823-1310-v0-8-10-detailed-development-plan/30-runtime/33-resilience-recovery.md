---
title: "오류·복구·회복성과 사용자 재진입"
document_id: "DXB-RUN-033"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-030", "DXB-ARC-015"]
---
# 오류·복구·회복성과 사용자 재진입

## Submission recovery

- `Prepared`: send가 시작되지 않았음이 durable하게 증명된다.
- `Dispatching`: first byte가 송신됐을 수 있으므로 authenticated binding lookup 전 abandon/replay/evict를 금지한다.
- `Observed/Terminal`: Runtime Receipt projection이 canonical outcome source다.

CLI startup scan은 선택된 Instance의 journal만 bounded하게 읽는다. read-only command와 help/version을 불필요하게 막지 않으며, mutation 전 관련 record의 integrity/version/lock을 확인한다.

### Prepared

동일 operation key/schema와 materialized RequestDigest를 가진 새 invocation은 lock-free `Prepared` record를 재사용해 동일 CommandId/IdempotencyKey로 Dispatching할 수 있다. payload가 다르면 재사용하지 않는다.

`Prepared`는 provably-unsent이므로 minimum retention 이후, lock-free, hash-chain-valid, unknown version이 아닌 경우에만 bounded capacity policy가 oldest eligible record를 prune할 수 있다. prune은 canonical operation 취소가 아니며 diagnostic counter를 남긴다.

### Dispatching 이상

CLI는 global CommandId와 principal-scoped IdempotencyKey를 모두 조회한다.

- stored binding 존재 → stored Receipt를 관찰한다.
- 한 index만 존재하거나 digest/counterpart가 다름 → conflict/recovery-required.
- 두 binding이 모두 absent임이 확인됨 → 동일 IDs/digest/payload로만 replay할 수 있다.
- Runtime unavailable 또는 lookup 불확실 → 자동 replay하지 않고 `operation show`/`runtime doctor --section journal` action을 반환한다.

SIGINT, SIGTERM, broken pipe, pager 종료, local timeout은 local observation 종료이며 operation cancel 또는 `Abandoned` 전이가 아니다.

## Approval과 provider recovery

Approval pending operation은 restart 후 Approval/Operation binding을 재연결하고 decision이 있으면 재평가한다. wakeup 유실은 startup reconcile로 복구한다.

provider-unavailable은 blind retry loop를 만들지 않는다. error projection은 `runtime doctor --section provider`, 현재 visible capability requirement와 retryability를 제공한다.

cursor/subscription/export/stale endpoint는 bounded, explicit gap/resync, no silent overwrite/fallback 의미를 유지한다.

---
title: "오류·복구·회복성"
document_id: "DXB-RUN-033"
version: "0.8.9"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-RUN-030", "DXB-ARC-015"]
---
# 오류·복구·회복성

## CLI journal recovery

| Durable local state | 허용 동작 |
|---|---|
| `Prepared` | payload/digest 재검증 뒤 provably-unsent abandon 또는 최초 dispatch |
| `Dispatching` | Runtime lookup 전 resend/abandon/delete 금지 |
| `Observed` | stored operation key로 Receipt 재조회 |
| `Terminal` | immutable observation; retention policy에 따른 cleanup |
| truncated final record | 마지막 완전한 hash-chain record까지 사용 |
| invalid middle record, chain mismatch, unknown version | fail closed; 자동 send/replay/delete 금지 |

append lock은 single writer다. 원 writer 종료 뒤 OS lock이 해제되면 동일 immutable header를 검증한 process가 takeover할 수 있다. 다른 UID가 journal을 읽어도 principal-bound key와 Instance-global CommandId 검증 때문에 신규 operation을 만들 수 없다.

SIGINT, broken pipe, local timeout은 observation 종료다. 별도 typed cancel/control command 없이는 Runtime operation을 취소하거나 journal을 `Abandoned`로 바꾸지 않는다.

## Runtime recovery

Runtime은 authenticate와 key-scope 검증 뒤 global CommandId/principal key binding을 먼저 조회한다. alias 변경, authority revoke, archive가 이미 commit된 retry identity를 신규 submission으로 바꾸지 않는다. Receipt projection은 current redaction/visibility를 적용할 수 있다.

full Receipt가 compact됐으면 최소 tombstone으로 same submission, terminal disposition, expiry를 판정한다. tombstone/horizon 밖 request는 typed expired/recovery-required로 끝나며 absent 신규 mutation으로 처리하지 않는다.

Approval pending operation은 restart 후 Approval/Operation binding을 재연결하고 decision이 있으면 재평가한다. wakeup 유실은 startup reconcile로 복구한다.

## Cursor, subscription, export, endpoint

- cursor는 Instance, Principal visibility, query/filter/sort/schema, snapshot에 결박되고 byte/item/expiry 상한을 가진다. gap/expiry는 silent restart가 아니라 explicit resync다.
- subscription reconnect는 last acknowledged cursor를 사용하며 gap을 숨기지 않는다.
- export crash는 same-directory temp를 cleanup할 수 있으나 destination overwrite/partial publish를 허용하지 않는다.
- endpoint generation/protocol mismatch는 fail closed하며 자동 host-stop escalation을 금지한다.

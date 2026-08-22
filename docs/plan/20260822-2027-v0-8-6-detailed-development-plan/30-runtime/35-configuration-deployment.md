---
title: "구성·Bootstrap·Default Policy·Runtime Instance·배포"
document_id: "DXB-RUN-035"
version: "0.8.6"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-011", "DXB-RUN-032"]
---
# 구성·Bootstrap·Default Policy·Runtime Instance·배포

Linux user service, XDG paths, Unix-domain socket, data root/InstanceId당 active HostGeneration 하나를 사용한다. `$XDG_RUNTIME_DIR`가 없거나 unsafe하면 endpoint를 열지 않는다.

## First initialization

exclusive initialization transaction은 다음을 함께 생성한다.

```text
InstanceId / next HostGeneration
LocalPrincipalRef mapping for owner UID
DefaultBrainPolicyRef + generation
DefaultPermissionPolicyRef + generation
DefaultResourcePolicyRef + generation
DefaultProviderSelectionPolicyRef + generation
InstanceOwner AuthorityBinding + generation
DataSchemaVersion
```

중간 crash에서는 complete 또는 idempotent resume/rollback한다.

## Default semantic

- **Brain**: 하나의 logical Brain, bounded context, Memory는 proposal/validation 없이 자동 promotion하지 않음.
- **Permission**: unknown deny, owner principal의 local administration만 명시 허용, declassification/authority issuance/high-risk side effect는 Approval 필요.
- **Resource**: 모든 queue/cache/page/stream/execution에 finite ceiling과 control/recovery headroom이 필수. 유효한 bounded default가 없으면 init 실패.
- **Provider selection**: configured Ready production Provider만 선택. 없으면 typed `provider-unavailable`/Degraded이며 Reference Provider로 fallback하지 않음.

정확한 byte/concurrency/retention 숫자는 config schema와 benchmark ADR이 소유한다.

## Readiness

`ProcessStarted → StorageRecovered → RuntimeReady → ControlReady`. `runtime start` 기본 성공은 ControlReady다.

## Stop operation 분리

- `runtime stop`: `RequestRuntimeShutdown` Application Command. Receipt commit 후 Runtime drain/checkpoint/reconcile을 Directive/Status로 관찰한다.
- `runtime stop --host-stop --host-generation <g>`: `StopRuntimeHost` Host Action. Domain Receipt가 아니라 host-action result를 반환한다.

endpoint 오류가 자동 host-stop으로 전환되지 않는다. stale generation host action은 current process에 영향이 없다.

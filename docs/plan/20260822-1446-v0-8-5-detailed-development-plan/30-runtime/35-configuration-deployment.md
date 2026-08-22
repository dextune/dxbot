---
title: "구성·Bootstrap·Runtime Instance·배포"
document_id: "DXB-RUN-035"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-ARC-011", "DXB-RUN-032"]
---
# 구성·Bootstrap·Runtime Instance·배포

## Reference

Linux user service, XDG paths, Unix-domain socket, data root/InstanceId당 active HostGeneration 하나를 사용한다. `$XDG_RUNTIME_DIR`가 없거나 unsafe하면 endpoint를 열지 않는다.

## First initialization

exclusive instance initialization transaction은 다음 durable metadata를 함께 생성한다.

```text
InstanceId
next HostGeneration
LocalPrincipalRef mapping for authenticated owner UID
DefaultBrainPolicyRef + generation
DefaultPermissionPolicyRef + generation
DefaultResourcePolicyRef + generation
DefaultProviderSelectionPolicyRef + generation
DataSchemaVersion
```

중간 crash에서는 전체 complete 또는 idempotent resume/rollback한다. profile은 위치와 출력 preference만 선택하며 Authority/default policy truth가 아니다.

## Readiness

`ProcessStarted → StorageRecovered → RuntimeReady → ControlReady`. `runtime start` 기본 성공은 ControlReady다. Application `--wait` enum을 재사용하지 않고 host 전용 `--ready-at <process|storage|runtime|control>`을 사용한다.

## Stop

- 기본 `runtime stop`: ControlReady endpoint에 `RequestRuntimeShutdown`을 보내고 durable receipt로 graceful drain/checkpoint/reconcile을 관찰한다.
- ControlReady가 아니면 exit `runtime-unavailable`과 safe discovery summary를 반환한다.
- `runtime stop --host-stop --host-generation <g>`: 명시적 host-level escalation이다. local confirmation 또는 `--yes`가 필요하고, Domain receipt가 아닌 structured host action result를 반환한다.
- 기본 경로가 endpoint 오류만으로 자동 host-stop으로 전환되어서는 안 된다.
- stale generation escalation은 current Runtime에 영향이 없다.

## Provider bootstrap

default selection policy는 configured Provider generation을 resolve한다. Reference Provider는 deterministic test용이며 production Task의 silent fallback이 아니다. real Adapter가 unavailable하면 activation/admission이 typed failure/Degraded로 귀결한다.

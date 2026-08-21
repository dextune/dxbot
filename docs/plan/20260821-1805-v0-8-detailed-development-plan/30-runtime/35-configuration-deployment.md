---
title: "구성·Runtime Instance·배포"
document_id: "DXB-RUN-035"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-RUN-032"]
---

# 구성·Runtime Instance·배포

## 1. 목적

v0.8 P0 reference deployment의 Runtime Instance, 디렉터리, endpoint, single-instance fencing, readiness, profile precedence를 구현자가 발명하지 않도록 고정한다.

## 2. P0 Reference Platform

- OS: Linux
- lifecycle: user-scoped service manager; reference adapter는 `systemd --user`
- transport: Unix-domain socket
- principal binding: local peer credential + configured DXBOT principal mapping
- instance granularity: **one active Runtime per persistent data root / InstanceId**
- multi-user/system-wide/remote Runtime: P1/P2 별도 ADR

systemd 명칭은 reference adapter 선택이며 Domain/Application semantic은 service manager에 종속되지 않는다.

## 3. Instance Identity

```text
InstanceId           durable, data root에 저장
HostGeneration       active host fencing generation
RuntimeVersion       binary version
DataSchemaVersion    persistent schema
Protocol/SchemaVersion public contract
```

InstanceId는 최초 안전한 initialization에서 생성하고 data root의 immutable metadata에 저장한다. directory 이름·PID·socket path만으로 instance identity를 판단하지 않는다.

## 4. Reference Paths

XDG 환경변수가 없으면 freedesktop 기본값을 사용하되 `$XDG_RUNTIME_DIR` 부재 시 insecure `/tmp` fallback으로 Control Endpoint를 열지 않고 명시적으로 실패한다.

```text
config:  $XDG_CONFIG_HOME/dxbot/profiles/<profile>.toml
         default ~/.config/dxbot/profiles/<profile>.toml

data:    $XDG_DATA_HOME/dxbot/instances/<instance-id>/
         default ~/.local/share/dxbot/instances/<instance-id>/

state:   $XDG_STATE_HOME/dxbot/instances/<instance-id>/
         default ~/.local/state/dxbot/instances/<instance-id>/

runtime: $XDG_RUNTIME_DIR/dxbot/<uid>/<instance-id>/  mode 0700
         ├─ control.sock
         ├─ endpoint.json
         └─ runtime.lock
```

log는 state directory 아래 bounded rotation policy를 사용한다. secret/raw Memory dump를 기본 log로 만들지 않는다.

## 5. Endpoint Descriptor

`endpoint.json`은 safe summary만 가진다.

```text
InstanceId
HostGeneration
TransportKind
EndpointRelativeName
RuntimeVersion
Protocol/Schema compatibility
Readiness
StartedAt
```

credential/secret/authority를 저장하지 않는다. descriptor owner/mode/InstanceId/HostGeneration을 검증하고 socket path는 runtime directory 밖을 가리키지 못한다.

## 6. Single Instance / Fencing

- user service manager가 service activation을 serialize한다.
- Runtime은 `runtime.lock`을 exclusive acquire하고 data root의 generation record를 atomic update한다.
- active generation만 canonical storage writer/control endpoint가 될 수 있다.
- concurrent start loser는 existing service/readiness를 관찰한다.
- stale PID/socket/descriptor는 `DXB-RUN-033` 절차로만 제거한다.

## 7. Readiness

```text
ProcessStarted
→ StorageRecovered
→ RuntimeReady
→ ControlReady
```

`runtime start` 성공 기본 조건은 `ControlReady`다. `--wait accepted` 등 host lifecycle wait options가 있더라도 process spawn만을 Domain-ready로 표시하지 않는다. timeout은 local wait 종료이며 service가 계속 startup 중일 수 있음을 structured result로 반환한다.

## 8. Stop / Restart

`runtime stop`은 가능한 경우 Control Contract의 graceful shutdown request를 전달하고 신규 admission 차단 → drain/checkpoint/reconcile → endpoint close → service exit 순으로 진행한다. Runtime이 ControlReady가 아니면 Host Lifecycle Adapter가 service stop을 요청할 수 있다. 강제 kill은 명시된 escalation이며 Task cancel/reconcile semantic의 대체가 아니다.

stale stop/restart는 InstanceId+HostGeneration을 확인한다.

## 9. Profile Precedence

```text
explicit CLI --profile/--instance
→ selected profile
→ safe deployment defaults
```

profile은 endpoint/data/config 위치와 output preference를 선택할 뿐 Bot/Project/Channel state, Authority, Provider selection truth, resource decision을 소유하지 않는다. profile과 endpoint의 InstanceId가 다르면 explicit `instance-mismatch`다.

## 10. Config Generation

Runtime config는 typed validation → immutable generation → new admission 적용 → old generation drain을 따른다. running Execution snapshot을 rewrite하지 않는다. security/resource revoke는 current authorization/admission path에서 강화한다.

## 11. 검증 기준

- same data root concurrent start에서 writer/endpoint 1개.
- 다른 data root/InstanceId endpoint로 silent 연결 0.
- `$XDG_RUNTIME_DIR` 부재 또는 unsafe permission에서 fail closed.
- readiness 단계가 process status와 Domain readiness를 구분.
- stale generation stop/descriptor가 current Runtime에 영향 0.
- Runtime restart 후 Domain identity 유지.

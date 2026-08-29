---
title: "구성·Bootstrap·Default Policy·Runtime Instance·배포"
document_id: "DXB-RUN-035"
version: "0.8.10"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-23"
depends_on: ["DXB-ARC-011", "DXB-RUN-032"]
---
# 구성·Bootstrap·Default Policy·Runtime Instance·배포

## Local Instance selection

profile과 `--instance`는 local endpoint discovery hint이며 Authority가 아니다. CLI는 endpoint descriptor와 authenticated peer를 검증해 최종 InstanceId/HostGeneration을 얻는다.

선택 순서는 다음 하나다.

```text
explicit --instance
→ explicit/selected profile binding
→ exactly one verified local endpoint
→ runtime start에서만 no-existing-instance default bootstrap
```

0개이면 typed runtime-unavailable, 2개 이상이면 ambiguous-target이다. stale profile, endpoint failure, permission failure가 다른 Instance로 silent fallback하지 않는다.

profile state는 owner-only directory에서 descriptor-relative no-follow, temp+fsync+atomic replace로 갱신한다. `runtime start`가 endpoint authentication과 InstanceId 확인에 성공한 뒤에만 default binding을 기록한다. 실패한 bootstrap이 half-written profile을 남기지 않는다.

## First init

기존 verified Instance가 전혀 없는 `dxb runtime start`는 deterministic user-scoped data root에 safe default Instance를 생성할 수 있다. first init은 InstanceId, HostGeneration, LocalPrincipal, default Brain/Permission/Resource/Provider policy generations, owner AuthorityBinding, DataSchemaVersion을 원자적으로 생성한다.

다른 command는 Runtime을 자동 시작하지 않는다. `dxb`, help와 local client version은 Runtime 없이 동작하고 Runtime command 실패는 `runtime start/status/doctor` next action을 제공한다.

## Provider readiness

Bot creation은 Provider readiness와 분리한다. configured Ready production Provider가 없으면 Bot은 Inactive로 생성될 수 있고 Activate/Task admission에서 typed `provider-unavailable`을 반환한다. Reference Provider는 explicit test profile/policy에서만 선택한다.

M5 provider list/show가 없어도 M2 `runtime doctor --section provider`는 selected Instance의 capability readiness를 redacted하게 진단할 수 있어야 한다.

## Stop semantics

`runtime stop` graceful command는 HostGeneration을 argv로 요구하지 않는다. Control client가 verified endpoint descriptor에서 InstanceId/HostGeneration을 materialize하고 server가 current generation을 fence한다.

explicit `--host-stop`도 descriptor에서 generation을 local resolve하며 optional `--if-host-generation`은 automation CAS다. endpoint/descriptor failure가 자동 escalation이나 다른 process signal로 바뀌지 않는다.

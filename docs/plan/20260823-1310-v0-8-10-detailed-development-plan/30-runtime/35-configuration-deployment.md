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

## Production Provider configuration

`ADR-0138`에 따라 P0 config owner는 Runtime Host의 user-scoped runtime root에 있는 owner-only `provider-config.json`이다. 이 파일은 schema version, adapter, ProviderId, capability, ProviderGeneration, endpoint, model, bounded timeout, `env:<NAME>` credential reference만 저장한다. credential 원문은 파일·discovery·Application state·CLI journal·audit에 저장하지 않으며 Runtime Host process 환경에서 reference를 일시 resolve해 Common `HttpTransport`에만 전달한다.

schema v1의 production adapter allowlist와 wire contract는 다음과 같다.

- `deepseek-flash`: OpenAI-compatible Chat Completions, Bearer credential, configured endpoint에 `/v1/chat/completions`를 추가한다.
- `minimax-m3`: Anthropic-compatible Messages, scoped `x-api-key` + `anthropic-version: 2023-06-01`, configured API base(예: `https://api.minimax.io/anthropic/v1`)에 `/messages`를 추가한다. 기본 model은 `MiniMax-M3`이며 credential reference는 `env:MINIMAX_API_KEY`를 사용할 수 있다.

adapter가 wire format과 credential header를 명시적으로 선택하며 endpoint 문자열이나 model 이름으로 protocol을 추측하지 않는다. Anthropic P0 path는 non-streaming이며 unsupported stream을 silent downgrade하지 않는다.

파일 부재는 explicit `unconfigured`이며 Activate/Task admission은 typed `provider-unavailable`로 fail closed한다. symlink/non-regular file, owner 외 권한, unknown schema/field/adapter, invalid generation/endpoint/limit, unresolved credential reference는 Runtime start를 실패시킨다. Runtime Host는 구성한 **동일 ProviderHost instance**를 Control query/admission/execution에 전달하고 discovery/doctor는 그 owner state의 redacted ProviderId/readiness projection만 게시한다. ReferenceProvider는 production fallback으로 등록하지 않는다. P0는 단일 configured Provider만 허용하며 multi-provider registry, persistent credential vault, reload/replace는 별도 generation-drain 결정 전 비범위다.

## Provider readiness

Bot creation은 Provider readiness와 분리한다. configured Ready production Provider가 없으면 Bot은 Inactive로 생성될 수 있고 Activate/Task admission에서 typed `provider-unavailable`을 반환한다. Reference Provider는 explicit test profile/policy에서만 선택한다.

M5 provider list/show가 없어도 M2 `runtime doctor --section provider`는 selected Instance의 capability readiness를 redacted하게 진단할 수 있어야 한다.

## Supported supervisor service

`ADR-0140`에 따라 Linux P0 service profile은 `deploy/systemd/dxbot-runtime.service`와 `docs/runbooks/runtime-service.md`가 소유한다. unit의 foreground `dxb __runtime-host` token은 packaging 전용 service entrypoint이며 public 63-command CLI registry를 확장하지 않는다. 일반 사용자·automation은 `systemctl --user`와 authenticated `dxb runtime status/doctor/stop`만 사용한다.

service manager는 owner-only state/config 경계, `XDG_STATE_HOME`, optional owner-only credential environment file, `Restart=on-failure`, bounded start/stop timeout을 제공한다. `ExecStop`은 signal 선행 종료가 아니라 authenticated HostGeneration-fenced `runtime stop --host-stop`을 호출한다. Runtime은 admission stop → active Provider cancel/drain 및 Core Lease 0 → final audit projection/checkpoint → endpoint/discovery/PID unpublish → process exit/lock handoff 순서를 지킨다. 강제 SIGKILL은 supervisor timeout의 fallback이며 다음 start에서 uncertain activity를 `RecoveryRequired`로 복구하고 blind redispatch하지 않는다.

반복 restart에서 동일 InstanceId, 증가하는 HostGeneration, bounded 10-second single-writer lock handoff, stale endpoint/PID 제거를 요구한다. offline backup/restore/migration은 같은 host lock을 획득해야 하며 live service와 병행 실행할 수 없다.

## Stop semantics

`runtime stop` graceful command는 HostGeneration을 argv로 요구하지 않는다. Control client가 verified endpoint descriptor에서 InstanceId/HostGeneration을 materialize하고 server가 current generation을 fence한다.

explicit `--host-stop`도 descriptor에서 generation을 local resolve하며 optional `--if-host-generation`은 automation CAS다. endpoint/descriptor failure가 자동 escalation이나 다른 process signal로 바뀌지 않는다.

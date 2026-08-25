---
title: "로컬 실행 격리와 Sandbox lifecycle 보완 계획"
document_id: "DXB-ADP-021"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P1"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010", "DXB-ADP-020"]
target_owners: ["runtime-security", "runtime-host", "runtime-bootstrap", "provider-host"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 로컬 실행 격리와 Sandbox lifecycle 보완 계획

## 1. 판단

**분류: Observe → Close.** Grok의 local Docker connector에서 차용할 것은 Docker 명령이나 image가 아니라 다음 운영 invariant다.

- 소유권 label/identity 확인 후에만 start/stop/replace
- loopback-only endpoint
- content-addressed runtime artifact
- read-only runtime mount
- readiness probe 후 연결
- schema/artifact drift 시 명시적 replace
- unowned resource 조작 거부
- bounded startup timeout과 실패 diagnostics

DXBOT은 Docker를 필수 채택하지 않는다. 먼저 container/process/VM을 포괄하는 generic isolation contract를 정의한다.

## 2. 목표 경계

```text
Application Task/Execution
        ↓ capability request
Runtime Scheduler
        ↓ admitted ResourceGrant
Sandbox Host
        ↓ isolated process/container/VM
Provider/Tool execution
        ↓ Result/Evidence
Application commit
```

Application과 Provider는 Docker socket, process namespace, mount path를 알지 않는다.

## 3. Canonical type 방향

private prototype 기준:

```text
SandboxSpec
- sandbox_kind
- owner_instance_id
- execution_id
- host_generation
- runtime_artifact_digest
- image_or_runtime_ref
- network_policy_ref
- mount_policy_ref
- secret_grant_ref
- resource_limits
- startup_deadline
- execution_deadline
- cleanup_policy

SandboxHandle
- sandbox_id
- generation
- observed_runtime_digest
- endpoint_ref
- state
```

`SandboxHandle`은 Execution-local runtime state이며 Bot Identity나 Memory owner가 아니다.

## 4. Lifecycle

```text
Requested
→ Validating
→ Creating | ReusingVerified
→ Starting
→ Ready
→ Active
→ Draining
→ Stopped
→ Cleaning
→ Removed

Validating/Creating/Starting → Rejected | Failed
Active → Lost | Fenced
```

### 4.1 Reuse 조건

기존 sandbox 재사용은 아래가 모두 일치할 때만 허용한다.

- owner instance/host generation
- sandbox schema version
- runtime artifact digest
- security profile generation
- mount/network policy generation
- secret grant validity
- resource profile compatibility

하나라도 다르면 silent reuse하지 않고 replace 또는 reject한다.

### 4.2 Ownership fencing

- 이름만 같은 외부 process/container를 조작하지 않는다.
- owner token/label과 generation이 일치하지 않으면 stop/remove를 거부한다.
- stale host가 새 generation의 sandbox를 중지하지 못한다.
- cleanup orphan scan도 ownership proof를 요구한다.

## 5. Artifact와 mount

### 5.1 Runtime artifact

- 실행 binary/script는 SHA-256 이상 digest와 size를 기록한다.
- content-addressed path에 저장한다.
- 기존 파일 digest가 다르면 덮어쓰지 않고 corruption으로 처리한다.
- 실행 시 read-only로 제공한다.
- source commit/build profile/toolchain을 artifact manifest에 연결한다.

### 5.2 Mount policy

기본값은 deny다.

| 종류 | 기본 |
|---|---|
| Runtime artifact | read-only |
| Execution workspace | isolated read-write, execution-scoped |
| Bot canonical store | 직접 mount 금지 |
| Host credential directory | 전체 mount 금지 |
| Secret | 최소 파일/FD/env lease, 만료·회수 가능 |
| User project path | explicit ActionGrant와 path policy 필요 |

Grok처럼 전체 `.codex` 또는 `.claude` 디렉터리를 편의상 mount하는 방식은 DXBOT 기본안으로 채택하지 않는다.

## 6. Network policy

- control/gateway endpoint는 기본 loopback 또는 private IPC로 제한한다.
- sandbox outbound network는 capability별 allow/deny를 가진다.
- host bind address와 published port는 runtime-security가 검증한다.
- ephemeral token은 최소 권한, expiry, audience, generation binding을 가진다.
- health endpoint와 control endpoint의 권한을 분리한다.

## 7. Secret lifecycle

```text
Secret reference
→ policy authorization
→ execution-scoped SecretGrant
→ materialize minimal credential
→ sandbox consume
→ revoke/expire
→ secure cleanup evidence
```

- raw secret은 receipt, audit, diagnostic, projection에 복사하지 않는다.
- secret materialization 실패를 retry로 무한 반복하지 않는다.
- provider refresh token 소유권은 provider/security owner가 결정한다.
- sandbox는 host credential 원본을 갱신하지 않는다.

## 8. Readiness와 failure

Ready 조건:

1. expected owner/generation 확인
2. expected runtime digest 확인
3. endpoint authentication 성공
4. resource limit 적용 확인
5. required mounts/network policy 확인
6. health probe 성공

startup poll은 bounded backoff와 absolute deadline을 사용한다. 고정 1초 polling을 contract로 만들지 않는다.

실패 시 bounded evidence:

- sandbox state/reason code
- expected/observed generation·digest
- 마지막 N개 redacted lifecycle event
- truncated process/container log digest와 safe excerpt
- cleanup attempted/result

## 9. 구현 단계

1. Docker 비종속 `SandboxSpec`/lifecycle을 문서와 private type으로 정의한다.
2. local subprocess sandbox로 ownership/generation/cancel/cleanup을 먼저 증명한다.
3. Provider Host canary 하나를 sandbox 경계로 실행한다.
4. fault test 후 필요할 때만 Docker adapter를 추가한다.
5. remote execution도 같은 `SandboxCapability` 의미를 구현하게 한다.

## 10. Acceptance

- unowned resource는 start/stop/remove하지 않는다.
- artifact digest mismatch에서 실행하지 않는다.
- stale generation이 새 sandbox를 조작하지 못한다.
- canonical store와 전체 credential directory가 mount되지 않는다.
- cancellation/deadline 뒤 process, permit, temp secret, workspace가 누수되지 않는다.
- crash 후 orphan은 ownership proof를 거쳐 bounded cleanup된다.
- local/remote 실행이 동일 Application Result/Evidence contract를 사용한다.

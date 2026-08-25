---
title: "Acceptance와 Test 계획"
document_id: "DXB-ADP-050"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-040"]
target_owners: ["quality", "runtime-host", "runtime-security", "provider-host", "application"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Acceptance와 Test 계획

## 1. 목적

문서의 정교함이 실제 실행 가능성을 대신하지 못하도록 모든 차용 항목을 fault evidence와 resource bound로 판정한다.

## 2. 공통 Gate

모든 단계에 적용한다.

1. `cargo fmt --check`
2. workspace lint/clippy
3. 영향을 받는 crate unit/integration test
4. public schema snapshot 또는 golden 영향 확인
5. docs active-plan validator
6. secret/redaction test
7. Structural/Consistency Review
8. Cross-Layer Executability Review

실제 명령은 저장소의 CI와 AGENTS Guide를 따른다.

## 3. Extension composition 테스트

| ID | 시나리오 | 기대 결과 |
|---|---|---|
| EXT-01 | duplicate ID | start 전 거부 |
| EXT-02 | missing dependency | start 전 거부 |
| EXT-03 | self dependency | start 전 거부 |
| EXT-04 | cycle | deterministic cycle evidence와 거부 |
| EXT-05 | 입력 순서 permutation | 동일 resolved order |
| EXT-06 | N번째 start 실패 | 1..N-1 정확히 한 번 역순 teardown |
| EXT-07 | teardown 하나 실패 | 나머지 teardown 계속, bounded summary |
| EXT-08 | stop 재호출 | idempotent, 중복 side effect 없음 |
| EXT-09 | readiness 전 endpoint probe | endpoint 미노출 또는 not-ready |
| EXT-10 | drain 중 신규 command | admission 거부 |

## 4. Diagnostics/backstop 테스트

| ID | 시나리오 | 기대 결과 |
|---|---|---|
| DIA-01 | secret-like error payload | persistence 전 redaction |
| DIA-02 | oversized stack/log | byte cap과 truncated flag |
| DIA-03 | sink unavailable | canonical operation 판정 불변 |
| DIA-04 | low severity flood | bounded queue, dropped watermark |
| DIA-05 | corrupt projection | quarantine/degraded, default truth 반환 금지 |
| DIA-06 | stale watermark | partial/stale 명시 |
| DIA-07 | restart | health projection 재생성 |

## 5. Sandbox 테스트

| ID | 시나리오 | 기대 결과 |
|---|---|---|
| SBOX-01 | unowned resource 동일 이름 | stop/remove 거부 |
| SBOX-02 | stale host generation | 새 resource 조작 거부 |
| SBOX-03 | runtime digest mismatch | 실행 금지 |
| SBOX-04 | canonical store mount 요청 | policy 거부 |
| SBOX-05 | whole credential dir mount | 기본 거부 |
| SBOX-06 | startup timeout | bounded failure와 cleanup |
| SBOX-07 | active process cancellation | deadline 내 종료, permit/secret 정리 |
| SBOX-08 | host crash 후 orphan | ownership proof 후 bounded cleanup |
| SBOX-09 | network policy violation | connection 차단과 safe diagnostic |

## 6. Provider conformance 테스트

동일 fixture를 HTTP와 subprocess adapter에 적용한다.

- startup/readiness
- one-shot response
- streaming order와 terminal exactly-once
- cancellation before first token
- cancellation during stream
- absolute deadline
- malformed frame
- provider rejection/rate limit
- process crash/connection reset
- credential unavailable/expired
- tool request authorization rejection
- usage reported/partial/unknown
- retry duplicate side-effect guard
- adapter stop/restart generation

## 7. Multi-Bot Message/Wake 테스트

| ID | 시나리오 | 기대 결과 |
|---|---|---|
| MSG-01 | send success | Message commit acknowledgement, reply wait 없음 |
| MSG-02 | response loss 후 retry | 동일 Message 하나 |
| MSG-03 | delivery 후 crash, wake 전 | restart 후 wake effect 하나 |
| MSG-04 | wake 후 crash, marker 전 | duplicate Execution 방지 |
| MSG-05 | recipient suspended | Message 상태와 wake rejection 분리 |
| MSG-06 | burst messages | policy에 따른 bounded coalescing |
| MSG-07 | auto reply loop | causal hop/depth bound |
| MSG-08 | Channel large membership | participant/byte/deadline bound |
| MSG-09 | membership revision race | pinned membership semantics |
| MSG-10 | cross-label message | information-flow policy 적용 |

## 8. Projection 테스트

- generation temp write 중 crash
- manifest write 전 crash
- current pointer publish 전/후 crash
- projection file manual edit
- digest mismatch
- schema mismatch
- canonical revision advance 후 stale projection
- private Memory redaction
- deletion 후 rebuild
- local safe reader when runtime unavailable

## 9. Artifact provenance 테스트

- expected digest success
- one-byte tamper
- expected size mismatch
- partial download
- symlink/path traversal
- archive bomb item/byte/depth bound
- existing digest path with different bytes
- stale compatibility generation
- source unavailable + verified cache policy
- release archive secret/cache exclusion

## 10. Resource/Performance 기준

절대 수치는 해당 owner benchmark에서 결정하되 다음 항목을 측정한다.

- extension graph resolution O(V+E) 수준과 allocation count
- diagnostic queue item/byte cap
- projection generation peak memory
- provider stream buffer peak memory
- sandbox startup/cleanup deadline
- Channel fan-out batch size와 heap ceiling
- artifact hashing streaming memory

성능을 이유로 correctness check를 생략하지 않는다. 값은 benchmark와 production profile이 정해지기 전 public contract로 고정하지 않는다.

## 11. 완료 Evidence

각 단계 PR/commit에는 최소 다음을 남긴다.

```text
- owner 및 변경 분류
- 정상 경로 test IDs
- fault test IDs
- resource bound
- public contract 영향: none/changed
- migration/removal 영향
- docs/code/schema consistency 결과
- review 1 결과
- review 2 결과
```

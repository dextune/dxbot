---
title: "테스트·검증 전략"
document_id: "DXB-ENG-052"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-014", "DXB-ARC-013", "DXB-ENG-050"]
---


# 테스트·검증 전략

## 1. 목적

상태기계, 동시성, 복구, Harness 교체, 메모리 효율, 보안을 자동화된 증거로 검증하며 모델의 비결정성 때문에 핵심 Runtime 검증이 흔들리지 않게 한다.

## 2. 책임 범위

- test pyramid와 fixture
- deterministic environment
- property/model/fuzz/concurrency
- integration/e2e/conformance
- performance/soak/fault injection
- coverage와 release gate

## 3. 테스트 계층

| 계층 | 대상 | 외부 의존성 |
|---|---|---|
| Unit | 값 타입, 상태 전이, policy | 없음 |
| Property/Model | 상태기계 불변조건, graph | 없음 |
| Concurrency Model | race/lock/channel | 없음/가상 |
| Component | store, scheduler, memory, adapter | local fixture |
| Contract/Conformance | Provider/Transport/Storage | provider fixture |
| Integration | Runtime+DB+Fake Harness+CLI | local process |
| E2E | 실제 Harness/Model/Tool | opt-in |
| Fault/Recovery | kill, disk, network, malformed | controlled |
| Performance | latency/RSS/allocation | fixed env |
| Soak | 장시간 누수/회복 | fixed env |
| Security | sandbox, secret, injection, auth | isolated |

## 4. Deterministic Testkit

`dxb-testkit` 제공:
- fake/virtual Clock
- deterministic IdGenerator/RNG
- scripted Harness
- fake Model/Tool/Sandbox
- in-memory 또는 temp transactional store
- event recorder
- scheduler executor
- network fault transport
- test Artifact store
- policy/permission fixtures
- crash point/failpoint
- invariant checker

Production code가 system clock, random, global env를 직접 읽지 않게 Port를 주입한다.

## 5. 상태기계 테스트

각 Aggregate마다:
- 모든 유효 전이
- 모든 금지 전이
- idempotent command
- expected revision conflict
- terminal state 불변
- serialization/replay
- snapshot equivalence
- unknown version
- cancellation/completion race
- delete/update race

Property test는 임의 Command sequence 후 invariant를 검사한다.

## 6. 핵심 불변조건

예:
- BotId와 Memory namespace는 재시작 후 동일
- Session 삭제가 Bot/Memory를 삭제하지 않음
- terminal Task는 하나의 terminal decision만 가짐
- Core Lease 없이 result commit 불가
- expired fencing token write 불가
- Memory tombstone 후 recall 불가
- Core Working Context 간 직접 공유 없음
- global/per-Bot Core limit 초과 없음
- outbox event가 Domain commit 없이 존재하지 않음
- secret 원문이 event/log/trace에 없음
- Provider 교체가 Domain 의미를 바꾸지 않음

## 7. Harness Conformance

모든 Harness Adapter에 동일 scenario:
- single response
- streaming chunks
- multiple tool calls
- malformed chunk
- provider error as stream/throw/process exit
- cancellation before/after start
- timeout + partial output
- large output spill
- approval
- resume/checkpoint
- unknown event
- shutdown
- stdout protocol contamination(sidecar)
- secret env scrub

결과는 stable `ExecutionOutcome`과 trace invariants로 비교한다.

## 8. Storage/Recovery Test

- atomic command/event/state/outbox
- commit response loss
- process kill at failpoints
- DB busy/locked
- disk full/read-only
- migration crash/restart
- snapshot corruption
- backup/restore
- projection/index rebuild
- orphan Artifact
- outbox poison
- concurrent revision conflict

filesystem kill test는 실제 process를 사용하고 unit mock으로 대체하지 않는다.

## 9. Concurrency Test

- `loom` 또는 동등한 도구로 atomics/locks/channel 핵심 조합
- randomized scheduler stress
- cancel/complete race
- provider unload/start race
- deactivate/task submit race
- memory update/delete race
- lease expiry/result race
- shutdown/callback race
- permit leak
- mailbox overflow
- slow consumer

테스트가 timing sleep에 의존하지 않고 barriers/fake clock/event probes를 사용한다.

## 10. Security Test

- path traversal, symlink/junction, TOCTOU
- environment secret canary
- log/trace redaction
- approval digest replay
- revoked grant
- Bot delegation confused deputy
- untrusted content instruction attempt
- XSS/rendering(Web)
- malformed/decompression bomb
- sandbox provider failure fallback 금지
- plugin manifest/capability mismatch
- audit sink failure
- rate/cost abuse

실제 host에 위험한 side effect가 나가지 않도록 격리 fixture를 사용한다.

## 11. CLI/API/UI Test

### API
- schema compatibility
- idempotency
- cursor resume/gap
- authorization filtering
- error codes
- size limits

### CLI
- stdout/stderr separation
- JSON/NDJSON snapshots
- non-TTY no prompt
- exit codes
- Ctrl-C semantics
- server mismatch

### TUI/Web
- reducer duplicate/gap
- reconnect
- permission revoke
- large list/graph
- accessibility/security
- UI 종료가 Runtime에 영향 없음

## 12. 모델 비결정성 분리

P0 CI는 Fake Harness를 사용한다. 실제 모델 E2E:
- 별도 suite
- provider/version/prompt/config 기록
- deterministic expectation보다 contract/invariant 평가
- 비용/횟수 상한
- flaky result를 core gate로 사용하지 않음
- golden trace는 구조와 필수 event를 비교
- 성능 baseline에서 모델 latency를 Runtime overhead와 분리

## 13. Coverage와 Mutation

- line coverage만 목표로 하지 않음
- Domain state machine/permission/serialization은 높은 branch coverage
- critical invariant에 mutation testing 후보
- generated/error branches의 현실성 구분
- coverage 제외 사유 명시
- test가 implementation detail보다 public contract 검증
- panic/cleanup paths 포함

## 14. Release Gate

P0:
- format/lint/type/build
- unit/property
- storage integration
- fake harness e2e
- migration/replay
- security smoke
- performance smoke
- docs/traceability

P1:
- concurrency model
- full fault matrix
- sandbox platform matrix
- actual Harness canary
- soak
- API compatibility

## 15. 예외상황

- flaky test: quarantine로 무기한 방치 금지, owner/원인/만료
- external provider outage: canary skip를 명확히 표시, core gate는 Fake 유지
- platform-specific sandbox: capability matrix와 unsupported fail-closed
- benchmark noise: rerun policy/isolated runner
- test fixture가 production path 우회: 실제 entry path test 추가
- mock가 오류 정상화를 숨김: raw provider contract fixture 포함
- snapshot 남용: 의미 assertion 병행

## 16. 확장성

remote worker/cluster는 동일 conformance suite를 transport/provider에 적용한다. event replay dataset을 버전별로 보존한다. third-party plugin SDK는 certification suite를 제공하되 보안을 보증한다고 표현하지 않는다.

## 17. 구현 우선순위

- **P0:** testkit, state/property, storage recovery, fake Harness, CLI E2E
- **P1:** concurrency/fault/security/perf/actual Harness
- **P2:** distributed chaos, multi-node migration, plugin certification
- **P3:** 자동 scenario generation

## 18. 검증 기준

- 16개 최상위 원칙마다 수용 테스트 ID가 있다.
- 핵심 race가 sleep 기반이 아닌 deterministic test를 가진다.
- 실제 process kill 이후 replay/recovery 테스트가 통과한다.
- Fake와 DeepSeek Adapter가 같은 conformance suite를 통과한다.
- secret/path/approval security tests가 release gate다.
- flaky test 비율과 quarantine age가 관측된다.

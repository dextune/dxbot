---
title: "Rust 구현 규칙"
document_id: "DXB-ENG-050"
version: "0.7.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-030"]
---

# Rust 구현 규칙

## 1. 목적

v0.6의 공통성·재사용성·메모리/캐시 효율·동시성 안전·Provider dependency firewall을 유지하고, v0.7 Application Contract/Control/CLI 구현이 Backend shortcut이나 unbounded allocation을 만들지 않도록 한다.

## 2. 공통 구현 불변조건

기존 규칙에 더해 다음을 P0로 강제한다.

1. `dxb-cli`는 Domain/Runtime/Storage/Provider internal module을 import하지 않는다.
2. Runtime이 Ready인 뒤 모든 Domain mutation/query/subscription은 Control Client/Application Contract만 사용한다.
3. Runtime bootstrap은 narrow Host Lifecycle Client만 예외이며 process/service start/status/readiness/stop orchestration만 수행한다.
4. Host Lifecycle Client가 Bot/Task/Memory/Scheduler/Provider state를 직접 읽거나 mutate하지 않는다.
5. Wire DTO, Domain struct, Persistence row를 동일 타입으로 공유하지 않는다.
6. CLI local cancellation token을 Runtime Task/Execution cancellation token과 공유하지 않는다.
7. CLI watch/follow spawned task는 명시적 owner/cancel/join을 가진다.
8. stdout/stderr writer는 unbounded `String`/`Vec` aggregation을 기본으로 사용하지 않는다.
9. paging/streaming writer는 page/chunk가 출력된 뒤 메모리를 release할 수 있어야 한다.
10. large immutable DTO/payload는 필요할 때 shared immutable ref/borrow/stream을 사용하며 불필요한 clone을 피한다.
11. CLI convenience를 위해 Common authorization/resource/retry/memory policy를 재구현하지 않는다.
12. TUI/Web 전용 crate/dependency/module을 v0.7 active workspace에 추가하지 않는다.

## 3. Dependency Firewall

허용 예:

```text
dxb-cli → public contract values
dxb-cli → control client
dxb-cli → narrow host lifecycle client
dxb-control → application port/public mapping
dxb-runtime → provider host
```

금지:

```text
dxb-cli → dxb-domain internals
dxb-cli → dxb-runtime internals
dxb-cli → dxb-storage internals
dxb-cli → concrete Provider
dxb-control → Domain Store direct mutation bypassing Application
host lifecycle client → Domain/Task/Memory/Scheduler/Provider mutator
dxb-control DTO = persistence row
```

CI architecture test로 검출한다.

## 4. Host Lifecycle Client 구현 규칙

Host Lifecycle Client는 Domain API의 우회 경로가 아니다.

허용:
- OS service manager / launcher 호출
- process/service start/status/stop 요청
- endpoint/readiness discovery
- platform error를 stable host-lifecycle error로 mapping

금지:
- Runtime internal crate link로 Bot/Task/Memory 조작
- DB/store 직접 open으로 runtime status 조립
- process handle을 Task/Execution cancellation handle로 사용
- service manager kill을 Domain cancel/reconcile의 대체로 사용

Runtime Ready 이후 Domain state/status/doctor는 `DXB-IFC-040` Query/Command를 사용한다.

## 5. Async / Cancellation

- fire-and-forget/unbounded channel 금지
- CLI local deadline/timeout은 request wait를 끝낼 수 있으나 committed Runtime mutation을 rollback했다고 추측하지 않음
- watch SIGINT는 local stream task를 종료하고 target Task/Process cancel Command를 생성하지 않음
- server subscriber와 client receiver 모두 drop/unregister cleanup
- broken pipe 후 producer task가 orphan되지 않음
- host lifecycle operation timeout과 Domain operation timeout을 동일 token/state로 결합하지 않음

## 6. Serialization / Output

- human rendering과 machine serialization 경로를 분리
- JSON/JSONL machine output에 progress/log 혼입 금지
- huge JSON array 생성보다 incremental JSONL/paged output 우선
- serialization intermediate copy를 측정하고 줄임
- terminal color/width formatting을 machine DTO에 침투시키지 않음

## 7. Error / Retry

- stable typed Application error를 CLI exit class로 mapping
- host lifecycle error와 Runtime/Application error를 구분
- 문자열 matching으로 retry/permission/conflict 판정 금지
- response-loss retry는 기존 IdempotencyKey 유지
- stale conflict 자동 overwrite 금지
- Unknown Side Effect를 transient retry로 downgrade 금지

## 8. Review Checklist

- Canonical Owner/quality tier가 명확한가
- Application Contract/Control Client를 우회하지 않는가
- Host Lifecycle Client가 bootstrap 범위를 넘지 않는가
- forbidden dependency가 없는가
- public DTO가 Domain/Persistence type을 누출하지 않는가
- clone/allocation/channel byte cap이 정당한가
- local cancel과 Runtime cancel이 분리되는가
- machine output/exit class가 안정적인가
- large output이 bounded streaming인가
- test/Acceptance/Risk/Migration이 갱신됐는가

## 9. 검증 기준

- CLI forbidden dependency 0.
- Host Lifecycle Client→Domain mutation/internal dependency 0.
- Runtime Ready 이후 Domain use-case의 Control Contract bypass 0.
- unbounded stdout/result aggregation 0.
- watch/reconnect/broken-pipe task/resource leak 0.
- Provider Framework/Host/Conformance 규칙 regression 0.
- hot serialization/stream benchmark가 allocation count/bytes를 기록한다.

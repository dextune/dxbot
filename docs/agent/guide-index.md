# DXBOT Agent Guide Index

이 문서는 Agent가 작업에 필요한 규칙만 단계적으로 탐색하기 위한 최상위 Router다. 규칙의 상세 설명을 반복하지 않는다.

## 사용 방법

1. 현재 작업을 아래 유형에서 찾는다.
2. 해당 category `index.md`를 읽는다.
3. category index가 지정한 상세 Guide만 읽는다.
4. 제품 의미가 필요한 경우 상세 Guide의 `Canonical Contracts`를 따라 활성 `docs/plan/`의 Owner 문서를 확인한다.

`docs/agent/` 전체를 선행 로드하는 방식은 권장하지 않는다.

## Task Router

| 작업 | 먼저 읽을 문서 |
|---|---|
| 파일/디렉터리 추가·이동·삭제 | [repository/index.md](repository/index.md) |
| workspace/crate/module 배치 | [repository/index.md](repository/index.md) → [architecture/index.md](architecture/index.md) |
| 새 Domain/상태/정책 | [architecture/index.md](architecture/index.md) |
| 새 Capability/Provider/Plugin | [architecture/index.md](architecture/index.md) |
| 공통화/refactor | [architecture/common-extension.md](architecture/common-extension.md) |
| Rust public type/API | [rust/index.md](rust/index.md) |
| clone/allocation/large payload | [rust/memory-allocation.md](rust/memory-allocation.md) |
| async/spawn/lock/channel | [rust/concurrency-async.md](rust/concurrency-async.md) → [rust/channels-backpressure.md](rust/channels-backpressure.md) |
| cache/index/projection | [rust/cache-derived-state.md](rust/cache-derived-state.md) |
| error/retry/side effect | [rust/error-retry-side-effects.md](rust/error-retry-side-effects.md) |
| Runtime lifecycle/shutdown | [runtime/index.md](runtime/index.md) |
| Storage/migration/recovery | [runtime/persistence-recovery.md](runtime/persistence-recovery.md) |
| permission/approval/secret | [runtime/security-authority.md](runtime/security-authority.md) |
| test/fixture 작성 | [quality/index.md](quality/index.md) |
| concurrency/crash 검증 | [quality/concurrency-fault-testing.md](quality/concurrency-fault-testing.md) |
| 성능 최적화 | [quality/benchmark-performance.md](quality/benchmark-performance.md) |
| 문서 작성/구조 변경 | [documentation-rules.md](documentation-rules.md) → [quality/documentation-consistency.md](quality/documentation-consistency.md) |
| 완료/재검수 | [quality/review-completion.md](quality/review-completion.md) |

## Category Index

- [Repository](repository/index.md): tree, naming, crate/module/file placement
- [Architecture](architecture/index.md): ownership, dependency, Common/Extension, capability/provider/plugin, removal
- [Rust](rust/index.md): type, ownership, memory, concurrency, channel, cache, error, dependency/unsafe
- [Runtime](runtime/index.md): lifecycle, shutdown, resource, persistence/recovery, security/observability
- [Quality](quality/index.md): testing, fixture layout, fault testing, benchmark, documentation, completion

## Legacy Entry Points

기존 링크 호환을 위해 다음 문서는 Router로 유지한다.

- [architecture-and-code-rules.md](architecture-and-code-rules.md)
- [repository-and-git-rules.md](repository-and-git-rules.md)
- [testing-and-review-rules.md](testing-and-review-rules.md)

새 상세 규칙은 해당 legacy 문서에 다시 복제하지 않는다.

---
title: "v0.8.10 Review Fix Plan — Clippy·Warning·Error Loss 개선"
document_id: "DXB-DEL-065"
version: "0.8.10"
status: "Completed"
normative: false
priority: "P0"
last_updated: "2026-08-23 (all 9 issues resolved: 6 plan + 3 hidden clippy)"
depends_on: ["DXB-DEL-061", "DXB-ENG-052", "DXB-IFC-040"]
---
# v0.8.10 Review Fix Plan — Clippy·Warning·Error Loss 개선

## 1. 목적

본 문서는 2026-08-23 리뷰에서 발견된 6개 이슈를 해결하기 위한 실행 계획이다. 변경 범위는 기존 코드의 결함 수정에 국한되며, 신규 기능이나 계약 변경을 포함하지 않는다.

---

## 2. 발견 이슈 요약

| ID | Severity | Crate | 설명 |
|---|---|---|---|
| RV-001 | bug | `dxbot-core` | `cargo clippy --workspace`가 `result_large_err`로 실패. `DxbotError`(184 bytes)가 `Err` variant로 너무 큼 |
| RV-002 | suggestion | `cli` | `unpredictable_function_pointer_comparisons` 경고 — `recovery.rs:205`에서 `fn` 포인터를 `==`로 비교 |
| RV-003 | suggestion | `storage-spike` | `tests/m1a-support/mod.rs:35` — dead function `effect` |
| RV-004 | suggestion | `application` | `tests/subscription.rs:151` — 불필요한 `mut` |
| RV-005 | suggestion | `cli` | `tests/automation.rs:116` — unused variable `r` |
| RV-006 | nit | `storage-spike` | `From<DxbotError> for SpikeError` 변환에서 structured error 정보 손실 |

---

## 3. 작업 항목

### 3.1 RV-001 — `dxbot-core` clippy `result_large_err` 수정

**분류**: Tier A — Common / Contract (Workspace 빌드 실패)

**원인**: `DxbotError` 구조체가 184 bytes로 `clippy::result_large_err` threshold를 초과. `application-contract`는 `#![allow(clippy::result_large_err)]`를 선언했으나 `dxbot-core`는 미선언.

**작업**:
- `crates/dxbot-core/src/lib.rs`에 `#![allow(clippy::result_large_err)]` 추가
- `DxbotError`는 의도적으로 wide한 canonical error struct이므로 boxing보다 allow가 적절

**영향 파일**:
- `crates/dxbot-core/src/lib.rs`

**완료 조건**:
- `cargo clippy --workspace` exit 0
- 기존 `cargo test --workspace` 통과 유지

---

### 3.2 RV-002 — `cli/recovery.rs` 함수 포인터 비교 경고 수정

**분류**: Tier B — Extension (Implementation detail)

**원인**: `fn` 포인터를 `==`로 비교하면 codegen unit에 따라 다른 주소가 나올 수 있음. `#[warn(unpredictable_function_pointer_comparisons)]` 발생.

**작업**:
- `crates/cli/src/recovery.rs:205`에서 `self.binding_lookup == other.binding_lookup` → `std::ptr::fn_addr_eq(self.binding_lookup, other.binding_lookup)`로 변경

**영향 파일**:
- `crates/cli/src/recovery.rs`

**완료 조건**:
- `cargo check` warning 0

---

### 3.3 RV-003 — `storage-spike` dead function `effect` 제거

**분류**: Tier B — Extension (Test fixture)

**원인**: `crates/storage-spike/tests/m1a-support/mod.rs:35`의 `effect` 함수가 사용되지 않음.

**작업**:
- `effect` 함수 제거

**영향 파일**:
- `crates/storage-spike/tests/m1a-support/mod.rs`

**완료 조건**:
- `cargo test -p storage-spike` warning 0

---

### 3.4 RV-004 — `application/subscription` 불필요한 `mut` 제거

**분류**: Tier B — Extension (Test fixture)

**원인**: `crates/application/tests/subscription.rs:151`에서 `mut`가 불필요하게 선언됨.

**작업**:
- `cargo fix --test "subscription" -p application` 실행 또는 수동으로 `mut` 제거

**영향 파일**:
- `crates/application/tests/subscription.rs`

**완료 조건**:
- `cargo test -p application subscription` warning 0

---

### 3.5 RV-005 — `cli/automation` unused variable `r` 제거

**분류**: Tier B — Extension (Test fixture)

**원인**: `crates/cli/tests/automation.rs:116`에서 변수 `r`이 선언만 되고 사용되지 않음.

**작업**:
- `let r = ...` → `let _ = ...` 또는 `let _r = ...`로 변경

**영향 파일**:
- `crates/cli/tests/automation.rs`

**완료 조건**:
- `cargo test -p cli automation` warning 0

---

### 3.6 RV-006 — `SpikeError` error 변환 정보 손실 최소화

**분류**: Tier B — Extension (Conversion quality)

**원인**: `From<dxbot_core::DxbotError> for SpikeError` 구현이 `DxbotError`의 모든 구조화된 정보(code, category, field_violations, next_actions 등)를 버리고 단순 문자열로 변환함.

**작업**:
- `SpikeError`에 `DxbotError`를 wrapping하는 variant 추가 또는 `message` 필드 보존
- 최소한 `DxbotError.message`를 `SpikeError::InvariantViolation`의 메시지에 포함

**권장 구현**:

```rust
impl From<dxbot_core::DxbotError> for SpikeError {
    fn from(error: dxbot_core::DxbotError) -> Self {
        Self::InvariantViolation(
            format!("core domain error in storage spike: {}", error.message)
        )
    }
}
```

**영향 파일**:
- `crates/storage-spike/src/types.rs`

**완료 조건**:
- `DxbotError`의 `message` 필드가 `SpikeError` 변환 시 보존됨

---

## 4. 검증

### 4.1 빌드 검증

```bash
cargo check --workspace    # exit 0, warning 0
cargo clippy --workspace   # exit 0
cargo test --workspace     # exit 0, warning 0
```

### 4.2 Acceptance 연결

모든 수정은 기존 Acceptance(`AT-STORAGE-001`, `AT-JOURNAL-001`, `AT-APP-006`, `AT-APP-007`, `AT-CLI-AUTOMATION-001`)의 재검증으로 충분하다. 신규 Acceptance는 불필요.

### 4.3 재검수

수정 완료 후 다음을 수행한다.

1. **Recheck 1 — Structural / Consistency**: 파일 변경이 의도한 크레이트에만 국한되었는지, dependency 경계가 유지되는지 확인
2. **Recheck 2 — Cross-Layer Executability**: `cargo test --workspace` + `cargo clippy --workspace` 전체 통과 확인

---

## 5. 영향 범위

| 수정 | Tier | 영향 크레이트 | Acceptance 영향 |
|---|---|---|---|
| RV-001 | A | `dxbot-core` | 전체 workspace 빌드 |
| RV-002 | B | `cli` | `AT-JOURNAL-001` |
| RV-003 | B | `storage-spike` | `AT-STORAGE-001` |
| RV-004 | B | `application` | `AT-APP-007` |
| RV-005 | B | `cli` | `AT-CLI-AUTOMATION-001` |
| RV-006 | B | `storage-spike` | `AT-STORAGE-001` |

기존 29개 Acceptance 중 어느 것도 fail로 전환되지 않아야 한다.

---

## 6. 실행 순서

1. **RV-001** — workspace 빌드 실패이므로 최우선
2. **RV-002** — 컴파일러 경고 해소
3. **RV-003~005** — 테스트 경고 해소 (병렬 가능)
4. **RV-006** — error 정보 보존 품질 개선
5. 전체 검증: `cargo check` → `cargo clippy` → `cargo test`

---

## 7. 완료 정의

- `cargo check --workspace`: exit 0, warning 0
- `cargo clippy --workspace`: exit 0
- `cargo test --workspace`: all pass, warning 0
- `DXB-DEL-061` Acceptance 29개 전체 `Passed` 유지
- `DXB-DEL-065` status → `Completed`
---
title: "Observability, Diagnostics, State Backstop 보완 계획"
document_id: "DXB-ADP-022"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P0"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010", "DXB-ADP-020"]
target_owners: ["runtime-host", "runtime-audit", "runtime-security", "application"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Observability, Diagnostics, State Backstop 보완 계획

## 1. 판단

**분류: Adopt/Close.** Grok 재구성본에서 서로 다른 두 패턴을 분리해 참고한다.

- `session-diagnostics.ts`는 session 이상을 별도 reporter로 보내는 얇은 경로를 둔다.
- `state-backstop-service.ts`는 store snapshot을 debounce하고, 최대 byte cap을 적용하며, `uploaded/skipped/error` 결과와 dispose를 명시한다.

DXBOT은 이 구현을 그대로 복제하지 않는다. raw `store.db` 업로드나 자동 restore가 아니라 다음을 차용한다.

1. audit와 분리된 typed diagnostics 경로
2. item·byte·time bound를 가진 recovery backstop snapshot
3. snapshot capture/skip/error의 명시적 상태
4. restore를 별도 authorization·validation·receipt 경로로 강제
5. 의심 resource의 fail-closed quarantine

## 2. 네 가지 관찰·복구 계층

| 계층 | 목적 | Owner | Durable |
|---|---|---|---|
| Audit | 권한·정책·변경·side effect 책임 추적 | runtime-audit/application | 예 |
| Diagnostic Event | runtime/provider/storage 이상 원인 분석 | component owner + runtime-host | 정책에 따라 제한 |
| Health Projection | 현재 readiness/degraded 요약 | runtime-host | derived/rebuildable |
| Recovery Backstop | 마지막 검증 가능한 snapshot/evidence 보존 | canonical storage owner + recovery coordinator | 선택적 durable |

Backstop은 audit, backup 정책, canonical commit log를 대체하지 않는다.

## 3. Diagnostic envelope

```text
DiagnosticEvent
- event_id
- observed_at
- instance_id
- host_generation
- component_kind
- component_id
- family
- reason_code
- severity
- lifecycle_state
- operation_id? / execution_id? / provider_id?
- safe_attributes
- source_error_class
- retryable
- action_required
```

금지 필드:

- prompt/message/memory raw content
- secret/token/path 전체값
- arbitrary provider response body
- unbounded stack trace
- high-cardinality user-controlled label

`safe_attributes`는 schema별 key allowlist와 byte cap을 가진다. Grok의 자유형 `Record<string, unknown>` reporter보다 DXBOT 쪽을 더 엄격하게 만든다.

## 4. Reason family 초안

```text
runtime.lifecycle
runtime.extension-graph
runtime.shutdown
storage.integrity
storage.recovery
provider.lifecycle
provider.credential
provider.protocol
sandbox.ownership
sandbox.artifact
sandbox.readiness
projection.stale
subscription.gap
security.authorization
security.secret-materialization
resource.exhaustion
```

구체 code는 해당 owner가 소유한다. 문자열 자유 입력이 아니라 sealed enum/registry로 시작한다.

## 5. Recovery backstop snapshot

```text
BackstopSnapshot
- snapshot_id
- resource_kind
- resource_id
- source_owner
- source_generation
- source_revision_or_watermark
- schema_version
- content_digest
- byte_length
- captured_at
- storage_ref
- verification_state
- retention_class
```

### 5.1 Capture

- storage owner가 제공하는 consistent checkpoint/snapshot API만 사용한다.
- live DB/file을 임의로 `readFile`하여 consistency를 추정하지 않는다.
- capture는 byte cap, deadline, frequency/debounce, concurrent-inflight cap을 가진다.
- 같은 resource의 연속 요청은 bounded coalescing할 수 있다.
- `Captured`, `SkippedNoChange`, `SkippedOverLimit`, `Failed`를 구분한다.
- snapshot sink 실패가 canonical commit 판정을 바꾸지 않는다.
- snapshot bytes와 metadata를 atomic하게 publish한다.

### 5.2 Restore

startup에서 backstop을 silent current state로 자동 복원하지 않는다.

```text
operator/recovery trigger
→ authorization
→ snapshot digest/schema/generation validation
→ canonical log/receipt/audit watermark 비교
→ dry-run recovery plan
→ explicit recovery operation
→ receipt + audit
→ index/projection rebuild
```

snapshot이 최신이라는 보장이 없으면 partial/stale로 취급한다. 더 최신 canonical commit을 덮어쓰지 않는다.

### 5.3 Disposal과 retention

- capture worker는 shutdown에서 pending timer/task를 취소한다.
- stopped 이후 upload callback을 실행하지 않는다.
- retention은 count+bytes+age bound를 가진다.
- purge는 storage/security 정책과 audit를 따른다.

## 6. Quarantine contract

Quarantine은 Grok backstop의 직접 차용이 아니라 DXBOT fail-closed 복구 보완이다.

```text
QuarantineRecord
- resource_kind
- resource_id_or_digest
- detected_at
- detection_reason
- original_location_ref
- quarantine_location_ref
- observed_size/digest
- owner_generation
- recovery_status
```

- 이동 또는 격리는 atomic publish 가능한 storage primitive를 사용한다.
- 원본 secret/raw payload를 audit에 복제하지 않는다.
- 자동 삭제하지 않는다.
- recovery 또는 purge는 별도 authorization/receipt를 요구한다.
- quarantined resource를 default/empty canonical state로 대체하지 않는다.

## 7. Health projection

```text
InstanceHealth
- readiness
- host_generation
- started_at
- mandatory_components
- degraded_components (bounded)
- active_execution_count
- queue/resource saturation summary
- latest_event_watermark
- audit_watermark
- projection_watermarks
- latest_verified_backstop?
- safe action hints
```

CLI `status/doctor`가 이를 소비할 수 있으나 새 command 추가는 현재 계획 범위에서 하지 않는다. 기존 표면에 필요한 최소 필드만 연결한다.

## 8. Backpressure와 보존

- diagnostic channel은 bounded queue다.
- low severity overflow는 count와 dropped watermark를 남긴다.
- critical diagnostic는 audit/operation correctness를 대신하지 않는다.
- disk/object sink failure 시 memory queue를 무한 확대하지 않는다.
- stack/log excerpt는 byte cap, retention, redaction을 적용한다.
- metric label에는 ID 원문 대신 bounded category를 사용한다.
- backstop capture가 정상 request latency path를 block하지 않되, consistency checkpoint 비용은 owner budget에 포함한다.

## 9. 구현 작업

1. 현재 error/audit/health/recovery snapshot 필드를 inventory하고 중복 owner를 제거한다.
2. private `DiagnosticEvent`와 family registry를 정의한다.
3. runtime-host lifecycle, provider-host, storage-spike에서 각 1개 family를 연결한다.
4. bounded in-memory sink와 write-time redaction test를 만든다.
5. test storage owner에 consistent `capture_backstop` prototype을 만든다.
6. byte cap, debounce/coalescing, sink failure, shutdown dispose를 검증한다.
7. restore는 dry-run validation과 explicit recovery operation만 설계하고 자동 적용하지 않는다.
8. corrupt projection/runtime artifact에 대한 quarantine prototype을 분리한다.
9. health projection을 generation/watermark/backstop verification에 결박한다.

## 10. Acceptance

- diagnostic sink가 실패해도 canonical commit 여부가 변하지 않는다.
- secret-like payload가 write 전 redaction된다.
- unbounded labels/log/stack이 저장되지 않는다.
- backstop capture는 consistent source revision/watermark와 digest를 가진다.
- over-limit snapshot은 명시적으로 skip되고 memory allocation이 cap을 넘지 않는다.
- capture sink failure가 무한 retry나 queue 성장을 만들지 않는다.
- shutdown 후 pending capture callback이 실행되지 않는다.
- restore는 authorization·validation·receipt 없이 수행되지 않는다.
- quarantine 후 resource를 silent current state로 읽지 않는다.
- restart 후 health projection을 canonical state와 event/backstop watermark에서 재생성할 수 있다.

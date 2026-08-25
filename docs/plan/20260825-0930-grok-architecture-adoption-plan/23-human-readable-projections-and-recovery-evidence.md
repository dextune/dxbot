---
title: "사람이 검사 가능한 Projection과 Recovery Evidence 계획"
document_id: "DXB-ADP-023"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P1"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010", "DXB-ADP-022"]
target_owners: ["application", "dxbot-core", "cli", "runtime-host"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# 사람이 검사 가능한 Projection과 Recovery Evidence 계획

## 1. 판단

**분류: Adopt.** Grok의 profile/memory/session 파일은 사용자가 직접 확인하기 쉽다는 장점이 있다. 그러나 DXBOT에서는 파일을 canonical Identity·Memory로 삼지 않고 **read-only derived projection**으로만 차용한다.

## 2. 목적

- 운영자와 개발자가 Bot/Memory/Runtime 상태를 빠르게 검사
- backup/recovery 중 canonical store와 projection의 불일치 탐지
- CLI가 실패한 환경에서도 최소 safe evidence 확보
- issue report에 raw database나 secret을 첨부하지 않고 구조적 상태 공유

## 3. Projection 종류

P1 최소 범위:

```text
inspect/
  instance-summary.json
  runtime-health.json
  bots/
    <bot-id>/summary.json
  memories/
    <scope-id>/summary.md
  operations/
    recent-summary.jsonl
  manifest.json
```

실제 path는 storage profile과 사용자 데이터 디렉터리 규칙이 결정한다. 위 구조를 canonical path로 선결정하지 않는다.

## 4. 공통 header

각 projection은 다음 metadata를 포함한다.

```text
projection_schema_version
projection_kind
instance_id
source_owner
source_revision_or_watermark
policy_generation
generated_at
content_digest
partial
redaction_profile
```

Markdown projection도 YAML front matter 또는 인접 manifest를 통해 동일 metadata를 가진다.

## 5. Canonical 방화벽

- projection 파일 편집은 canonical state를 바꾸지 않는다.
- projection 파일 삭제는 canonical state 삭제가 아니다.
- startup 시 projection을 canonical source로 읽지 않는다.
- import가 필요하면 별도 parse → validate → proposal/command → receipt 경로를 사용한다.
- Memory projection은 assertion state, provenance reference, temporal validity를 잃지 않는다.
- raw conversation history를 Memory summary로 자동 승격하지 않는다.

## 6. Atomic publication

```text
build to temporary generation directory
→ fsync files where required
→ write manifest last
→ atomic publish current generation pointer
→ retain bounded prior generation
```

부분 생성된 디렉터리를 current로 노출하지 않는다. 생성 실패는 이전 valid projection을 유지하되 health에 stale watermark를 표시한다.

## 7. Redaction과 정보량

- private/sensitive Memory statement는 기본 projection에서 원문을 제외한다.
- operation summary는 command key, status, timestamps, safe IDs만 제공한다.
- paths, provider response, prompt, tokens, credentials를 제외한다.
- item/byte cap을 적용하고 truncated/partial을 표시한다.
- 사람이 읽는 summary와 machine-readable manifest를 분리한다.

## 8. Recovery evidence

```text
RecoveryEvidence
- recovery_run_id
- source store generation
- last valid receipt/outbox/audit watermarks
- projection generation before/after
- detected gaps/corruption reason codes
- actions attempted
- actions committed with operation/receipt refs
- remaining operator actions
```

RecoveryEvidence는 상태를 변경하지 않는다. 실제 repair는 기존 Application operation과 receipt가 소유한다.

## 9. CLI 관계

CLI는 projection 파일을 직접 mutation authority로 사용하지 않는다.

- 정상 상태: Application Query/Control Client 사용
- runtime unavailable: local safe reader로 projection manifest와 stale 상태만 표시
- automation output: canonical API result와 projection fallback을 명시적으로 구분
- fallback에서 성공 exit를 주지 말아야 하는 경우를 operation status와 분리 정의

## 10. 구현 작업

1. 가장 작은 `instance-summary` projection 하나를 구현한다.
2. source watermark/digest/partial metadata를 고정한다.
3. atomic publish와 crash test를 추가한다.
4. Memory summary는 accepted/current 상태만 단순화하지 말고 state/provenance를 보존한다.
5. local safe reader는 manifest 검증 후에만 읽는다.
6. projection rebuild/delete/corruption test를 추가한다.

## 11. Acceptance

- projection 삭제 후 canonical store에서 완전 재생성된다.
- partial generation이 current로 publish되지 않는다.
- source revision 변경 후 stale projection이 current로 오인되지 않는다.
- projection 편집이 canonical state를 변경하지 않는다.
- private data와 secret이 기본 projection에 나타나지 않는다.
- fallback reader가 digest/schema mismatch를 정상 데이터로 반환하지 않는다.

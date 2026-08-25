---
title: "Build Provenance와 Artifact Verification 계획"
document_id: "DXB-ADP-026"
version: "0.1.0"
status: "Reference Snapshot"
normative: false
priority: "P1"
last_updated: "2026-08-25"
depends_on: ["DXB-ADP-010", "DXB-ADP-021", "DXB-ADP-025"]
target_owners: ["runtime-bootstrap", "provider-host", "runtime-security", "scripts"]
package_path: "docs/plan/20260825-0930-grok-architecture-adoption-plan"
source_baseline:
  dxbot_commit: "e43739614631c95752482c2ad2ec53cb7ebd251f"
  grok_reconstructed_commit: "a9f633e09d49a85829b8236331b9e21f7e612634"
---
# Build Provenance와 Artifact Verification 계획

## 1. 판단

**분류: Observe → Adopt when needed.** Grok 재구성본은 pinned upstream artifact, SHA-256 검증, deterministic overlay, package verification을 사용한다. DXBOT의 순수 Rust workspace에는 이를 그대로 도입할 필요가 없다. 다만 다음 외부 artifact가 runtime에 들어오는 순간 공통 규칙이 필요하다.

- subprocess Provider binary/CLI
- sandbox base image
- helper daemon
- model/tool runtime asset
- downloaded schema/protocol fixture
- prebuilt native library

## 2. 목표

외부 artifact의 “어디서 왔는가”, “어떤 bytes인가”, “어떤 runtime과 호환되는가”를 startup 전에 검증한다.

## 3. 최소 manifest

```text
ArtifactManifestEntry
- artifact_id
- kind
- source_uri_or_distribution_ref
- expected_sha256
- expected_size
- version
- platform/architecture
- compatibility_generation
- license/provenance_note
- fetched_at
- verified_at
```

manifest에는 credential이 포함되지 않는다. source URI가 민감한 내부 주소라면 safe distribution ref로 대체한다.

## 4. Content-addressed staging

```text
fetch/read source
→ stream digest and size validation
→ write temporary file
→ fsync as required
→ atomic move to digest path
→ mark verified
→ runtime consumes read-only
```

- digest path의 기존 bytes가 다르면 덮어쓰지 않는다.
- verification 전 artifact를 실행하지 않는다.
- symlink와 path traversal을 거부한다.
- 압축 해제는 entry count, total bytes, path depth를 bound한다.
- cache는 삭제 가능하며 manifest/source로 재구성 가능해야 한다.

## 5. Runtime binding

Sandbox/Provider descriptor는 artifact ID만 참조하지 않고 expected digest와 compatibility generation에 결박한다.

```text
Provider Adapter Generation
↔ Protocol Version
↔ Artifact Digest
↔ Security Profile Generation
```

하나가 변하면 readiness를 다시 평가하고 stale runtime을 silent reuse하지 않는다.

## 6. Publication check

P1 최소 publication check:

- required source/config/test/manifest 존재
- generated artifact가 repository source로 오인되지 않음
- lockfile/toolchain 일치
- external artifact digest 목록 완전성
- package/archive에서 secret/local cache 제외
- fresh checkout에서 manifest validation 가능

완전한 supply-chain attestation, remote signing infrastructure, 조직 전체 SBOM 시스템은 이 계획 범위가 아니다.

## 7. Provenance 분류

| 분류 | 예 | 정책 |
|---|---|---|
| Source-built | DXBOT Rust binary | commit/toolchain/profile 기록 |
| Downloaded pinned | helper binary/image | digest/size/source 검증 필수 |
| User-provided | local provider CLI | version/path/permission probe, bytes 정책 별도 |
| Generated | projection/cache/bundle | source watermark/manifest로 재생성 가능 |
| Secret material | token/credential | artifact manifest에 포함 금지 |

## 8. 실패 의미

- digest mismatch: `ArtifactIntegrityMismatch`, 실행 금지
- missing manifest: required artifact면 readiness 실패
- incompatible generation: replace 또는 operator action 필요
- source unavailable but verified cache exists: policy에 따라 degraded/ready 결정
- verified cache corruption: quarantine 후 재획득
- unknown user binary: explicit opt-in과 warning 없이는 실행 금지

## 9. 구현 작업

1. 외부 artifact가 실제로 들어오는 경로를 inventory한다.
2. subprocess canary 하나에 manifest/digest 검증을 적용한다.
3. streaming hash와 atomic staging을 공통 helper로 만든다.
4. sandbox descriptor와 artifact digest를 연결한다.
5. tamper, partial download, path traversal, stale generation test를 만든다.
6. archive/ZIP release에 `artifact-manifest.json`과 checksum을 포함한다.

## 10. Acceptance

- 검증되지 않은 required artifact가 실행되지 않는다.
- digest mismatch가 cache overwrite로 숨겨지지 않는다.
- partial download가 verified path에 publish되지 않는다.
- manifest와 runtime generation drift가 readiness에서 탐지된다.
- archive가 secret, local credential, runtime cache를 포함하지 않는다.
- fresh checkout에서 source-built path는 외부 binary 없이도 가능한 범위를 명확히 유지한다.

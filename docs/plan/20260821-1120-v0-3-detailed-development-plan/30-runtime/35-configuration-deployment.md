---
title: "구성·프로파일·배포"
document_id: "DXB-RUN-035"
version: "0.3.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-016", "DXB-ARC-017", "DXB-RUN-032"]
---

# 구성·프로파일·배포

## 1. 목적

환경/Bot/Task별 설정을 typed immutable snapshot으로 관리하고 Provider config를 표준 schema/lifecycle/Host validation과 연결한다.

## 2. Config Precedence

낮은 우선순위부터 compiled safe defaults → deployment → workspace/project → runtime profile → Bot profile → Goal/Task policy override → one-shot Execution override → emergency restriction을 사용한다. 보안 hard ceiling은 하위 layer가 완화하지 못한다.

## 3. Config Entry Metadata

모든 key는 owner, type/unit, default source, valid range, scope, reloadability, sensitivity, version/deprecation을 가진다. 둘 이상의 문서에서 쓰는 limit/default는 catalog 또는 Policy Owner가 SSOT다.

## 4. Provider Config Contract

각 Provider는 Capability Contract/Provider SDK가 요구하는 표준 config envelope 안에서 provider-specific schema를 정의한다.

최소 metadata:
- provider ID/version
- capability contract version range
- Provider SDK compatibility range
- provider-specific schema version
- config digest
- SecretRef fields/sensitivity
- required external endpoint/runtime feature
- lifecycle restart/reload requirement
- deprecation/removal replacement

Provider config는 global untyped namespace에 임의 key를 삽입하지 않는다.

## 5. Validation / Snapshot

`parse → schema validate → reference/secret metadata validate → capability/SDK compatibility → security/resource preflight → lifecycle validate → immutable config snapshot/digest → start candidate`

Provider는 raw merged config map을 받아 런타임에 임의 해석하지 않는다. Call 시에는 필요한 config snapshot/reference만 Provider Host가 pin한다.

## 6. Provider Selection Configuration

- selector rule은 Common Policy가 소유
- Provider config는 자신을 기본으로 선택하도록 강제할 수 없음
- registration order는 semantics가 아님
- Execution은 Provider ID/version/config generation을 pin
- automatic fallback은 explicit selector policy가 있는 경우만

## 7. Removed/Stale Provider Config

- deprecated key: replacement/removal 정보
- removed Provider ID: startup/reload diagnostic
- required Provider removed: startup fail 또는 explicit Degraded policy
- optional stale config: silent ignore 금지
- migration tool은 dry-run/diff/backup 제공

## 8. Plugin Configuration

Plugin config는 Plugin Manifest schema/version을 따른다. Plugin이 제공하는 각 Provider config는 Plugin namespace 안에서도 Provider Contract validation을 별도로 통과한다.

## 9. Hot Reload

Candidate parse/validate → component/lifecycle candidate build → immutable generation swap → old generation Draining. 진행 중 Execution은 시작 generation을 유지한다.

restart/quiescence가 필요한 config를 Provider가 임의 hot-apply하지 않는다.

## 10. Deployment

P0 daemon+embedded storage+local control+optional Provider sidecar, P1 multiple Provider/Plugin Manager, P2 remote Provider/Core/isolated Plugin host를 목표로 한다. 원격화해도 Provider Host semantic enforcement를 유지한다.

## 11. 검증 기준

- 동일 config source가 deterministic digest를 만든다.
- invalid capability/SDK/config version Provider가 Ready가 되지 않는다.
- removed Provider stale config가 silent ignore되지 않는다.
- Provider가 config를 통해 권한/resource ceiling을 확대하지 못한다.
- reload 실패 시 기존 generation이 유지된다.

---
title: "구성·프로파일·배포"
document_id: "DXB-RUN-035"
version: "0.2.0"
status: "Draft"
normative: true
priority: "P0/P1"
last_updated: "2026-08-21"
depends_on: ["DXB-ARC-011", "DXB-ARC-012", "DXB-ARC-013", "DXB-ARC-016", "DXB-RUN-032"]
---

# 구성·프로파일·배포

## 1. 목적

환경/Bot/Task별 설정을 typed immutable snapshot으로 관리하고 Provider selector, Plugin config, Limit SSOT, stale config를 일관되게 처리한다.

## 2. Config Precedence

낮은 우선순위부터:
1. compiled safe defaults
2. deployment
3. workspace/project
4. runtime profile
5. Bot profile
6. Goal/Task policy override
7. one-shot Execution override
8. emergency restriction

보안 hard ceiling은 하위 layer가 완화하지 못한다.

## 3. Config Entry Metadata

모든 key는 owner, type/unit, default source, valid range, scope, reloadability, sensitivity, version/deprecation을 가진다.

둘 이상의 문서에서 쓰는 limit/default는 이 catalog 또는 명시 Policy Owner가 SSOT다.

## 4. Provider Configuration

- provider ID/version/capability versions
- selector priority/rules
- provider-specific config digest
- credentials는 SecretRef
- availability/required 여부
- drain/deprecation/removal state

Provider가 여러 개면 selector rule이 필수다. registration order는 configuration semantics가 아니다.

Execution은 provider selection/config generation을 pin한다.

## 5. Removed/Stale Provider Config

- deprecated key: replacement + removal version 경고
- removed Provider ID: startup/reload diagnostic
- required Provider removed: startup fail 또는 명시 Degraded policy
- optional stale config: silent ignore 금지
- automatic fallback은 selector policy에 명시된 경우만
- migration tool은 dry-run/diff/backup 제공

## 6. Plugin Configuration

Plugin config는 Plugin Manifest의 schema/version을 따른다.

- global untyped config namespace에 임의 key 삽입 금지
- plugin-owned namespace
- secret는 SecretRef
- enable/disable generation
- config migration/rollback
- uninstall 시 config retain/purge policy

Plugin package와 config의 호환성을 enable 전에 검증한다.

## 7. Hot Reload

Candidate parse/validate → reference resolve → component candidate build → immutable generation swap → old generation drain.

reload 가능: logging, 일부 soft limits, selector weights/rules, 일부 Bot defaults.

quiescence/restart 필요 가능: storage backend, schema, sandbox host, cryptographic key, Plugin public protocol major, hard resource topology.

진행 중 Execution의 Provider/Plugin generation은 바뀌지 않는다.

## 8. Deployment

P0 daemon+embedded storage+local control+optional Provider sidecar.
P1 remote control/TUI, multiple Provider, Plugin Manager.
P2 remote Core, external storage/index, isolated Plugin host/registry.

microservice 분해는 실제 격리/병목 증거 후 진행한다.

## 9. Logical Data Paths

config, data DB, artifacts, runtime sockets/locks, cache/index, logs/audit spool, backups, provider packages, plugin packages/data, Bot workspace를 분리한다. cache 삭제가 Canonical State를 손상시키지 않아야 한다.

## 10. 검증 기준

- 동일 config source가 deterministic digest를 만든다.
- shared limit/default의 owner가 하나다.
- removed Provider stale config가 silent ignore되지 않는다.
- Plugin config version mismatch가 enable 전에 차단된다.
- reload 실패 시 기존 generation이 유지된다.
- 진행 중 Execution은 시작 provider/config generation을 유지한다.

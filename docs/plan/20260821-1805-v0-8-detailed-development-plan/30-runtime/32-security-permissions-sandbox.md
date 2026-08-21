---
title: "보안·권한·정보 흐름·Local CLI Threat Model"
document_id: "DXB-RUN-032"
version: "0.8.0"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-RUN-031", "DXB-DOM-025", "DXB-DOM-029", "DXB-ARC-012"]
---

# 보안·권한·정보 흐름·Local CLI Threat Model

## 1. 목적

Common Authorization, Information Flow, ActionGrant, Sandbox 의미와 함께 local Runtime endpoint, terminal, file export의 공격면을 P0 계약으로 닫는다.

## 2. Authorization Owner

Runtime Authorization Decision Owner는 하나다. Control Endpoint/Application/CLI는 policy enforcement consumer이며 Role→Authority, approval, resource policy를 복제하지 않는다.

모든 request는 다음 tuple로 재검증된다.

```text
PrincipalRef + Action + ResolvedResourceRef + CurrentRevision/Generation + Context
```

selector 해석, OS user, CLI profile, `--yes`는 권한을 생성하지 않는다.

## 3. Endpoint / Bootstrap Security

P0 Linux reference:
- endpoint directory는 해당 user 소유, mode `0700`이다.
- Unix socket/descriptor/lock은 같은 directory 안에 두고 owner/permission을 검증한다.
- peer credential(`SO_PEERCRED` 또는 동등 검증)을 Runtime principal resolution에 결박한다.
- world/group-writable endpoint directory와 insecure `/tmp` fallback을 금지한다.
- path component와 descriptor를 no-follow 방식으로 열어 symlink replacement를 거부한다.
- stale endpoint는 lock/HostGeneration/process identity 검증 뒤에만 제거한다.
- Host Lifecycle Adapter는 shell string composition을 사용하지 않고 argv/structured service API를 사용한다.
- profile/endpoint override가 다른 data root·principal로 silent 연결되지 않게 InstanceId를 확인한다.

## 4. Credential / Secret

- argv에 token/password를 넣는 UX를 기본 제공하지 않는다.
- secret는 env/plain config/log/trace/receipt/local journal/metric label에 복사하지 않는다.
- config/doctor/export/error는 default redaction한다.
- verbose/debug가 redaction을 자동 해제하지 않는다.
- raw credential handle은 최소 scoped capability로 Provider Host에 전달한다.

## 5. Approval / Confirmation

Interactive confirmation은 local UX guard다. Runtime Approval/ActionGrant는 principal, target, action digest, revision, expiry, budget/use count를 가진 durable security semantic이다. `--yes`/`--non-interactive`는 ActionGrant를 생성하거나 우회하지 않는다.

## 6. Terminal Safety

Model/Tool/Message/Artifact/name/error text는 untrusted다.

기본 human renderer는:
- C0/C1 control을 printable escaped form으로 변환
- ANSI CSI/OSC/DCS/APC/PM, clipboard OSC 52, title/hyperlink sequence를 제거 또는 neutralize
- bidi/invisible/confusable 위험을 diagnostics에서 식별 가능하게 표시
- terminal width truncation이 원문/ID를 다른 resource처럼 보이게 하지 않음

machine JSON/JSONL은 semantic string을 올바르게 JSON escape하고 raw terminal action을 수행하지 않는다. `--raw`는 P0에 제공하지 않는다.

## 7. File / Export Safety

- caller가 지정한 output path를 canonicalize해 허용 root 정책과 비교하고 path traversal(경로 탈출)을 거부한다.
- symlink, hardlink 위험, device/FIFO/socket/directory를 destination으로 허용하지 않는다.
- 기존 파일 overwrite는 P0에서 거부한다.
- same-directory random temp를 exclusive/no-follow로 생성하고 sensitive output은 mode `0600`으로 제한한다.
- 성공 시 fsync 가능한 범위에서 flush 후 atomic rename한다.
- failure/cancel에서 partial temp를 삭제하고 삭제 실패는 안전한 경로/permission과 함께 명시한다.
- secret/raw sensitive data는 별도 authorization과 explicit export mode 없이 출력하지 않는다.

## 8. Information Flow와 Sandbox

Private/Sensitive → Project/Channel/Export는 Declassification과 audit를 요구한다. untrusted Model/Tool output은 permission, selector, file path, shell command를 자체 생성할 수 없다. sandbox 불가 시 unrestricted fallback을 기본값으로 사용하지 않는다.

## 9. 검증 기준

- 다른 user/permission의 fake socket·symlink endpoint 연결 거부.
- stale socket replacement race에서 hijack 0.
- ANSI/OSC/clipboard/title/hyperlink injection fixture 무해화.
- `../`, symlink, FIFO/device, existing file export 거부.
- partial sensitive file 잔존 0 또는 explicit restricted recovery artifact.
- `--yes`로 approval bypass 0.
- secret canary가 argv/history/log/trace/receipt/machine output에 나타나지 않음.

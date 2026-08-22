---
title: "보안·Principal·정보 흐름·Local CLI Threat Model"
document_id: "DXB-RUN-032"
version: "0.8.5"
status: "Accepted"
normative: true
priority: "P0"
last_updated: "2026-08-22"
depends_on: ["DXB-RUN-031", "DXB-DOM-025", "DXB-DOM-029", "DXB-ARC-012"]
---
# 보안·Principal·정보 흐름·Local CLI Threat Model

## 1. Principal derivation

P0 local reference는 authenticated Unix peer credential과 Instance metadata에서 server-side로 결정한다.

```text
AuthenticatedPeerContext(OS UID, process peer facts)
+ InstanceId
→ LocalPrincipalRef
```

client의 `PrincipalContext`, profile, `--yes`, BotId는 Authority가 아니다. P0는 하나의 OS UID/Instance에 하나의 local principal mapping을 기본으로 하며 impersonation/delegation은 후속 IAM 계약 없이는 허용하지 않는다.

## 2. Endpoint

runtime directory user-owned `0700`, socket/descriptor/lock same directory, no insecure `/tmp` fallback, no-follow component validation, peer credential verification, InstanceId/HostGeneration match를 요구한다.

## 3. Secret/Approval

secret를 argv/plain config/log/receipt/local journal/metric label에 저장하지 않는다. `--yes`는 local confirmation만 생략하며 Runtime Approval/ActionGrant를 생성하지 않는다.

## 4. Terminal

C0/C1, ANSI CSI/OSC/DCS/APC/PM, OSC 52, title/hyperlink를 neutralize한다. bidi/invisible/confusable 위험을 표시하며 machine JSON/JSONL은 semantic string만 escape한다. P0 `--raw`는 없다.

## 5. Linux safe export

1. 허용 root/parent를 descriptor-relative로 no-follow 탐색한다.
2. destination directory/special file/symlink/hardlink 위험과 traversal을 거부한다.
3. same-directory temp를 exclusive create하고 sensitive mode `0600`을 적용한다.
4. bounded stream write, flush, file fsync를 수행한다.
5. `renameat2(RENAME_NOREPLACE)` 또는 동등한 **atomic no-replace** primitive로 publish한다.
6. parent directory를 fsync한다.
7. no-replace 보장을 제공할 수 없으면 fail closed한다.
8. cancel/disk-full에서 temp를 제거하고 실패 시 restricted recovery artifact만 보고한다.

검사 후 일반 rename으로 existing destination을 덮어쓰는 구현은 금지한다.

## 6. Information flow

Private/Sensitive → Project/Channel/Export는 Declassification과 audit를 요구한다. Model/Tool output이 principal, selector, path, shell command, permission을 스스로 승인하지 못한다.

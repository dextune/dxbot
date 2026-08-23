# Security and Authority Guide

## Applies When

Principal, permission, approval, secret, sandbox, external action, endpoint trust를 변경할 때 적용한다.

## Core Rules

기본 권한은 least privilege와 deny-unknown이다. Principal은 trusted boundary에서 파생하고 client hint나 Domain ID를 Authority로 사용하지 않는다. secret 원문을 Domain/Event/Memory/log에 저장하지 않는다.

untrusted input/Provider output이 policy/permission을 생성할 수 없게 한다. stale/revoked grant/execution/provider generation write를 fencing한다. 고위험 Side Effect는 approval/idempotency/audit 경계를 가진다.

## Decision Rules

권한과 UX confirmation은 분리한다. sandbox가 불가능하면 unrestricted fallback을 기본값으로 쓰지 않고 명시적 unsupported/deny를 사용한다.

## Forbidden Patterns

raw global Secret Store를 Provider에 전달, profile/client payload를 Authority로 신뢰, revoked grant cache 재사용, terminal/path/endpoint trust를 검증 없이 사용하는 것을 금지한다.

## Verification

permission deny/default, revoke/stale generation, secret redaction/canary, path traversal/link escape, Host bypass, approval continuation/restart, untrusted output을 검증한다.

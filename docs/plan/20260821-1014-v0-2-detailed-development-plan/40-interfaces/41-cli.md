---
title: "CLI Reference Interface"
document_id: "DXB-IFC-041"
version: "0.1.0"
status: "Draft"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: ["DXB-IFC-040", "DXB-DOM-026"]
---


# CLI Reference Interface

## 1. 목적

모든 핵심 Runtime 기능을 UI 없이 검증하고 자동화할 수 있는 안정적 Reference Interface를 제공한다.

## 2. 책임 범위

- command taxonomy
- human/json/ndjson output
- interactive/non-interactive
- exit code와 stderr/stdout
- local/remote 연결
- scripting, approval, trace
- backward compatibility

CLI는 Domain 로직이나 DB access를 포함하지 않고 Control Client만 사용한다.

## 3. 실행 파일

권장:
- `dxb`: 사용자 CLI
- `dxb daemon`: Runtime 실행 또는 별도 `dxbd`
- 동일 binary multi-call 여부는 ADR로 결정

CLI는 embedded development mode를 지원할 수 있으나 내부적으로 동일 Control API transport를 사용한다.

## 4. Command Tree

```text
dxb
├─ runtime
│  ├─ start | status | stop | doctor | version
├─ bot
│  ├─ create | list | show | activate | deactivate
│  ├─ update | archive | restore | clone | export | import | delete
├─ goal
│  ├─ create | list | show | pause | resume | close
├─ task
│  ├─ submit | list | show | graph | cancel | retry | wait | result
├─ core
│  ├─ list | show | watch | cancel
├─ memory
│  ├─ put | get | search | history | correct | forget | compact | index
├─ message
│  ├─ send | ask | delegate | inbox | outbox | trace
├─ capability
│  ├─ list | show | test
├─ approval
│  ├─ list | show | approve | deny
├─ config
│  ├─ show | validate | diff | reload
├─ trace
│  ├─ show | follow | export
└─ completion
```

실제 명칭은 API와 용어집에 맞춰 확정한다. alias는 제한하고 모호한 `run` 하나에 모든 의미를 넣지 않는다.

## 5. 출력 계약

### Human
- 읽기 쉬운 표/요약
- terminal width 대응
- 중요 상태와 stale/degraded 명시
- 색상은 보조 수단이며 `NO_COLOR`/비TTY 지원
- 긴 content는 pager 또는 Artifact 안내
- ID는 축약 표시 가능하나 copy용 전체 ID 제공

### JSON
- stdout에 단일 machine-readable document
- schema version
- 로그/progress는 stderr
- stable field names
- secret redaction
- exit code와 결과 일치

### NDJSON
- stream/watch/bulk 결과
- 한 줄 한 event
- stdout protocol 순수성
- heartbeat 옵션
- sequence/cursor 포함

## 6. 표준 입력과 파일

- `--file`, `--stdin`, inline argument 우선순위 명시
- binary/large content는 Artifact upload
- TTY가 아니면 interactive prompt 금지
- secret는 command-line argument로 받지 않고 stdin/secret store/reference
- `@path`와 같은 확장 syntax는 path ambiguity/security 검토 후 도입
- output file은 atomic write와 overwrite flag
- export/import checksum 검증

## 7. Exit Code

예시 범주:
- `0`: 성공/요청 수락
- `1`: 일반 Command 실패
- `2`: CLI usage/validation
- `3`: authorization/approval
- `4`: conflict
- `5`: unavailable/degraded
- `6`: timeout
- `7`: partial/bulk mixed result
- `8`: incompatible version
- `130`: 사용자 interrupt

정확한 값은 한 문서/모듈에서 소유하고 command별 임의 코드를 금지한다. JSON error code는 더 세분화한다.

## 8. Task 제출 예시 의미

```text
dxb task submit --bot <BOT_ID> --file task.md --wait
```

- `--wait`는 Command commit 후 Task terminal까지 event stream을 구독
- terminal timeout과 Command timeout을 분리
- Ctrl-C는 기본적으로 wait만 중단하고 Task 취소 여부를 묻거나 `--cancel-on-interrupt`로 명시
- `--idempotency-key` 지원
- `--json`에서는 progress를 stdout에 섞지 않음
- result Artifact를 자동 다운로드하지 않고 참조 표시 가능

## 9. Interactive Approval

- TTY에서 structured summary와 exact scope 표시
- approve/deny/constraint
- 입력 digest 확인
- non-interactive에서는 machine policy 또는 fail
- `--yes`가 권한/approval을 무조건 우회하지 않음
- destructive bulk action은 dry-run과 explicit scope
- approval response는 Control API로 전송되고 CLI 로컬 상태가 아님

## 10. Watch/Trace

- `task wait`, `core watch`, `trace follow`
- cursor resume
- reconnect with backoff
- duplicate event dedup
- terminal event 도달 시 종료
- `--since`, `--output ndjson`
- model reasoning 원문은 권한과 설정에 따라 제한
- stderr에 reconnect/lag 경고

## 11. 구성과 연결

우선순위:
1. explicit flags
2. selected CLI context/profile
3. env references
4. config file
5. defaults

server endpoint, Bot default, output format를 context로 저장할 수 있으나 secret 원문은 저장하지 않는다. 현재 context를 모든 명령에서 확인 가능하게 한다.

## 12. 예외상황

- daemon 없음: 명확한 start 안내 또는 명시적 embedded flag
- server version mismatch: supported range 표시
- broken pipe: stdout 종료를 오류 폭주로 처리하지 않음
- pager failure: direct output fallback
- terminal width 0/nonTTY: plain output
- event stream 끊김: cursor reconnect
- partial bulk: item별 result와 exit 7
- command commit 후 client timeout: idempotency lookup 명령 제공
- Unicode/locale: UTF-8 기준, invalid external bytes는 안전 표시

## 13. 확장성

CLI plugin command는 P2 이후 검토하며 core command namespace를 오염시키지 않는다. shell completion은 server capability catalog를 캐시할 수 있으나 offline fallback을 제공한다. remote fleet context가 추가돼도 동일 command schema를 사용한다.

## 14. 구현 우선순위

- **P0:** runtime/bot/task/memory/status/doctor, JSON output, idempotency
- **P1:** message/core/watch/approval/config, remote endpoint
- **P2:** bulk/admin/plugin, advanced trace/export
- **P3:** workflow scripting helpers

## 15. 검증 기준

- 모든 P0 use case가 GUI 없이 CLI에서 완료된다.
- JSON stdout에 로그/진행 표시가 섞이지 않는다.
- non-TTY에서 prompt로 hang하지 않는다.
- Ctrl-C의 Task 취소 의미가 flag에 따라 일관된다.
- CLI가 storage crate를 의존하지 않는다.
- command snapshot/golden test와 API compatibility test가 존재한다.

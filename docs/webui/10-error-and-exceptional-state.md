---
title: "Web UI Error와 예외 상태"
document_id: "DXB-WEB-010"
status: "Accepted"
normative: true
priority: "web-P0"
plan_baseline: "0.8.10"
last_updated: "2026-08-29"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-008"]
---
# Web UI Error와 예외 상태

## 목적

정상 데이터 외의 모든 상태에 대한 UI 정의 규칙을 소유한다. DXBOT은 실패·취소·중단·재시도가 정상 설계 범위이므로([AGENTS.md](../../AGENTS.md) §1.7) 예외 상태는 부가 기능이 아니라 필수 화면이다.

## 1. 반드시 UI를 정의해야 하는 상태

```text
loading                  최초 로딩
refreshing               데이터 갱신 중(기존 값 유지)
empty                    데이터 없음
partial                  일부 섹션만 실패
network-error            요청 실패
runtime-unavailable      Runtime 미가동/미준비
reconnecting             stream 재연결 중
resync-required          cursor gap/재시작
provider-unavailable     Provider 없음/열화
degraded                 부분 기능 제한
run-failed               실행 실패
run-cancelled            실행 취소
streaming-interrupted    응답 스트림 중단
stale                    화면 값이 최신이 아님
approval-required        승인 대기
permission-denied        권한 거부
conflict                 revision/generation 충돌
recovery-required        결과 불확실
superseded               다른 operation이 대체
deferred-outcome         operation 커밋 후 Domain이 작업을 지연
rejected-outcome         operation 커밋 후 Domain이 작업을 거부
observation-interrupted  관측만 끊김 (실행은 계속)
unknown-value            알 수 없는 enum/상태
```

각 상태는 `DXB-IFC-040`의 error category / exit registry 의미와 대응한다. Web 전용 실패 의미를 새로 만들지 않는다.

### 1.1 exit registry 대응

`DXB-IFC-040`의 stable exit registry가 실패 분류의 원본이다. UI 상태는 그 category의 표현이며, 다른 이름의 평행 분류 체계를 만들지 않는다.

| exit category | UI 상태 |
|---|---|
| `invalid-input` | 필드 인라인 검증 오류 |
| `not-found` | empty 또는 대상 없음 |
| `ambiguous-target` | 선택 요구 |
| `conflict` | conflict |
| `permission-denied` | permission-denied |
| `approval-required` | approval-required |
| `resource-exhausted` | degraded (admission 거부) |
| `runtime-unavailable` | runtime-unavailable |
| `incompatible` | 버전·스키마 비호환 안내 |
| `timeout` | network-error 계열 (로컬 대기 종료) |
| `interrupted` | 관측 중단 (실행 중단 아님) |
| `partial-or-resync` | partial / resync-required |
| `recovery-required` | recovery-required |
| `provider-unavailable` | provider-unavailable |
| `storage-or-corruption` | 전역 배너 + 조치 안내 |
| `internal-invariant` | 전역 배너 + 보고 경로 |

`usage`는 CLI 파서 오류이므로 Web UI에 대응 상태가 없다.

### 1.2 Receipt 상태와 Domain outcome은 다른 축이다

- Receipt: `Accepted | Committed | Rejected | Superseded | RecoveryRequired`
- Task outcome: `Deferred | Rejected | Cancelled | Completed | Failed | RecoveryRequired` 등

**Receipt `Committed` + Task outcome `Rejected/Deferred`는 operation 실패가 아니다.** operation은 성공적으로 커밋되었고 Domain이 그 작업을 거부·지연한 것이다. UI는 이를 "요청 실패"로 표시하지 않고 "요청 접수됨 / 작업 거부됨(사유)"으로 구분해 표시한다(`DXB-DOM-023`, `DXB-DOM-025`).

`Superseded`는 다른 operation이 대체했음을 뜻한다. 실패로 표시하지 않고 대체 대상을 안내한다.

## 2. 표현 위치 규칙

모든 오류를 toast로 처리하지 않는다. 오류의 **영향 범위**에 따라 위치를 정한다.

| 영향 범위 | 표현 |
|---|---|
| 단일 입력/필드 | 필드 인라인 메시지 |
| 단일 액션 | 액션 근처 인라인 + 액션 비활성 이유 표시 |
| 하나의 카드/섹션 | 섹션 내부 `error-state` (다른 카드는 정상 유지) |
| 화면 전체 데이터 | 화면 수준 오류 뷰 |
| 연결/Runtime 전역 | 전역 배너 (지속 표시, 자동 소멸 금지) |
| 일시적 사용자 피드백 | toast (성공 알림·경미한 경고 한정) |

특히 Cockpit은 여러 데이터 소스를 동시에 표시하므로 **한 섹션 실패가 화면 전체를 대체하지 않는다.** Workflow가 실패해도 Thread와 Run Queue는 계속 동작해야 한다.

## 3. 상태별 요구

1. `loading`: skeleton을 사용하고 레이아웃 높이를 유지해 점프를 만들지 않는다.
2. `refreshing`: 기존 값을 유지하고 갱신 중임을 약하게 표시한다. 화면을 비우지 않는다.
3. `empty`: 원인과 다음 행동을 함께 제시한다. "데이터 없음"만 표시하지 않는다.
4. `partial`: 실패한 섹션만 표시를 바꾸고, 어떤 부분이 누락되었는지 명시한다.
5. `reconnecting` / `resync-required`: 표시 중인 값이 최신이 아닐 수 있음을 알린다. 조용히 오래된 값을 보여주지 않는다.
6. `approval-required`: 성공으로 표시하지 않는다. 승인 대기임을 명시하고 승인 참조를 노출한다.
7. `conflict`: 자동 재시도하지 않고 현재 서버 값을 보여준 뒤 사용자가 결정하게 한다.
8. `recovery-required`: 결과가 불확실함을 명시하고, 조회/조정 경로를 제공한다. 실패로 단정하지 않는다.
9. `permission-denied`: 존재 여부를 추측할 수 있는 정보를 노출하지 않는다.
10. `unknown-value`: 화면을 깨지 않고 중립 표현으로 렌더링한다.
11. `streaming-interrupted`: 중단 지점을 표시하고 재시도 액션을 제공한다. 부분 응답을 완성된 응답처럼 보이게 하지 않는다.
12. `observation-interrupted`: **관측 중단을 실행 중단으로 표시하지 않는다.** 구독이 끊겼을 뿐 Task/Process는 계속 진행 중일 수 있다([18-domain-invariant-compliance.md](18-domain-invariant-compliance.md) §5).
13. `deferred-outcome` / `rejected-outcome`: operation이 접수·커밋되었음을 명확히 하고, 작업이 지연·거부된 사유를 별도 층으로 표시한다. 두 층을 하나의 "실패"로 합치지 않는다.
14. `superseded`: 대체된 사실과 대체 대상을 안내한다. 실패로 표시하지 않는다.

## 4. 금지 사항

- 오류를 콘솔에만 남기고 UI를 정상처럼 유지
- 실패를 빈 상태로 위장
- 모든 오류를 동일 문구로 축약
- 재시도 버튼 없는 종료 상태
- 사용자가 원인을 알 수 없는 무음 실패
- 자동 무한 재시도

## 5. 검증

- 각 상태를 강제로 주입하는 테스트 케이스를 갖춘다.
- 섹션 단위 실패 시 다른 섹션이 살아 있는지 확인한다.
- Dark/Light 두 모드에서 오류 표현의 대비를 확인한다.
- 상태별 스크린샷을 [15-verification-and-visual-regression.md](15-verification-and-visual-regression.md)의 component baseline에 포함한다.

## 관련 문서

- [08-api-and-state-architecture.md](08-api-and-state-architecture.md) — 이벤트·재연결
- [11-accessibility.md](11-accessibility.md) — 오류 전달 접근성

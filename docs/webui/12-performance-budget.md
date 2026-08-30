---
title: "Web UI 성능·메모리 예산"
document_id: "DXB-WEB-012"
status: "Accepted"
normative: true
priority: "web-P1"
plan_baseline: "0.8.10"
last_updated: "2026-08-29"
owner: "Web Control Center 문서 패키지"
depends_on: ["DXB-WEB-008"]
---
# Web UI 성능·메모리 예산

## 목적

Cockpit이 실시간으로 계속 갱신되는 화면이라는 특성에서 오는 성능·메모리 제약을 소유한다.

## 1. 문제의 성격

Cockpit은 한 화면에서 메시지 스트림, 실행 그래프, 실행 대기열, 이벤트 목록, 자원 metric을 동시에 표시한다. 이벤트 하나가 화면 전체를 재렌더하면 실사용에서 곧 무너진다. 따라서 재렌더 경계가 **설계 시점의 요구사항**이다.

## 2. Rendering

- 긴 목록(메시지, 이벤트, Run 목록)은 가상화한다.
- list item key는 안정적인 Domain 식별자를 사용한다. index를 key로 쓰지 않는다.
- 구독은 selector 기반으로 좁힌다. 큰 객체 전체를 구독해 하위 컴포넌트를 재렌더하지 않는다.
- 항목 단위 컴포넌트 경계를 명확히 해 한 항목 갱신이 형제 항목을 재렌더하지 않게 한다.
- Workflow 그래프는 노드/엣지 단위로 갱신한다. 상태 하나가 바뀔 때 그래프 전체를 재구성하지 않는다.
- 파생 데이터는 필요한 지점에서만 memoize한다. 전역 memo 캐시를 만들지 않는다.
- theme 전환이 전체 트리 재마운트를 유발하지 않게 한다.

## 3. Memory

- 이벤트·메시지 이력을 무제한 보관하지 않는다. 화면당 보관 한도와 폐기 정책을 정한다.
- 한도는 측정 없이 영구 고정하지 않는다([documentation-rules.md](../agent/documentation-rules.md) §8). 초기값을 정하고 실측으로 조정하며, 값의 소유는 구현 config에 둔다.
- 대형 payload(로그 본문, 첨부, 그래프 원본 데이터)를 여러 계층에 복사하지 않는다.
- binary/첨부를 global state에 보관하지 않는다.
- 화면 이탈 시 subscription과 타이머를 해제한다. 누수 여부를 테스트로 확인한다.
- unbounded queue/buffer를 만들지 않는다([AGENTS.md](../../AGENTS.md) §1.4).

## 4. Network

- 동일 endpoint 중복 요청을 dedupe한다.
- cache invalidation 대상을 명시한다. 전역 무효화로 전체를 다시 받지 않는다.
- 재연결은 backoff를 적용하고 상한을 둔다.
- 중복 stream event를 idempotent하게 처리한다([08-api-and-state-architecture.md](08-api-and-state-architecture.md) §3).
- polling으로 stream을 대체하지 않는다. stream이 없으면 그 사실을 UI에 드러낸다.

## 5. 검증

- 대량 데이터(메시지 수천 건, Run 수백 건, 노드 수십 개) fixture로 렌더 성능을 측정한다.
- 고빈도 이벤트 주입 시 재렌더 횟수를 측정한다. 항목 1개 갱신이 화면 전체 재렌더를 유발하면 결함이다.
- 장시간 실행(수십 분) 후 메모리 증가 추세를 확인한다.
- 화면 전환 반복 후 subscription 잔존 여부를 확인한다.

측정값은 벤치마크 기준과 함께 기록한다. 근거 없는 수치 목표를 문서에 고정하지 않는다.

## 관련 문서

- [08-api-and-state-architecture.md](08-api-and-state-architecture.md) — 캐시·이벤트 구조
- [05-component-inventory.md](05-component-inventory.md) — 그래프·목록 컴포넌트
- [quality/benchmark-performance.md](../agent/quality/benchmark-performance.md) — 성능 검증 규칙

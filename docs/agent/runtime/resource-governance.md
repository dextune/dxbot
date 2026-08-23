# Resource Governance Guide

## Applies When

CPU/core lease, task admission, queue/cache/storage/output budget, provider quota를 추가·조정할 때 적용한다.

## Core Rules

Resource policy와 accounting Owner는 하나다. admission은 실제 작업 시작 전에 수행하고 permit/lease는 명시적 lifetime과 반환 경로를 가진다. queue/cache/cursor/output에는 item/byte 상한을 적용 가능한 범위에서 함께 둔다.

Provider가 Common admission을 우회해 자체 semantic work queue나 quota를 만들지 않는다.

## Decision Rules

상한 숫자는 측정/운영 근거 없이 장기 Guide에 고정하지 않는다. 제품별 default/limit은 Config/Policy 또는 활성 Plan Owner가 소유한다.

## Forbidden Patterns

unbounded resource, hidden queue, permit leak, per-Provider global admission policy 중복, resource exhaustion을 retry storm으로 바꾸는 것을 금지한다.

## Verification

capacity boundary, cancellation/release, crash/restart accounting, slow consumer, disk/memory pressure, fairness/starvation을 검증한다.

---
title: "DXBOT 1차 컨셉 기획안 원문 보존본"
document_id: "DXB-SOURCE-000"
version: "1.0-source"
status: "Source of Truth"
normative: true
priority: "P0"
last_updated: "2026-08-21"
depends_on: []
---

# DXBOT 1차 컨셉 기획안

> 본 파일은 사용자가 제공한 기준 문서를 독립 문서 묶음 안에 보존하기 위한 원문 보존본이다. 상세 문서와 충돌하면 본 문서의 제품 철학과 확정 원칙을 우선한다.

## 1. 프로젝트 개요

**DXBOT**은 일반적인 세션 기반 AI Agent가 아니라, **지속적인 기억을 중심으로 존재하는 AI Bot Runtime 및 Bot 운영 시스템**을 목표로 한다.

각 Bot은 단순히 사용자의 요청을 받아 한 번의 작업을 수행하는 Agent가 아니다. 독립적인 정체성과 기억, 역할, 목표를 유지하며 장기간 존재하고, 필요에 따라 여러 작업을 동시에 수행하거나 다른 Bot과 협력할 수 있는 **지속형 AI 엔티티**로 정의한다.

DXBOT의 실행 기반에는 **DeepSeek Harness의 설계 철학과 구조를 차용한 Harness Layer**를 사용하며, 그 위에 DXBOT 고유의 Bot / Memory / Brain / Core / Multi-Bot 개념을 구축한다.

---

# 2. 핵심 정의

DXBOT을 한 문장으로 정의하면 다음과 같다.

> **DXBOT은 기억을 중심으로 지속적으로 존재하는 AI Bot들이 자신의 실행 능력을 동적으로 분산하고, 서로 협업하며, 하나의 시스템 안에서 통합 관리될 수 있도록 하는 멀티봇 AI Runtime이다.**

DXBOT에서 가장 중요한 단위는 Session이 아니라 **Bot**이다.

---

# 3. 기존 AI Agent와의 차이

일반적인 AI Agent는 대부분 다음 구조를 중심으로 동작한다.

**요청 → 세션 생성 → 컨텍스트 구성 → 작업 실행 → 세션 종료**

DXBOT은 이 구조를 뒤집는다.

**Bot 존재 → Memory 유지 → 필요할 때 실행 → 작업 종료 후에도 Bot 유지**

즉 세션은 Bot 자체가 아니다.

CLI, TUI, Web UI에서 이루어지는 대화와 작업 화면은 이미 존재하고 있는 Bot에 접속하기 위한 **Interface 또는 View**에 가깝다.

따라서 DXBOT의 기본 원칙은 다음과 같다.

> **Session이 Memory를 소유하지 않는다. Bot이 Memory를 소유하고 Session이 Bot에 접근한다.**

사용자가 터미널을 종료하거나 Web UI를 닫더라도 Bot의 정체성과 기억은 그대로 유지된다.

---

# 4. Bot

Bot은 DXBOT 시스템에서 지속적으로 존재하는 가장 중요한 논리적 엔티티다.

하나의 Bot은 개념적으로 다음 요소를 가진다.

* Identity
* Brain
* Memory
* Goals
* Tasks
* Skills
* Tools
* Permissions
* Workspace
* Routines
* Communication
* Core Runtime

각 Bot은 서로 다른 역할과 정체성을 가질 수 있다.

예를 들어 하나의 DXBOT 환경에 다음과 같은 Bot들이 존재할 수 있다.

* Main Bot
* Developer Bot
* Research Bot
* Review Bot
* Operations Bot

그러나 역할의 종류는 시스템에 고정하지 않는다.

사용자가 필요에 따라 자유롭게 새로운 Bot을 만들고 역할을 부여할 수 있어야 한다.

---

# 5. Memory-Centric Architecture

DXBOT에서 Bot의 연속성을 결정하는 핵심은 **Memory**다.

Bot이 어떤 모델을 사용하는지, 현재 어떤 프로세스에서 실행되고 있는지는 Bot의 본질이 아니다.

Bot의 핵심적인 연속성은 다음 요소에서 발생한다.

**Identity + Memory + Persistent State**

따라서 런타임이 종료되었다 다시 시작되더라도 해당 요소가 유지된다면 동일한 Bot으로 간주한다.

Memory는 단순한 과거 채팅 로그가 아니다.

DXBOT의 Memory는 개념적으로 다음과 같은 정보를 다룰 수 있다.

### Long-term Memory

Bot이 장기간 유지해야 하는 사실, 경험, 사용자와의 관계, 프로젝트 정보 등을 관리한다.

### Semantic Memory

Bot이 학습하거나 정리한 지식 및 개념적 정보를 관리한다.

### Episodic Memory

과거에 어떤 일을 했으며 어떤 결과가 발생했는지를 관리한다.

### Procedural Memory

특정 작업을 어떻게 수행해야 하는지에 대한 반복 가능한 방식이나 Routine을 관리한다.

### Working Memory

현재 수행 중인 작업과 사고에 필요한 일시적인 정보를 관리한다.

이 중 Working Memory는 Core마다 독립될 수 있지만, 장기적인 Bot Memory는 동일한 Bot에 속하는 모든 Core가 공유한다.

---

# 6. Brain

Brain은 Bot의 정체성, 기억, 목표와 실행 능력을 연결하는 논리적인 중심이다.

하나의 Bot에는 하나의 Brain이 존재하는 것으로 본다.

Brain은 특정 LLM 자체를 의미하지 않는다.

LLM은 Brain이 사고를 수행하기 위해 Harness를 통해 사용하는 하나의 실행 자원이다.

따라서 향후 하나의 Bot이 상황에 따라 서로 다른 모델을 사용할 수도 있다.

Bot은 Brain을 중심으로 자신의 Memory를 참조하고, Task를 판단하며, 필요한 Tool과 Core를 사용한다.

---

# 7. Core 개념

DXBOT에서 Core는 Bot의 복제본이나 독립적인 Sub-Agent가 아니다.

Core는 다음과 같이 정의한다.

> **Core는 하나의 Bot Brain이 동시에 여러 사고 및 작업 흐름에 관여하기 위해 사용하는 논리적 실행 단위다.**

하나의 Bot이 여러 일을 동시에 처리해야 할 경우 동일한 Bot 안에서 여러 Core가 동적으로 활성화될 수 있다.

예를 들어 하나의 Bot이 동시에 다음 작업을 수행할 수 있다.

* Core A — 프로젝트 구조 분석
* Core B — 테스트 수행
* Core C — 관련 기술 조사
* Core D — 결과 검토

이들은 서로 다른 Bot이 아니다.

모두 같은 Bot의 Identity와 Memory를 공유하면서 서로 다른 Working Context를 가진다.

---

# 8. Dynamic Core

Bot을 생성할 때 사용자가 Core 개수를 설정하지 않는다.

Core는 Bot의 고정된 물리 자원이 아니라 **런타임에서 필요에 따라 생성되고 회수되는 실행 개념**이다.

평상시에는 Core가 없거나 하나만 활성화되어 있을 수 있으며, 복잡하거나 병렬화 가능한 작업에서는 여러 Core가 동시에 활성화될 수 있다.

시스템은 전체적인 안정성, 비용, 모델 사용량 및 하드웨어 자원을 고려하여 **동시에 활성화할 수 있는 최대 Core 수(N)**만 정책적으로 제한한다.

따라서 개념적으로 다음과 같은 구조가 된다.

**Bot Brain**

→ Dynamic Core Scheduler
→ Core
→ Core
→ Core
→ ...

Core는 작업이 종료되면 다시 해제된다.

사용자는 Core Pool을 직접 관리하는 것이 아니라 Bot에게 일을 맡긴다.

**Core를 몇 개 사용할지 결정하는 것은 Bot과 Runtime의 책임이다.**

---

# 9. Shared Brain / Isolated Context

Core 간에는 완전한 독립성을 두지 않는다.

모든 Core는 동일한 Bot의 Brain에 속한다.

공유되는 영역은 다음과 같다.

* Identity
* Long-term Memory
* Semantic Memory
* Goals
* Bot-level Knowledge
* Global Task State
* Permissions

반면 각각의 Core는 자신이 맡은 작업을 수행하기 위한 별도의 Working Context를 가진다.

* Current Task
* Temporary Context
* Reasoning Flow
* Tool Execution State
* Temporary Artifacts

이를 통해 여러 Core가 동시에 작업하더라도 서로의 사고 흐름이 불필요하게 섞이지 않도록 한다.

동시에 Core가 발견한 중요한 결과는 Bot의 Shared Memory 또는 Shared State에 반영되어 다른 Core에서 활용될 수 있다.

즉 DXBOT은 다음 원칙을 따른다.

> **Brain은 공유하고, 사고 흐름은 분리한다.**

---

# 10. Bot-to-Bot Interaction

Bot은 Tool뿐 아니라 **다른 Bot을 호출할 수 있다.**

예를 들어 Main Bot이 특정 작업을 Developer Bot에게 요청하고, Developer Bot이 Research Bot에게 추가적인 조사를 요청할 수 있다.

Bot 간에는 다음과 같은 상호작용이 가능해야 한다.

* Request
* Message
* Delegate
* Ask
* Result
* Notify
* Collaborate

Bot을 호출한다는 것은 단순히 Sub-Agent를 생성하는 것과 다르다.

호출되는 Bot은 이미 자신만의 Identity와 Memory를 가지고 지속적으로 존재하는 독립적인 Bot이다.

따라서 Bot 간의 관계는 일회성 Parent-Child 관계보다 **지속적인 Bot Network**에 가깝다.

---

# 11. Multi-Bot System

DXBOT은 하나의 Bot만 실행하는 시스템이 아니라 여러 Bot을 하나의 Runtime에서 운영할 수 있도록 한다.

개념적으로 다음과 같은 형태가 가능하다.

**User**

→ Main Bot
→ Developer Bot
→ Research Bot
→ Review Bot
→ Operations Bot

각 Bot은 필요에 따라 자기 Core를 동적으로 확장할 수 있다.

따라서 시스템의 동시성 구조에는 두 가지 차원이 존재한다.

### Bot-level Concurrency

여러 Bot이 동시에 서로 다른 작업을 수행한다.

### Core-level Concurrency

하나의 Bot이 여러 Core를 이용하여 동시에 여러 작업을 수행한다.

DXBOT의 높은 병렬성은 이 두 계층을 결합하여 구현한다.

---

# 12. Bot Control Plane

여러 Bot이 존재하기 때문에 이를 중앙에서 관리하기 위한 **Bot Control Plane**이 필요하다.

Control Plane은 특정 Bot과 대화하는 인터페이스가 아니라 전체 Bot Ecosystem을 관찰하고 운영하는 시스템이다.

Control Plane에서는 개념적으로 다음 정보와 기능을 제공한다.

### Bot 관리

* Bot 생성
* Bot 제거
* Bot 활성화 / 비활성화
* Bot 상태 확인
* Bot 역할 및 설정 관리

### 실행 관리

* 현재 활성 Core
* 진행 중 Task
* 대기 중 Task
* 실행 상태
* Resource Usage

### Memory 관리

* Bot Memory 상태
* Memory 검색
* Memory 관리 및 정리
* Memory 간 관계 확인

### Communication 관리

* Bot 간 메시지
* Bot 간 Task 전달
* Bot 간 호출 관계
* 현재 협업 구조

### Runtime 관리

* Model
* Tool
* Permission
* Resource Limit
* Harness 상태

Control Plane은 향후 Web UI의 핵심적인 기능이 된다.

---

# 13. DeepSeek Harness Layer

DXBOT은 실행 엔진을 처음부터 완전히 새롭게 만드는 것보다 **DeepSeek Harness의 구조와 철학을 적극적으로 차용**하는 것을 기본 방향으로 한다.

Harness Layer는 Bot이 실제 세계와 상호작용하기 위한 실행 기반 역할을 담당한다.

개념적으로 Harness가 담당하는 영역은 다음과 같다.

* Model execution
* Agent loop
* Tool execution
* Skill
* Sandbox
* Storage integration
* Context handling
* Runtime lifecycle
* Scheduling
* Plugin system

DXBOT은 Harness 자체를 제품의 핵심 정체성으로 보지 않는다.

Harness는 **Bot의 사고와 행동을 실행시키는 Execution Substrate**다.

그 위에서 DXBOT 고유의 다음 개념을 구축한다.

* Persistent Bot
* Memory-Centric Runtime
* Brain
* Dynamic Core
* Bot Network
* Bot Control Plane

즉 전체적인 관계는 다음과 같다.

**DeepSeek-inspired Harness**

↓

**DXBOT Bot Runtime**

↓

**Bot / Brain / Memory / Core / Task**

↓

**CLI / TUI / Web**

---

# 14. Rust 기반 Runtime

DXBOT Core Runtime의 주 언어는 **Rust**를 기본 방향으로 한다.

Rust를 선택하는 이유는 DXBOT이 장기간 실행되는 상주형 Runtime이며, 동시에 다수의 Bot과 Core를 처리해야 하기 때문이다.

특히 중요하게 보는 요소는 다음과 같다.

* 높은 동시성 처리 능력
* 낮은 런타임 오버헤드
* 효율적인 메모리 관리
* 데이터 구조 및 메모리 배치 제어
* 높은 캐시 효율
* 장시간 실행 안정성
* 시스템 프로그래밍 수준의 제어 능력
* 대규모 비동기 작업 처리

DXBOT Core는 Rust를 중심으로 구성하고, 필요에 따라 Web UI와 일부 외부 Integration은 다른 기술을 사용할 수 있도록 한다.

---

# 15. Interface Strategy

DXBOT은 처음부터 Web UI를 중심으로 개발하지 않는다.

기능과 Runtime을 먼저 완성한 후 Interface를 단계적으로 확장한다.

우선순위는 다음과 같다.

**CLI → TUI → Web UI**

## CLI

DXBOT의 가장 기본적인 Interface다.

모든 핵심 기능은 UI 없이 CLI를 통해 사용할 수 있어야 한다.

CLI는 DXBOT Runtime의 기능을 검증하는 가장 직접적인 Reference Interface 역할을 한다.

## TUI

CLI 기능이 안정화된 이후 Codex CLI와 유사한 Interactive Terminal Experience를 제공한다.

개별 Bot과 대화하거나 작업 진행 상황, Tool 실행, Core 활동 등을 확인하는 데 적합하다.

## Web UI

Web UI는 단순히 채팅 UI를 제공하는 것이 목적이 아니다.

전체 Bot Ecosystem을 관리하는 **Control Center** 역할을 중심으로 한다.

Web UI에서는 여러 Bot, Core, Task, Memory, Bot 간 관계를 시각적으로 관리할 수 있어야 한다.

---

# 16. UI와 Runtime의 분리

DXBOT에서는 UI 안에 Bot의 핵심 로직을 구현하지 않는다.

CLI, TUI, Web UI는 동일한 DXBOT Runtime과 연결되는 서로 다른 표현 계층이어야 한다.

따라서 Bot의 동작은 Interface의 존재 여부와 관계없이 동일해야 한다.

이 구조를 통해 향후 다음과 같은 Interface 추가도 가능하다.

* Desktop App
* Mobile App
* Remote Client
* API
* IDE Integration
* Third-party Interface

---

# 17. DXBOT의 핵심 시스템 구조

전체 시스템은 개념적으로 다음과 같이 정리한다.

**Interface Layer**

CLI
TUI
Web UI
External API

↓

**Bot Control Plane**

Bot Management
Task Management
Resource Management
Monitoring
Communication

↓

**Bot Runtime**

Bot
Brain
Memory
Goals
Tasks
Dynamic Core Scheduler
Bot Network

↓

**Harness Layer**

Model
Tools
Skills
Sandbox
Context
Storage
Execution Loop

↓

**System Resources**

CPU
Memory
Filesystem
Network
External Services
Model Providers

---

# 18. DXBOT의 핵심 차별점

DXBOT의 차별점은 단순히 여러 Agent를 동시에 실행하는 것이 아니다.

### 1. Session-less Identity

Bot의 정체성이 세션에 종속되지 않는다.

### 2. Memory-Centric

대화 기록이 아니라 Memory가 Bot의 지속성을 결정한다.

### 3. Persistent Bot

Bot은 요청이 끝났다고 사라지지 않는다.

### 4. Shared Brain

한 Bot의 여러 작업은 동일한 Brain과 Memory를 공유한다.

### 5. Dynamic Core

Bot은 필요에 따라 자신의 실행 능력을 여러 Core로 동적으로 분산한다.

### 6. Multi-Bot

여러 Bot이 독립적으로 존재하고 서로를 호출하거나 협업할 수 있다.

### 7. Central Control

모든 Bot과 실행 상태를 하나의 Control Plane에서 관리할 수 있다.

### 8. Interface Independent

CLI, TUI, Web은 Bot 자체가 아니라 Bot Runtime에 접근하는 Interface다.

### 9. Harness Separation

Agent 실행 기술과 Bot이라는 제품 개념을 분리한다.

---

# 19. 제품 철학

DXBOT은 사용자가 AI에게 매번 새로운 요청을 던지는 시스템에서 벗어나고자 한다.

사용자는 매번 새로운 Agent를 생성하는 것이 아니라 **자신이 만들어 놓은 Bot에게 다시 일을 맡긴다.**

Bot은 이전에 무엇을 했는지 알고 있으며, 자신의 역할과 기억을 유지한다.

필요하면 동시에 여러 작업에 자신의 Brain을 분산하고, 자신의 능력만으로 해결하기 어려운 문제라면 다른 Bot에게 도움을 요청한다.

결과적으로 사용자는 개별 AI Session을 관리하는 것이 아니라 **AI Bot 조직을 운영하게 된다.**

---

# 20. 1차 개발 방향

초기 개발에서는 시각적인 완성도보다 DXBOT 고유의 Runtime 개념을 검증하는 것을 우선한다.

### Phase 1 — Harness Foundation

DeepSeek Harness의 구조를 분석하고 DXBOT이 사용할 실행 기반을 구축한다.

### Phase 2 — Persistent Bot Runtime

Bot Identity, Memory, Lifecycle을 중심으로 세션과 독립적인 Bot Runtime을 구현한다.

### Phase 3 — Core Runtime

하나의 Bot이 여러 작업을 동시에 수행할 수 있도록 Dynamic Core 개념을 구현한다.

### Phase 4 — Multi-Bot

Bot 간 호출, 메시징, Task 위임 및 협업 구조를 구현한다.

### Phase 5 — CLI

DXBOT의 전체 기능을 CLI에서 사용할 수 있도록 한다.

### Phase 6 — TUI

Interactive Terminal 환경에서 Bot과 Runtime 상태를 효율적으로 사용할 수 있도록 한다.

### Phase 7 — Control Plane / Web UI

다수의 Bot, Task, Core, Memory 및 Bot 간 관계를 통합 관리할 수 있는 Control Center를 구현한다.

---

# 21. 현재 단계에서 확정된 핵심 원칙

DXBOT 1차 컨셉에서는 다음 사항을 핵심 원칙으로 본다.

1. **DXBOT은 일반적인 Session 기반 Agent가 아니다.**
2. **Bot의 본체는 Memory와 Identity다.**
3. **Bot은 장기간 지속적으로 존재한다.**
4. **하나의 Bot에는 하나의 Brain이 존재한다.**
5. **Bot은 자신의 Brain을 여러 Core를 통해 동시에 여러 작업에 관여시킬 수 있다.**
6. **Core는 Bot의 복제본이 아니다.**
7. **Core 수는 Bot 생성 시 지정하지 않는다.**
8. **Runtime이 필요한 만큼 Core를 동적으로 활성화한다.**
9. **시스템은 최대 동시 Core 수만 정책적으로 제한한다.**
10. **Core는 Brain과 Memory를 공유하되 Working Context는 분리한다.**
11. **Bot은 다른 Bot을 호출하고 협업할 수 있다.**
12. **여러 Bot을 중앙 Control Plane에서 관리할 수 있다.**
13. **DeepSeek Harness의 구조를 실행 기반으로 적극 차용한다.**
14. **DXBOT Core Runtime은 Rust를 중심으로 개발한다.**
15. **기능 구현 우선순위는 CLI → TUI → Web UI 순서로 한다.**
16. **UI와 Bot Runtime은 완전히 분리한다.**

---

# 22. 프로젝트 방향 요약

DXBOT의 목표는 또 하나의 AI Agent CLI를 만드는 것이 아니다.

DXBOT은 **지속적인 기억을 가진 AI Bot이 하나의 컴퓨팅 엔티티처럼 존재하고, 자신의 Brain을 여러 작업에 동적으로 배분하며, 다른 Bot과 협력할 수 있는 새로운 Bot Runtime**을 만드는 프로젝트다.

DeepSeek Harness가 **어떻게 생각하고 행동할 것인가**를 담당한다면,

DXBOT은 그 위에서

**누가 존재하고, 무엇을 기억하며, 어떻게 동시에 일하고, 서로 어떻게 협력할 것인가**

를 정의한다.

이 차이가 DXBOT의 핵심이다.

---

# 상세 문서 생성 지시

위 「DXBOT 1차 컨셉 기획안」을 최상위 기준 문서로 삼아, 실제 개발에 사용할 수 있는 매우 상세한 세부 기획 문서를 주제별 여러 개의 Markdown(.md) 파일로 분리한다. 각 문서는 목적, 책임범위, 구성요소, 데이터 흐름, 상호작용, 예외상황, 확장성, 구현 우선순위, 검증기준을 명확히 정의한다. 설계 전반은 Rust 기반을 전제로 한다. 모든 구현 계획은 중복을 최소화하고 공통 모듈·추상화·인터페이스의 재사용성을 최우선으로 하며, 불필요한 복사·할당·상태중복을 피하고 메모리 사용량과 캐시 효율을 강하게 고려한다. 단기 편의보다 장기 유지보수성, 일관성, 확장성, 동시성 안전성을 우선하며 동일 기능의 중복 구현을 금지한다.

# AGENTS.md (Root)

이 문서는 `RepoPilot` 프로젝트의 **최상위 개발 규칙**이다.
세부 모듈 규칙은 `docs/agents/*.md`를 반드시 함께 따른다.

## 1. 절대 원칙 (Non-Negotiable)

1. Clean Architecture + DDD + Hexagonal Architecture(Ports & Adapters)를 **절대적으로 준수**한다.
2. 의존성 방향을 절대 역전시키지 않는다.
3. 빠른 구현보다 경계(boundary) 보존을 우선한다.
4. 아키텍처 위반이 발생하면 기능 추가보다 먼저 구조를 바로잡는다.
5. 포트는 `application`이 소유하고, 어댑터 구현은 `infrastructure/interface`에 둔다.

## 2. 계층/헥사고날 의존성 규칙

허용되는 의존성 방향:

- `interface -> application`
- `application -> domain`
- `infrastructure -> application(ports), domain`
- `main/lib(조립) -> 모든 계층`

Hexagonal 해석:

- `application`: 유스케이스 + 포트(inside)
- `interface`: 인바운드 어댑터(사용자 입력)
- `infrastructure`: 아웃바운드 어댑터(외부 시스템 연동)

금지:

- `domain -> application/infrastructure/interface`
- `application -> infrastructure/interface`
- `interface -> infrastructure` (단, main의 조립 단계에서 주입은 허용)

## 3. 현재 프로젝트 구조

- `src/domain`: 엔티티, 값 객체, 도메인 정책
- `src/application`: 유스케이스, 포트(인터페이스)
- `src/infrastructure`: 외부 시스템 연동 구현체
- `src/interface`: CLI 입출력 어댑터
- `src/main.rs`, `src/lib.rs`: composition root / 실행 진입점

## 4. 변경 시 필수 체크리스트

1. 새 코드가 어느 계층인지 먼저 명시한다.
2. 포트가 필요한지 먼저 판단하고, 구현체를 바로 만들지 않는다.
3. 도메인 규칙은 `domain`에, 흐름 제어는 `application`에 둔다.
4. 외부 API/CLI/파일/네트워크 접근은 `infrastructure`에만 둔다.
5. 신규 외부 연동은 `application` 포트 정의 후 `infrastructure/interface` 어댑터로 구현한다.
6. `cargo check` 통과를 기본 완료 조건으로 한다.
7. `cargo clippy -- -D warnings` 린트 검사를 반드시 통과한다.
8. 모든 소스 파일에는 최소 1개 이상의 주석(`//!`, `///`, `//`)을 포함한다.

## 5. 문서 우선순위

1. 이 루트 `AGENTS.md`
2. `docs/agents/README.md`
3. 해당 모듈 상세 문서 (`docs/agents/domain.md` 등)

충돌 시 더 상위 문서의 규칙을 따른다.

## 6. 금지 사항

- 편의상 계층을 건너뛰는 직접 참조
- 유스케이스 내부에서 구체 인프라 타입 직접 생성
- 도메인 타입 안에 HTTP/CLI/환경변수 접근 로직 혼합
- "일단 동작"을 이유로 구조 위반 커밋

## 7. 권장 커밋 단위

- `refactor(domain): ...`
- `refactor(application): ...`
- `refactor(infrastructure): ...`
- `refactor(interface): ...`

기능 커밋과 구조 커밋을 가능한 분리한다.

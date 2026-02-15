# Agents Guide Index

`RepoPilot`의 세부 개발 지침 모음.

기본 전제: 이 프로젝트는 `Clean Architecture + DDD + Hexagonal Architecture(Ports & Adapters)`를 함께 적용한다.

## 문서 목록

- `domain.md`: 도메인 모델/정책 작성 규칙
- `application.md`: 유스케이스/포트 작성 규칙
- `infrastructure.md`: 외부 연동 구현 규칙
- `interface.md`: CLI 인터페이스 규칙
- `testing.md`: 테스트 전략과 계층별 테스트 범위

## 적용 방법

1. 변경하려는 파일의 계층을 먼저 식별한다.
2. 해당 문서 규칙을 우선 적용한다.
3. 공통 규칙은 루트 `AGENTS.md`를 따른다.
4. 포트는 `application`에, 어댑터 구현은 `interface/infrastructure`에 둔다.

## 빠른 매핑

- `src/domain/**` -> `domain.md`
- `src/application/**` -> `application.md`
- `src/infrastructure/**` -> `infrastructure.md`
- `src/interface/**` -> `interface.md`

## Hexagonal 매핑

- Inside: `domain`, `application(usecase + ports)`
- Inbound Adapter: `interface` (CLI 입력)
- Outbound Adapter: `infrastructure` (VCS/Provider/Config 등 외부 연동)

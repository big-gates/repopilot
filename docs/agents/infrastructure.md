# Infrastructure Layer Guide

대상: `src/infrastructure/**`

## 역할

외부 시스템과 통신하는 기술 구현체를 제공한다.
Application 포트를 구현하며, Domain/Application을 침범하지 않는다.
Hexagonal 기준으로 `infrastructure`는 대표적인 아웃바운드 어댑터 계층이다.

## 하위 모듈 규칙

### 1) `config`
- 설정 파일 로딩/병합/검증 담당
- 환경변수/파일시스템 접근은 이곳에서만 수행
- 결과는 도메인/애플리케이션이 쓰기 쉬운 타입으로 제공

### 2) `vcs`
- GitHub/GitLab API 호출 담당
- HTTP 세부 포맷/헤더/인증 처리 담당
- 상위 계층에는 `ReviewComment`, `sha`, `diff` 등 의미 데이터만 전달

### 3) `providers`
- `codex`, `claude`, `gemini` CLI 실행 담당
- stdout/stderr 파싱, 재시도 전략(예: stdin not terminal) 담당
- 파싱 실패는 상위로 일관된 에러로 전달

### 4) `render`
- 코멘트 Markdown 포맷 책임
- 비즈니스 의사결정 로직 포함 금지

### 5) `adapters`
- Application 포트 구현체 조립
- 인프라 구체 타입 <-> 포트 타입 매핑
- 포트 계약은 변경하지 않고 구현만 제공

## 설계 규칙

1. 외부 장애/응답 오류를 문맥 포함 에러로 래핑한다.
2. 타임아웃/재시도 등 회복 전략은 인프라 레벨에서 처리한다.
3. 인프라 타입이 Domain 내부 규칙을 바꾸지 않는다.
4. 포트 인터페이스를 인프라에서 새로 정의하지 않고 `application` 포트를 구현한다.

## 리뷰 체크포인트

- 이 구현이 포트 계약을 정확히 지키는가?
- 외부 API 스키마 변경 영향이 인프라 경계를 넘지 않는가?

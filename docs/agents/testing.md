# Testing Guide (Clean Architecture + DDD + Hexagonal)

## 목표

계층별 책임을 독립적으로 검증하고, 경계 위반을 조기에 탐지한다.

## 계층별 테스트 전략

### Domain
- 순수 단위 테스트 중심
- 입력/출력 기반 정책 검증
- 외부 목(mock) 불필요

### Application
- 포트 목킹 기반 유스케이스 테스트
- 성공/실패/경계조건(중복 claim, force, dry-run) 시나리오 검증
- 인프라 없이 비즈니스 흐름이 재현되어야 함

### Infrastructure
- 어댑터/파서 테스트
- 외부 응답 포맷 변화에 대한 회귀 테스트
- 가능하면 네트워크 없는 테스트 우선

### Interface
- CLI 파싱 테스트
- 필수 인자 누락/서브커맨드 분기 테스트

## 최소 권장 케이스

1. 동일 SHA에 claim/final 마커 존재 시 skip
2. `--force`일 때 재실행
3. provider 일부 실패 시 전체 요약 생성
4. review guide 파일 적용 여부
5. token usage 파싱 실패 시 `n/a` 처리

## 아키텍처 검증

- 테스트에서도 계층 의존성 역전 금지
- integration 테스트에서만 실제 인프라 결합
- 포트 계약 테스트와 어댑터 계약 준수 테스트를 분리
- 유스케이스 테스트는 포트 목킹으로 inside 검증, 인프라 테스트는 adapter 동작 검증

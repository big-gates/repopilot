# prpilot

`prpilot`은 로컬 환경에서 실행되는 Rust CLI 도구로, 아래 대상을 멀티 에이전트로 코드 리뷰합니다.
- GitHub Pull Request
- GitLab Merge Request

사용자는 PR/MR URL 하나만 입력하면 되고, 플랫폼(GitHub/GitLab)은 자동으로 감지됩니다.

## 주요 기능

- URL만으로 실행: `prpilot "<PR_OR_MR_URL>"`
- GitHub/GitLab 자동 감지
- 멀티 프로바이더 리뷰 (Codex, Claude, Gemini)
- API 키 대신 **로컬에 설치/로그인된 CLI 명령** 실행
- API 기반 diff 조회 (로컬 checkout 불필요)
- claim/final 마커 기반 중복 실행 방지
- 에이전트별 개별 코멘트 작성 + 최종 요약 코멘트 작성
- 최종 요약 코멘트에 \"에이전트 간 상호 의견\" 포함
- 여러 config 파일 경로 병합/덮어쓰기 지원
- 현재 적용 config 확인 명령: `prpilot config`

## 아키텍처

`prpilot`은 **Clean Architecture + DDD + Hexagonal Architecture(Ports & Adapters)**를 함께 적용합니다.

핵심 원칙:
- `domain`은 가장 안쪽(inside core)이며 외부 기술을 모릅니다.
- `application`은 유스케이스와 포트(인터페이스)를 소유합니다.
- `interface`는 인바운드 어댑터(사용자 입력)입니다.
- `infrastructure`는 아웃바운드 어댑터(외부 API/CLI/파일)입니다.
- 의존성 방향은 안쪽으로만 향합니다.

의존성 규칙:
- 허용: `interface -> application`, `application -> domain`, `infrastructure -> application(ports), domain`
- 금지: `application -> infrastructure/interface`, `domain -> application/infrastructure/interface`

현재 코드 매핑:
- `src/domain`
  - 엔티티/값 객체/도메인 정책
  - 예: SHA 마커 정책, 교차 에이전트 프롬프트 정책, 사용량 집계 정책
- `src/application`
  - 유스케이스 + 포트
  - 예: `usecases/review_pr/*`, `ports.rs`
- `src/interface`
  - CLI + composition root
  - 예: `cli.rs`, `composition.rs`
- `src/infrastructure`
  - 아웃바운드 어댑터 구현
  - 예: `vcs/*`, `providers/*`, `config/*`, `render.rs`, `adapters/*`

## 사전 준비

`prpilot` 실행 전, 각 도구가 로컬에서 이미 로그인/설정되어 있어야 합니다.

- `codex`
- `claude`
- `gemini`

명령이 PATH에서 실행 가능해야 합니다.

## 설치 / 빌드

```bash
cargo build --release
```

소스에서 바로 실행:

```bash
cargo run --bin prpilot -- "https://github.com/org/repo/pull/123" --dry-run
```

빌드된 바이너리 실행:

```bash
./target/release/prpilot "https://github.com/org/repo/pull/123"
```

## 사용법

기본 명령:

```bash
prpilot "<PR_OR_MR_URL>"
```

예시:

```bash
prpilot "https://github.com/org/repo/pull/123"
prpilot "https://gitlab.com/group/subgroup/repo/-/merge_requests/45"
```

옵션:

- `--dry-run`: 최종 Markdown만 stdout에 출력하고 코멘트/노트는 작성하지 않음
- `--force`: 현재 HEAD SHA에 대해 이미 claim/review가 있어도 강제로 재실행

실행 흐름:
1. claim 코멘트 생성/업데이트
2. 각 에이전트 1차 리뷰 실행
3. 에이전트별 개별 코멘트 생성/업데이트
4. 각 에이전트가 다른 에이전트 의견에 대한 2차 코멘트 생성
5. claim 코멘트를 최종 요약 코멘트로 업데이트
6. 모든 에이전트 응답을 한국어로 통일해 출력

## 설정 (JSON)

`prpilot`은 아래 순서로 JSON config 파일을 읽고 병합합니다.
(낮은 우선순위 -> 높은 우선순위)

1. `/etc/prpilot/config.json`
2. `~/.config/prpilot/config.json` (OS 표준 config 디렉터리)
3. `./.prpilot/config.json`
4. `./prpilot.config.json`
5. `PRPILOT_CONFIG=/path/to/config.json` (최우선)

뒤에서 읽은 파일의 값이 앞의 값을 덮어씁니다.

### `prpilot.config.json` 예시

```json
{
  "defaults": {
    "max_diff_bytes": 120000,
    "system_prompt": "You are a strict senior code reviewer. Output Markdown with sections: Critical, Major, Minor, Suggestions.",
    "review_guide_path": "./review-guide.md",
    "comment_language": "ko"
  },
  "hosts": {
    "github.com": {
      "token_env": "GITHUB_TOKEN"
    },
    "gitlab.com": {
      "token_env": "GITLAB_TOKEN"
    }
  },
  "providers": {
    "openai": {
      "enabled": true,
      "command": "codex",
      "args": ["exec"]
    },
    "anthropic": {
      "enabled": true,
      "command": "claude",
      "args": []
    },
    "gemini": {
      "enabled": true,
      "command": "gemini",
      "args": []
    }
  }
}
```

### Provider 설정 필드

- `enabled`: provider 사용 여부 (`true`/`false`)
- `command`: 실행할 로컬 명령 이름 또는 경로
- `args`: 명령 인자 배열
- `use_stdin` (선택): 설정하지 않으면 기본값 `true`
- `defaults.review_guide_path`: 리뷰 지침 Markdown 파일 경로. 내용이 system prompt에 추가됨
- `defaults.comment_language`: 리뷰 결과 언어 (`ko` 또는 `en`, 기본값 `ko`)

추가 규칙:
- `use_stdin=false`일 때 `args` 안에 `{prompt}`가 있으면 치환해서 전달
- `use_stdin=false`이고 `{prompt}`가 없으면 프롬프트 문자열을 마지막 인자로 자동 추가

## 현재 적용 Config 확인

```bash
prpilot config
```

출력(JSON)에는 다음 정보가 포함됩니다.
- 탐색한 config 경로 목록 (`searched_paths`)
- 실제 로드된 경로 목록 (`loaded_paths`)
- 원본 defaults (`defaults`)
- 폴백 포함 최종 defaults (`effective_defaults`)
- host별 토큰 소스/해결 여부
- provider별 command/args/use_stdin 및 command 감지 여부

특정 파일로 강제 테스트:

```bash
PRPILOT_CONFIG=/tmp/config.json prpilot config
```

## 중복 실행 방지 (로컬 전용)

각 대상의 HEAD SHA마다 기존 코멘트/노트에서 아래 마커를 확인합니다.

- final marker: `<!-- prpilot-bot sha=<SHA> -->`
- claim marker: `<!-- prpilot-bot claim sha=<SHA> -->`

동작 순서:

1. 현재 HEAD SHA 조회
2. 동일 SHA의 마커가 이미 있으면 스킵 (`--force`면 진행)
3. 없으면 claim 코멘트/노트 생성 또는 업데이트
4. provider들을 병렬로 실행
5. claim 코멘트/노트를 최종 리뷰 코멘트로 업데이트

## 참고 사항

- 실제 코멘트 작성에는 해당 host의 VCS 토큰이 필요합니다.
- `--dry-run`은 코멘트 작성은 하지 않지만, private 저장소에서는 API 읽기 권한이 여전히 필요할 수 있습니다.
- diff가 `defaults.max_diff_bytes`를 초과하면 잘리고 `... (diff truncated)` 문구가 추가됩니다.
- provider 커맨드가 PATH에서 발견되지 않으면 해당 provider는 자동 제외됩니다.
- 일부 CLI가 `stdin is not a terminal` 오류를 내면 `prpilot`은 자동으로 stdin 없는 방식으로 1회 재시도합니다.
- 1차 리뷰/상호 코멘트 프롬프트는 영어로 구성되며, 최종 출력 언어는 `defaults.comment_language` 값으로 제어됩니다.

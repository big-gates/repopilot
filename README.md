# RepoPilot

`RepoPilot`은 로컬 환경에서 실행되는 Rust CLI 도구로, 아래 대상을 멀티 에이전트로 코드 리뷰합니다.
- GitHub Pull Request
- GitLab Merge Request

사용자는 PR/MR URL 하나만 입력하면 되고, 플랫폼(GitHub/GitLab)은 자동으로 감지됩니다.

## 주요 기능

- URL만으로 실행: `repopilot "<PR_OR_MR_URL>"`
- GitHub/GitLab 자동 감지
- 멀티 프로바이더 리뷰 (Codex, Claude, Gemini)
- API 키 대신 **로컬에 설치/로그인된 CLI 명령** 실행
- API 기반 diff 조회 (로컬 checkout 불필요)
- claim/final 마커 기반 중복 실행 방지
- 에이전트별 개별 코멘트 작성 + 최종 요약 코멘트 작성
- 최종 요약 코멘트에 \"에이전트 간 상호 의견\" 포함
- 여러 config 파일 경로 병합/덮어쓰기 지원
- 현재 적용 config 확인 명령: `repopilot config`

## 아키텍처

`RepoPilot`은 **Clean Architecture + DDD + Hexagonal Architecture(Ports & Adapters)**를 함께 적용합니다.

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
  - 인바운드 인터페이스 계층
  - 현재는 CLI 인터페이스만 구현: `src/interface/cli/*`
  - 예: `src/interface/cli/command.rs`, `src/interface/cli/repl.rs`, `src/interface/cli/composition.rs`
- `src/infrastructure`
  - 아웃바운드 어댑터 구현
  - 예: `vcs/*`, `providers/*`, `config/*`, `render.rs`, `adapters/*`

## 사전 준비

`RepoPilot` provider는 두 가지 실행 모드를 지원합니다.

- API 모드(권장): provider별 `api_key` 또는 `api_key_env` 설정
- CLI 모드: 로컬 바이너리(`codex`/`claude`/`gemini`) 설치 + 로그인

동작 우선순위: **API key가 있으면 API 모드**, 없으면 CLI 모드.

## 설치 / 빌드

```bash
cargo build --release
```

소스에서 바로 실행:

```bash
cargo run --bin repopilot -- "https://github.com/org/repo/pull/123" --dry-run
```

빌드된 바이너리 실행:

```bash
./target/release/repopilot "https://github.com/org/repo/pull/123"
```

## GitLab 배포 (Runner 없이)

러너가 없어도 로컬 머신에서 직접 배포할 수 있습니다.
`scripts/` 아래 스크립트가 **로컬 태그 푸시 -> 빌드 -> Package 업로드 -> Release 생성/업데이트**까지 처리합니다.

### 1) macOS/Linux에서 배포

```bash
GITLAB_TOKEN=<YOUR_TOKEN> \
scripts/publish-gitlab.sh \
  --project-id <PROJECT_ID> \
  --tag v0.1.0 \
  --gitlab-url https://gitlab.your-company.com
```

옵션:
- 태그 푸시를 건너뛰려면 `--no-tag-push`
- Release 생성/업데이트를 건너뛰려면 `--no-release`

### 2) Windows에서 배포

```powershell
$env:GITLAB_TOKEN=\"<YOUR_TOKEN>\"
.\\scripts\\publish-gitlab.ps1 `
  -ProjectId <PROJECT_ID> `
  -Tag v0.1.0 `
  -GitLabUrl https://gitlab.your-company.com
```

Windows 배포 스크립트도 기본적으로 태그를 생성/푸시합니다.
- 태그 푸시를 건너뛰려면 `-NoTagPush`
- Release 생성/업데이트를 건너뛰려면 `-NoRelease`

### 3) 사용자 설치 (전역 실행 가능)

macOS/Linux:
```bash
scripts/install-gitlab.sh \
  --project-id <PROJECT_ID> \
  --tag v0.1.0 \
  --gitlab-url https://gitlab.your-company.com \
  --token <YOUR_TOKEN>
```

Windows:
```powershell
.\\scripts\\install-gitlab.ps1 `
  -ProjectId <PROJECT_ID> `
  -Tag v0.1.0 `
  -GitLabUrl https://gitlab.your-company.com `
  -Token <YOUR_TOKEN>
```

설치 후 어느 경로에서든 아래처럼 실행됩니다.
```bash
repopilot --help
```

## 사용법

기본 명령:

```bash
repopilot "<PR_OR_MR_URL>"
```

대화형 모드(슬래시 커맨드):

```bash
repopilot
```

대화형 모드 시작 시 상태 대시보드가 먼저 출력됩니다.
- config 로딩 상태
- host/token 해석 상태
- provider별 실행 모드(api/cli)와 실행 가능 여부
- review guide 경로
- comment language

대화형 명령:
- `/`로 입력을 시작하면 실시간 명령 추천 표시 (방향키 이동 + Tab 자동완성 + Enter 실행)
- `/config`
- `/review <PR_OR_MR_URL> [--dry-run] [--force]`
- `/exit` 또는 `/quit`

예시:

```bash
repopilot "https://github.com/org/repo/pull/123"
repopilot "https://gitlab.com/group/subgroup/repo/-/merge_requests/45"
```

옵션:

- `--dry-run`: 최종 Markdown만 stdout에 출력하고 코멘트/노트는 작성하지 않음
- `--force`: 현재 HEAD SHA에 대해 이미 claim/review가 있어도 강제로 재실행

최초 실행 시 설정 파일이 없으면 아래 템플릿이 자동 생성됩니다.
- `./repopilot.config.json`
- `./review-guide.md`

실행 흐름:
0. 상태 대시보드 출력
1. claim 코멘트 생성/업데이트
2. 각 에이전트 1차 리뷰 실행
3. 에이전트별 개별 코멘트 생성/업데이트
4. 각 에이전트가 다른 에이전트 의견에 대한 2차 코멘트 생성
5. claim 코멘트를 최종 요약 코멘트로 업데이트
6. `defaults.comment_language` 설정값으로 에이전트 응답 언어를 통일

상태 대시보드에는 아래가 포함됩니다.
- Config 정상 로딩 여부
- Target Host
- Host Token 해석 여부 및 API 접근 검증 결과
- Provider별 enabled/mode(api|cli)/실행 가능 여부
- `review_guide_path` 및 파일 존재 여부
- `comment_language`

## 설정 (JSON)

`RepoPilot`은 아래 순서로 JSON config 파일을 읽고 병합합니다.
(낮은 우선순위 -> 높은 우선순위)

1. `/etc/repopilot/config.json`
2. `~/.config/repopilot/config.json` (OS 표준 config 디렉터리)
3. `./.repopilot/config.json`
4. `./repopilot.config.json`
5. `REPOPILOT_CONFIG=/path/to/config.json` (최우선)

뒤에서 읽은 파일의 값이 앞의 값을 덮어씁니다.

### `repopilot.config.json` 예시

```json
{
  "defaults": {
    "max_diff_bytes": 120000,
    "system_prompt": "You are a strict senior code reviewer. Output Markdown with sections: Critical, Major, Minor, Suggestions.",
    "review_guide_path": "./review-guide.md",
    "comment_language": "ko",
    "update_check_url": "https://gitlab.your-company.com/api/v4/projects/<PROJECT_ID>/releases/permalink/latest",
    "update_download_url": "https://gitlab.your-company.com/your-group/your-project/-/releases",
    "update_timeout_ms": 1200
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
      "api_key_env": "OPENAI_API_KEY",
      "model": "gpt-4.1-mini"
    },
    "anthropic": {
      "enabled": true,
      "api_key_env": "ANTHROPIC_API_KEY",
      "model": "claude-3-7-sonnet-latest"
    },
    "gemini": {
      "enabled": true,
      "api_key_env": "GEMINI_API_KEY",
      "model": "gemini-2.0-flash",
      "command": "gemini",
      "args": []
    }
  }
}
```

### Provider 설정 필드

- `enabled`: provider 사용 여부 (`true`/`false`)
- `api_key` / `api_key_env`: API 인증 키(또는 OAuth access token) 값/환경변수
- `api_base` (선택): API 베이스 URL override
- `model` (선택): provider 기본 모델 ID
- `command`: CLI 모드에서 실행할 로컬 명령 이름 또는 경로
- `args`: CLI 모드 명령 인자 배열
- `use_stdin` (선택): CLI 모드에서 프롬프트 전달 시 기본값 `true`
- `defaults.review_guide_path`: 리뷰 지침 Markdown 파일 경로. 내용이 system prompt에 추가됨
- `defaults.comment_language`: 리뷰 결과 언어 (`ko` 또는 `en`, 기본값 `ko`)
- `defaults.update_check_url`: 최신 버전 확인 endpoint (plain text 버전 문자열 또는 JSON)
- `defaults.update_download_url`: 업데이트 안내에 출력할 다운로드 URL (선택)
- `defaults.update_timeout_ms`: 업데이트 체크 타임아웃(ms, 기본 `1200`)

추가 규칙:
- `api_key` 또는 `api_key_env`가 설정되면 API 모드가 우선 사용됨
- API 키가 없을 때만 CLI 모드(`command`/`args`)를 사용함
- `use_stdin=false`일 때 `args` 안에 `{prompt}`가 있으면 치환해서 전달
- `use_stdin=false`이고 `{prompt}`가 없으면 프롬프트 문자열을 마지막 인자로 자동 추가

## 현재 적용 Config 확인

```bash
repopilot config
```

출력(JSON)에는 다음 정보가 포함됩니다.
- 탐색한 config 경로 목록 (`searched_paths`)
- 실제 로드된 경로 목록 (`loaded_paths`)
- 원본 defaults (`defaults`)
- 폴백 포함 최종 defaults (`effective_defaults`)
- host별 토큰 소스/해결 여부
- provider별 resolved mode(api/cli), runnable 여부, command/args/use_stdin 정보

특정 파일로 강제 테스트:

```bash
REPOPILOT_CONFIG=/tmp/config.json repopilot config
```

## 중복 실행 방지 (로컬 전용)

각 대상의 HEAD SHA마다 기존 코멘트/노트에서 아래 마커를 확인합니다.

- final marker: `<!-- repopilot-bot sha=<SHA> -->`
- claim marker: `<!-- repopilot-bot claim sha=<SHA> -->`

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
- API key가 설정되지 않았고 provider 커맨드가 PATH에서 발견되지 않으면 해당 provider는 자동 제외됩니다.
- 일부 CLI가 `stdin is not a terminal` 오류를 내면 CLI 모드에서 stdin 없는 방식으로 1회 재시도합니다.
- 1차 리뷰/상호 코멘트 프롬프트는 영어로 구성되며, 최종 출력 언어는 `defaults.comment_language` 값으로 제어됩니다.
- `defaults.update_check_url`이 설정되어 있으면 실행 시작 시 최신 버전이 있는지 확인하고, 새 버전이 있으면 업데이트 안내를 출력합니다.

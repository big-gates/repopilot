#!/usr/bin/env bash
# GitLab Generic Package Registry에서 prpilot 바이너리를 내려받아
# macOS/Linux 어디서든 `prpilot`로 실행 가능하도록 전역 경로에 설치한다.

set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  scripts/install-gitlab.sh --project-id <id> --tag <vX.Y.Z> [options]

Options:
  --project-id <id>        GitLab project numeric ID (required)
  --tag <tag>              Version tag, e.g. v0.1.0 (required)
  --token <token>          GitLab token for private project (optional)
  --gitlab-url <url>       GitLab base URL (default: https://gitlab.com)
  --package-name <name>    Generic package name (default: prpilot)
  --install-dir <dir>      Install directory override

Examples:
  scripts/install-gitlab.sh --project-id 123 --tag v0.1.0 --token <TOKEN>
  scripts/install-gitlab.sh --project-id 123 --tag v0.1.0 --gitlab-url https://gitlab.company.com
USAGE
}

detect_os() {
  case "$(uname -s)" in
    Darwin) echo "darwin" ;;
    Linux) echo "linux" ;;
    *)
      echo "error: unsupported OS for this installer: $(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *)
      echo "error: unsupported architecture for this installer: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

PROJECT_ID=""
TAG=""
TOKEN="${GITLAB_TOKEN:-${GL_TOKEN:-}}"
GITLAB_URL="${GITLAB_URL:-https://gitlab.com}"
PACKAGE_NAME="prpilot"
INSTALL_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --project-id)
      PROJECT_ID="${2:-}"
      shift 2
      ;;
    --tag)
      TAG="${2:-}"
      shift 2
      ;;
    --token)
      TOKEN="${2:-}"
      shift 2
      ;;
    --gitlab-url)
      GITLAB_URL="${2:-}"
      shift 2
      ;;
    --package-name)
      PACKAGE_NAME="${2:-}"
      shift 2
      ;;
    --install-dir)
      INSTALL_DIR="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown option: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$PROJECT_ID" || -z "$TAG" ]]; then
  echo "error: --project-id and --tag are required" >&2
  usage
  exit 1
fi

OS_NAME="$(detect_os)"
ARCH_NAME="$(detect_arch)"
ARCHIVE_NAME="${PACKAGE_NAME}-${TAG}-${OS_NAME}-${ARCH_NAME}.tar.gz"
DOWNLOAD_URL="${GITLAB_URL%/}/api/v4/projects/${PROJECT_ID}/packages/generic/${PACKAGE_NAME}/${TAG}/${ARCHIVE_NAME}"

if [[ -z "$INSTALL_DIR" ]]; then
  if [[ -w "/usr/local/bin" ]]; then
    INSTALL_DIR="/usr/local/bin"
  elif [[ -d "/opt/homebrew/bin" && -w "/opt/homebrew/bin" ]]; then
    INSTALL_DIR="/opt/homebrew/bin"
  else
    INSTALL_DIR="${HOME}/.local/bin"
  fi
fi

mkdir -p "$INSTALL_DIR"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
ARCHIVE_PATH="${TMP_DIR}/${ARCHIVE_NAME}"

CURL_ARGS=(--fail --show-error --location --output "$ARCHIVE_PATH")
if [[ -n "$TOKEN" ]]; then
  CURL_ARGS+=(--header "PRIVATE-TOKEN: ${TOKEN}")
fi

echo "downloading: ${DOWNLOAD_URL}"
curl "${CURL_ARGS[@]}" "$DOWNLOAD_URL"

echo "extracting package"
tar -C "$TMP_DIR" -xzf "$ARCHIVE_PATH"

if [[ ! -f "${TMP_DIR}/prpilot" ]]; then
  echo "error: extracted binary not found" >&2
  exit 1
fi

install -m 0755 "${TMP_DIR}/prpilot" "${INSTALL_DIR}/prpilot"

echo "installed: ${INSTALL_DIR}/prpilot"
echo

case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    echo "PATH already includes ${INSTALL_DIR}"
    ;;
  *)
    echo "PATH에 ${INSTALL_DIR}가 없습니다. 아래를 셸 설정 파일에 추가하세요:"
    echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
    ;;
esac

echo "check: prpilot --help"

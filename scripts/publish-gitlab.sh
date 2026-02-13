#!/usr/bin/env bash
# 로컬 머신(러너 없이)에서 prpilot 바이너리를 빌드하고
# GitLab Generic Package Registry + Release에 업로드한다.

set -euo pipefail

usage() {
  cat <<USAGE
Usage:
  scripts/publish-gitlab.sh --project-id <id> --tag <vX.Y.Z> [options]

Options:
  --project-id <id>        GitLab project numeric ID (required)
  --tag <tag>              Release tag, e.g. v0.1.0 (required)
  --token <token>          GitLab PAT (default: GITLAB_TOKEN or GL_TOKEN)
  --gitlab-url <url>       GitLab base URL (default: https://gitlab.com)
  --package-name <name>    Generic package name (default: prpilot)
  --no-release             Skip release creation API call

Examples:
  GITLAB_TOKEN=... scripts/publish-gitlab.sh --project-id 123 --tag v0.1.0
  scripts/publish-gitlab.sh --project-id 123 --tag v0.1.0 --gitlab-url https://gitlab.company.com
USAGE
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "error: '$1' command is required" >&2
    exit 1
  }
}

detect_os() {
  case "$(uname -s)" in
    Darwin) echo "darwin" ;;
    Linux) echo "linux" ;;
    *)
      echo "error: unsupported OS for this script: $(uname -s)" >&2
      exit 1
      ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "amd64" ;;
    aarch64|arm64) echo "arm64" ;;
    *)
      echo "error: unsupported architecture for this script: $(uname -m)" >&2
      exit 1
      ;;
  esac
}

PROJECT_ID=""
TAG=""
TOKEN="${GITLAB_TOKEN:-${GL_TOKEN:-}}"
GITLAB_URL="${GITLAB_URL:-https://gitlab.com}"
PACKAGE_NAME="prpilot"
CREATE_RELEASE="1"

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
    --no-release)
      CREATE_RELEASE="0"
      shift
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

if [[ -z "$TOKEN" ]]; then
  echo "error: token is missing (use --token or GITLAB_TOKEN/GL_TOKEN)" >&2
  exit 1
fi

require_cmd cargo
require_cmd curl
require_cmd tar
require_cmd shasum

OS_NAME="$(detect_os)"
ARCH_NAME="$(detect_arch)"
BIN_PATH="target/release/prpilot"
ARCHIVE_NAME="${PACKAGE_NAME}-${TAG}-${OS_NAME}-${ARCH_NAME}.tar.gz"
PACKAGE_URL="${GITLAB_URL%/}/api/v4/projects/${PROJECT_ID}/packages/generic/${PACKAGE_NAME}/${TAG}/${ARCHIVE_NAME}"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

echo "[1/4] building release binary"
cargo build --release

if [[ ! -f "$BIN_PATH" ]]; then
  echo "error: build output not found: $BIN_PATH" >&2
  exit 1
fi

echo "[2/4] packaging ${ARCHIVE_NAME}"
cp "$BIN_PATH" "$TMP_DIR/prpilot"
tar -C "$TMP_DIR" -czf "$ARCHIVE_NAME" prpilot
SHA256="$(shasum -a 256 "$ARCHIVE_NAME" | awk '{print $1}')"

echo "[3/4] uploading package -> ${PACKAGE_URL}"
curl --fail --show-error --silent \
  --header "PRIVATE-TOKEN: ${TOKEN}" \
  --upload-file "$ARCHIVE_NAME" \
  "$PACKAGE_URL" >/dev/null

echo "uploaded: ${ARCHIVE_NAME}"
echo "sha256 : ${SHA256}"

if [[ "$CREATE_RELEASE" == "1" ]]; then
  echo "[4/4] creating/updating release metadata"
  RELEASE_ENDPOINT="${GITLAB_URL%/}/api/v4/projects/${PROJECT_ID}/releases"
  RELEASE_JSON="${TMP_DIR}/release.json"

  cat > "$RELEASE_JSON" <<JSON
{
  "name": "${PACKAGE_NAME} ${TAG}",
  "tag_name": "${TAG}",
  "description": "${PACKAGE_NAME} ${TAG}\\n\\n- os: ${OS_NAME}\\n- arch: ${ARCH_NAME}\\n- sha256: ${SHA256}",
  "assets": {
    "links": [
      {
        "name": "${ARCHIVE_NAME}",
        "url": "${PACKAGE_URL}",
        "link_type": "package"
      }
    ]
  }
}
JSON

  if ! curl --fail --show-error --silent \
    --request POST \
    --header "PRIVATE-TOKEN: ${TOKEN}" \
    --header "Content-Type: application/json" \
    --data @"$RELEASE_JSON" \
    "$RELEASE_ENDPOINT" >/dev/null; then
    echo "warn: release creation failed (maybe already exists). package upload succeeded."
  else
    echo "release created: ${TAG}"
  fi
fi

echo
echo "Done."
echo "Install (mac/linux) example:"
echo "  scripts/install-gitlab.sh --project-id ${PROJECT_ID} --tag ${TAG} --gitlab-url ${GITLAB_URL} --token <TOKEN>"

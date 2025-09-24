#!/usr/bin/env bash

set -euo pipefail

REPO_URL="https://github.com/base/node.git"
DEFAULT_REF="main"

usage() {
  cat <<'EOF'
Usage: build-base-node-image.sh <client> [docker build args...]

Arguments:
  <client>             Which execution client to build. Must be one of: geth, reth.
  [docker build args]  Additional arguments forwarded to docker build.

Environment variables:
  BASE_NODE_REPO  Override the git repository to clone. Defaults to https://github.com/base/node.git.
  BASE_NODE_REF   Git ref (branch, tag, or commit) to checkout after cloning. Defaults to main.

Examples:
  ./build-base-node-image.sh geth
  ./build-base-node-image.sh reth --build-arg FOO=bar
EOF
}

if [[ ${1:-} == "-h" || ${1:-} == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -lt 1 ]]; then
  echo "error: missing required <client> argument" >&2
  usage
  exit 1
fi

CLIENT=$1
shift || true

case "$CLIENT" in
  geth|reth)
    ;;
  *)
    echo "error: unsupported client '$CLIENT'; expected 'geth' or 'reth'" >&2
    exit 1
    ;;
esac

REPO_URL=${BASE_NODE_REPO:-$REPO_URL}
GIT_REF=${BASE_NODE_REF:-$DEFAULT_REF}

if ! command -v git >/dev/null 2>&1; then
  echo "error: git is required but not installed" >&2
  exit 1
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is required but not installed" >&2
  exit 1
fi

TMP_DIR=$(mktemp -d 2>/dev/null || mktemp -d -t "base-node")
cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

echo "Cloning $REPO_URL into $TMP_DIR" >&2
git clone "$REPO_URL" "$TMP_DIR"

pushd "$TMP_DIR" >/dev/null

if [[ -n "$GIT_REF" ]]; then
  echo "Checking out ref $GIT_REF" >&2
  git checkout "$GIT_REF"
fi

IMAGE_NAME="base-node-$CLIENT"
DOCKERFILE_PATH="$CLIENT/Dockerfile"

if [[ ! -f "$DOCKERFILE_PATH" ]]; then
  echo "error: expected Dockerfile at $DOCKERFILE_PATH" >&2
  exit 1
fi

echo "Building Docker image '$IMAGE_NAME' using $DOCKERFILE_PATH" >&2
docker build -t "$IMAGE_NAME" -f "$DOCKERFILE_PATH" "$@" .

popd >/dev/null

echo "Successfully built image $IMAGE_NAME" >&2

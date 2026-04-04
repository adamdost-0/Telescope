#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
IMAGE="${IMAGE:-telescope-devtest:local}"
PNPM_VERSION=9.15.4

usage() {
  cat <<'EOF'
Run Telescope commands inside the shared dev/test container.

Usage:
  ./scripts/dev-test.sh [test]
  ./scripts/dev-test.sh shell
  ./scripts/dev-test.sh run <command...>
  ./scripts/dev-test.sh build-image

Modes:
  test        Build the image and run the repo validation subset (default)
  shell       Open an interactive bash shell inside the container
  run         Execute an arbitrary command inside the container
  build-image Only build the reusable container image
EOF
}

build_image() {
  docker build -f "$ROOT_DIR/tools/devtest/Dockerfile" -t "$IMAGE" "$ROOT_DIR"
}

run_in_container() {
  local command="$1"
  local tty_flags=()
  local env_flags=(
    -e PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
    -e PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
  )

  if [[ -t 0 && -t 1 ]]; then
    tty_flags=(-it)
  elif [[ -t 1 ]]; then
    tty_flags=(-t)
  fi

  if [[ -n "${CI:-}" ]]; then
    env_flags+=(-e "CI=${CI}")
  fi

  docker run --rm "${tty_flags[@]}" \
    --init \
    --ipc=host \
    -v "$ROOT_DIR":/repo \
    -v telescope-cargo-registry:/home/pwuser/.cargo/registry \
    -v telescope-cargo-git:/home/pwuser/.cargo/git \
    -v telescope-rustup:/home/pwuser/.rustup \
    -v telescope-pnpm-store:/home/pwuser/.local/share/pnpm/store \
    -w /repo \
    "${env_flags[@]}" \
    "$IMAGE" \
    bash -lc "
      set -euo pipefail
      cd /repo
      COREPACK_HOME=/repo/.corepack corepack prepare pnpm@${PNPM_VERSION} --activate >/dev/null
      ${command}
    "
}

MODE="${1:-test}"

case "$MODE" in
  test)
    build_image
    CI=1 run_in_container '
      rustc --version
      cargo --version
      node --version
      npm --version
      pnpm --version

      echo "== Rust (fmt/clippy/test) =="
      cargo fmt --all -- --check
      cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
      cargo test --workspace --exclude telescope-desktop --all-features

      echo "== pnpm install =="
      ./scripts/pnpm.sh install --frozen-lockfile

      echo "== Web unit tests =="
      ./scripts/pnpm.sh -C apps/web test

      echo "== Web E2E =="
      ./scripts/pnpm.sh -C apps/web e2e
    '
    ;;
  shell)
    build_image
    run_in_container 'exec bash'
    ;;
  run)
    shift || true
    if (($# == 0)); then
      usage >&2
      exit 1
    fi
    build_image
    printf -v escaped_command '%q ' "$@"
    run_in_container "$escaped_command"
    ;;
  build-image)
    build_image
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "Unknown mode: $MODE" >&2
    usage >&2
    exit 1
    ;;
esac

#!/usr/bin/env bash
set -euo pipefail

IMAGE=telescope-devtest:local

# Build a deterministic dev test container with Rust + Node + Playwright deps.
docker build --pull=false -f tools/devtest/Dockerfile -t "$IMAGE" .

# Run tests inside container. We mount repo at /repo.
docker run --rm -t \
  -v "$PWD":/repo \
  -w /repo \
  -e CI=1 \
  "$IMAGE" \
  bash -lc '
    set -euo pipefail
    rustc --version
    cargo --version
    node --version
    npm --version

    echo "== Rust (fmt/clippy/test) =="
    cargo fmt --all -- --check
    cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
    cargo test --workspace --exclude telescope-desktop --all-features

    echo "== pnpm install =="
    ./scripts/pnpm.sh install --frozen-lockfile

    echo "== Web unit tests =="
    ./scripts/pnpm.sh -C apps/web test

    echo "== Web E2E =="
    # Playwright Docker base image already includes the OS deps and browsers.
    # Avoid installing browsers at runtime to keep the loop fast/deterministic.
    export PLAYWRIGHT_BROWSERS_PATH=/ms-playwright
    ./scripts/pnpm.sh -C apps/web e2e
  '

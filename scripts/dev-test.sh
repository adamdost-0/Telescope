#!/usr/bin/env bash
set -euo pipefail

IMAGE=telescope-devtest:local

# Build a deterministic dev test container with Rust + Node + Playwright deps.
docker build -f tools/devtest/Dockerfile -t "$IMAGE" .

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

    echo "== Node install =="
    npm ci

    echo "== Web unit tests =="
    npm -w apps/web test

    echo "== Web E2E =="
    npx playwright install --with-deps
    npm -w apps/web run e2e
  '

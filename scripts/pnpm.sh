#!/usr/bin/env bash
set -euo pipefail

# Repo-local pnpm runner (no global install required).
# Uses corepack-prepared pnpm stored under .corepack.

PNPM_CJS="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.corepack/v1/pnpm/9.15.4/bin/pnpm.cjs"

if [[ ! -f "$PNPM_CJS" ]]; then
  echo "pnpm not prepared. Run: COREPACK_HOME=./.corepack corepack prepare pnpm@9.15.4 --activate" >&2
  exit 1
fi

exec node "$PNPM_CJS" "$@"

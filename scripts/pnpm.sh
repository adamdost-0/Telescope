#!/usr/bin/env bash
set -euo pipefail

# Repo-local pnpm runner (no global install required).
# Uses corepack-prepared pnpm stored under .corepack.

PNPM_CJS="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.corepack/v1/pnpm/9.15.4/bin/pnpm.cjs"

if [[ ! -f "$PNPM_CJS" ]]; then
  # Auto-prepare pnpm into the repo-local Corepack home (works in CI and dev containers).
  REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  export COREPACK_HOME="$REPO_ROOT/.corepack"

  # corepack is shipped with Node; don't require global pnpm.
  corepack prepare pnpm@9.15.4 --activate >/dev/null
fi

exec node "$PNPM_CJS" "$@"

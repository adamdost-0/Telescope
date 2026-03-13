# Agent Guidance — CI/CD Workflows

## Overview

`.github/workflows/` contains five GitHub Actions CI/CD pipelines. All PRs and pushes to `main` run through `ci.yml`. Specialized workflows handle integration tests, desktop artifact builds, and releases.

## Workflows

### 1. `ci.yml` — Main CI (PR + push to main)

Runs on every PR and push to `main`. Contains four parallel jobs:

#### Job: `rust` (ubuntu-latest)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

`telescope-desktop` excluded on Linux — GTK/WebKit system deps unavailable. Desktop CI runs separately.

#### Job: `web` (ubuntu-latest)

```bash
pnpm install --frozen-lockfile
pnpm -C apps/web test          # Vitest unit tests
pnpm -C apps/web build         # Static build validation (⚠️ this is also what "lint" runs)
```

Note: `pnpm -r --if-present lint` is NOT run in CI. `apps/web`'s `lint` script runs the build.

#### Job: `web-e2e` (ubuntu-latest, depends on `web`)

```bash
pnpm install --frozen-lockfile
pnpm -C apps/web exec playwright install --with-deps chromium   # chromium only
pnpm -C apps/web e2e
```

E2E tests spin up two local servers (see `playwright.config.ts`):
1. `tests-e2e/stub/stub-server.mjs` — fake `/api/v1/*` responses at port 4274
2. Vite dev server at port 4273 with `PUBLIC_ENGINE_HTTP_BASE=http://127.0.0.1:4274`

Tests do NOT connect to a real Kubernetes cluster.

#### Job: `desktop-build` (matrix: `[windows-latest, macos-latest]`)

```bash
pnpm install --frozen-lockfile
pnpm -C apps/desktop build     # Tauri debug build
```

Validates desktop compiles on both target platforms. Does NOT produce release artifacts — use `build-desktop.yml` or `release.yml` for that.

### 2. `integration.yml` — Engine Integration Tests (k3d)

**Trigger:** Manual (`workflow_dispatch`) or push to `main` when `crates/engine/**`, `crates/core/**`, or `tools/k3d-fixtures/**` change.

```bash
# Installs k3d, creates a local cluster, applies fixtures, runs tests
k3d cluster create telescope-ci --agents 1 --wait --timeout 120s
kubectl apply -f tools/k3d-fixtures/
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d -- --nocapture
k3d cluster delete telescope-ci
```

Runs integration tests in `crates/engine/tests/integration_k3d.rs` against a real (local) Kubernetes cluster.

### 3. `build-desktop.yml` — Standalone Desktop Artifact Build

**Trigger:** Manual (`workflow_dispatch`) or push to `main` when `crates/**`, `apps/desktop/**`, or key `apps/web` files change.

Runs on Windows only (as of current config). Builds a full release bundle (`pnpm -C apps/desktop bundle`) and uploads artifacts:
- `telescope-windows-exe` — raw `telescope-desktop.exe`
- `telescope-windows-installers` — MSI/NSIS installer bundles (if present)

Artifact retention: 14 days.

### 4. `release.yml` — Tagged Release

**Trigger:** Push of any `v*` tag.

```yaml
permissions: { contents: write }   # required to create GitHub Release
```

Steps per platform (matrix: `[windows-latest, macos-latest]`):
1. Extract version from tag (`v0.1.0` → `0.1.0`)
2. Stamp all `Cargo.toml` crate versions + `tauri.conf.json` `version` field
3. `pnpm -C apps/desktop bundle` — full release installer
4. Upload versioned binary + bundle dirs to GitHub Release (via `softprops/action-gh-release@v2`)

The release includes: raw binary, MSI/NSIS (Windows), DMG/macOS app (macOS).

## CI Enforcement Summary

| Check | Enforced | Notes |
|---|---|---|
| Rust formatting | ✅ | `cargo fmt --check` |
| Rust linting | ✅ | `cargo clippy -D warnings` |
| Rust unit tests | ✅ | `cargo test --all-features` |
| Web unit tests | ✅ | Vitest |
| Web static build | ✅ | Validates SvelteKit builds cleanly |
| Web E2E tests | ✅ | Playwright against stub (chromium only) |
| Desktop debug build | ✅ | Win + macOS only |
| Engine integration tests | ⚠️ Conditional | `integration.yml` — requires k3d; triggers on engine/core changes |
| Desktop release bundle | ⚠️ Conditional | `build-desktop.yml` — manual or path-triggered |
| JavaScript linting | ❌ | No ESLint configured; `lint` runs build |
| Security scanning | ❌ | No Dependabot, `cargo audit`, or CodeQL |
| Code coverage | ❌ | No coverage reporting |
| Hub deployment | ❌ | No CD pipeline |

## Concurrency

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

Multiple pushes to the same branch cancel older in-progress runs.

## Common Failure Modes

1. **Rust format:** Fix with `cargo fmt --all`. Verify with `cargo fmt --all -- --check`.

2. **Clippy warnings:** Fix with `cargo clippy --fix --workspace --all-targets --all-features`. Verify with full clippy command.

3. **Rust test failures:** Run `cargo test --workspace --exclude telescope-desktop --all-features` locally.

4. **Frozen lockfile mismatch:** Run `pnpm install` locally and commit updated `pnpm-lock.yaml`.

5. **Web unit test failure:** Run `pnpm -C apps/web test` locally. Check Vitest output.

6. **Web build failure:** Run `pnpm -C apps/web build` locally. Common causes: TypeScript errors, missing imports, Svelte 5 syntax violations.

7. **E2E failure:** Run `pnpm -C apps/web e2e` locally. The stub server is in `tests-e2e/stub/stub-server.mjs`. Check if stub data matches what the test expects.

8. **Desktop build failure:** Check matrix-specific CI logs (Windows vs. macOS). Verify Tauri deps in `apps/desktop/src-tauri/Cargo.toml`.

## Adding a New Check

```yaml
jobs:
  new-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run check
        run: ./scripts/my-check.sh
```

Use `needs: [rust, web]` for dependencies. Keep jobs independent for parallel execution.

## CI Action Versions (Use These)

- `actions/checkout@v4`
- `actions/setup-node@v4` with `node-version: 22`
- `dtolnay/rust-toolchain@stable`
- `Swatinem/rust-cache@v2`
- `actions/upload-artifact@v4`
- `softprops/action-gh-release@v2`
- Enable `corepack` for pnpm: `run: corepack enable`
- Always use `pnpm install --frozen-lockfile` for deterministic installs
- Set `permissions: { contents: read }` by default (least privilege)

## Agent Delivery Policy

After validated changes are complete:
1. Commit and push the branch upstream
2. Create and push a `v*` tag to trigger `release.yml`
3. If a version isn't specified, continue the existing SemVer sequence

## What's Missing (High Priority)

1. ESLint / `svelte-eslint-parser` for real JavaScript linting
2. `cargo audit` or Dependabot for Rust supply-chain security
3. Code coverage reporting (Codecov, Coveralls, etc.)
4. Hub CD pipeline (deploy to container registry)
5. Docker image publishing for `apps/hub`
6. Linux desktop CI builds (requires Docker or system dep installation)
7. Changelog automation (release notes generator)
8. Multi-cluster E2E tests (against real AKS/k3d)

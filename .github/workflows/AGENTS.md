# Agent Guidance — CI/CD Workflows

## Overview

`.github/workflows/` contains GitHub Actions CI pipelines for code quality, testing, and release.

**Primary workflow:** `ci.yml` — runs on all PRs and pushes to `main`.

**Release workflow:** `release.yml` — runs on pushed Git tags matching `v*`.

## Current CI Jobs (`ci.yml`)

### 1. Rust Job (`rust`)

Runs on: `ubuntu-latest`

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

`telescope-desktop` is excluded on Linux (GTK/WebKit system deps). Desktop builds run separately on Windows/macOS.

### 2. Web Job (`web`)

Runs on: `ubuntu-latest`

```bash
pnpm install --frozen-lockfile
pnpm -C apps/web test      # Vitest unit tests
pnpm -C apps/web build     # Production build
```

### 3. Web E2E Job (`web-e2e`)

Runs on: `ubuntu-latest` (depends on `web` job)

```bash
pnpm install --frozen-lockfile
pnpm -C apps/web exec playwright install --with-deps chromium
pnpm -C apps/web e2e       # Playwright tests against stub server
```

E2E tests run against `tools/devtest/stub-server.mjs` with deterministic fake data (no live K8s cluster).

### 4. Desktop Build Job (`desktop-build`)

Runs on: **Matrix** — `[windows-latest, macos-latest]`

```bash
pnpm install --frozen-lockfile
pnpm -C apps/desktop build   # Tauri debug build
```

Platform-specific native dependencies:
- **macOS:** Xcode command-line tools
- **Windows:** Windows SDK
- **Linux:** Excluded (GTK/WebKit system deps not in CI)

## Release Workflow (`release.yml`)

Runs on: pushed tags matching `v*`

**Matrix:** `windows-latest`, `macos-latest`

Steps:
1. Extract version from tag
2. Stamp version into `Cargo.toml` manifests and `tauri.conf.json`
3. `pnpm install --frozen-lockfile`
4. Configure macOS signing/notarization env (Developer ID + notarization when secrets are present, ad-hoc fallback otherwise)
5. `pnpm -C apps/desktop bundle` — full release build
6. Verify macOS `.app` signature integrity and app signature inside DMG
7. Rename binary with version tag
8. Create GitHub Release via `softprops/action-gh-release@v2`

**Release artifacts:**
- Windows: versioned `.exe`, MSI installer, NSIS installer
- macOS: versioned binary and DMG (with signature integrity checks enforced in CI)

**Version stamping targets:**
- `apps/desktop/src-tauri/Cargo.toml`
- `crates/core/Cargo.toml`
- `crates/engine/Cargo.toml`
- `apps/desktop/src-tauri/tauri.conf.json`

## CI Enforcement Summary

| Check | Enforced | Notes |
|-------|----------|-------|
| Rust formatting | [ok] | `cargo fmt --check` |
| Rust linting | [ok] | `cargo clippy -D warnings` |
| Rust tests | [ok] | `cargo test --all-features` |
| Web unit tests | [ok] | Vitest via `pnpm -C apps/web test` |
| Web build | [ok] | `pnpm -C apps/web build` |
| Web E2E tests | [ok] | Playwright against stub server |
| Desktop builds | [ok] | Windows + macOS matrix |
| Tagged releases | [ok] | `release.yml` on `v*` tags |
| Security scanning | [fail] | No Dependabot, CodeQL, or audit checks |

## Concurrency

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

Multiple pushes to the same branch cancel older runs.

## CI Toolchain

- `actions/checkout@v4`
- `dtolnay/rust-toolchain@stable`
- `Swatinem/rust-cache@v2`
- `actions/setup-node@v4` with `node-version: 22`
- `corepack enable` for pnpm support
- `--frozen-lockfile` for deterministic installs
- `permissions: { contents: read }` for CI, `{ contents: write }` for releases

## Debugging CI Failures

1. **Reproduce locally first:** Run `./scripts/dev-test.sh` to reproduce failures in the same containerized environment CI uses. This is the fastest path to diagnosing fmt, clippy, test, and E2E issues.
2. **Rust format:** Fix with `cargo fmt --all`, verify with `cargo fmt --all -- --check`
3. **Clippy warnings:** Fix with `cargo clippy --fix --workspace --all-targets --all-features`
4. **Test failures:** Run locally: `cargo test --workspace --all-features` or `pnpm -C apps/web test`
5. **Frozen lockfile:** Run `pnpm install` locally, commit updated `pnpm-lock.yaml`
6. **Desktop build:** Check matrix-specific logs (Windows vs macOS)

## Local CI Mirror

The `./scripts/dev-test.sh` script runs the same checks as `ci.yml` jobs (`rust`, `web`, `web-e2e`) inside the dev container. Agents and contributors should use it as the primary local validation gate before pushing to any branch. This keeps `main` green and reduces CI churn.

```bash
# Equivalent of CI rust + web + web-e2e jobs
./scripts/dev-test.sh
```

## Agent Delivery Policy

After validated changes are finished:
1. Run `./scripts/dev-test.sh` to confirm the full validation suite passes in the dev container
2. Commit the completed work
3. Push the branch upstream
4. Create and push a release tag matching `v*` to trigger the release workflow

**Gate rule:** Never push a branch or create a tag without a passing `./scripts/dev-test.sh` run. This prevents broken code from reaching `main` or triggering a release.

If the user does not provide a version, continue the existing SemVer-style tag sequence.

## When to Edit

- **Add validation:** Add a job or step in `ci.yml`
- **Change build matrix:** Edit `strategy.matrix` for platform coverage
- **Add deployment:** Create a separate workflow (don't mix with CI)

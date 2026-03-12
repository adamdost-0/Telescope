# Agent Guidance — CI/CD Workflows

## Overview

`.github/workflows/` contains GitHub Actions CI pipelines that enforce code quality, testing, and build validation.

**Primary workflow:** `ci.yml` — runs on all PRs and pushes to `main`.

## Current CI Jobs

### 1. Rust Job (`rust`)

Runs on: `ubuntu-latest`

Validates:
```bash
cargo fmt --all -- --check          # Format check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

**Note:** `telescope-desktop` is excluded on Linux because of platform-specific GTK/WebKit dependencies. Desktop builds run separately on Windows/macOS.

### 2. Web Job (`web`)

Runs on: `ubuntu-latest`

Validates:
```bash
pnpm install --frozen-lockfile
pnpm -r --if-present lint           # Runs lint script in all packages (many are no-ops)
pnpm -r --if-present test           # Runs test script in all packages
```

**Current reality:**
- `apps/web/lint` is a no-op (needs ESLint setup)
- `apps/desktop/lint` is a no-op
- `packages/ui/lint` and `packages/ui/test` are no-ops (package is empty)

### 3. Web E2E Job (`web-e2e`)

Runs on: `ubuntu-latest` (depends on `web` job)

Validates:
```bash
pnpm install --frozen-lockfile
pnpm -C apps/web exec playwright install --with-deps
pnpm -C apps/web e2e                # Playwright tests against stub server
```

**Test setup:** E2E tests run against `tools/devtest/stub-server.mjs` with deterministic fake data (no live K8s cluster).

### 4. Desktop Build Job (`desktop-build`)

Runs on: **Matrix** — `[windows-latest, macos-latest]`

Validates:
```bash
pnpm install --frozen-lockfile
pnpm -C apps/desktop build          # Tauri debug build
```

**Why matrix?** Desktop has platform-specific native dependencies:
- **macOS:** Xcode command-line tools
- **Windows:** Windows SDK
- **Linux:** Excluded (GTK/WebKit system deps not in CI environment)

## CI Enforcement Summary

What CI actually enforces (vs. what's aspirational):

| Check | Enforced | Notes |
|-------|----------|-------|
| Rust formatting | ✅ Yes | `cargo fmt --check` |
| Rust linting | ✅ Yes | `cargo clippy -D warnings` |
| Rust tests | ✅ Yes | `cargo test --all-features` |
| Web unit tests | ✅ Yes | `pnpm test` runs Vitest |
| Web E2E tests | ✅ Yes | Playwright against stub server |
| JavaScript linting | ⚠️ Partial | `pnpm lint` runs but many scripts are no-ops |
| Desktop builds | ✅ Yes | On Windows/macOS only |
| Hub deployment | ❌ No | No deployment pipeline yet |
| Security scanning | ❌ No | No Dependabot, CodeQL, or audit checks |

## Concurrency

```yaml
concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true
```

Multiple pushes to the same branch cancel older runs (saves CI time).

## What's Missing

High-priority CI gaps:

1. **Real JavaScript linting:** ESLint + svelte-eslint-parser not configured
2. **Security scanning:** No Dependabot, `cargo audit`, or CodeQL
3. **Code coverage:** No coverage reporting (Codecov, Coveralls, etc.)
4. **Release automation:** No tagged release builds or artifact publishing
5. **Deployment pipelines:** No CD for `apps/hub` or `apps/web`
6. **Docker image publishing:** `apps/hub` Dockerfile exists but not published
7. **Changelog automation:** No release notes generation
8. **Linux desktop builds:** Excluded due to system deps (could use Docker)

## Adding a New Check

To add a validation step:

1. Add a new job in `.github/workflows/ci.yml`
2. Use `needs: [...]` for dependencies between jobs
3. Keep jobs independent when possible (parallel execution)
4. Use matrix strategy for multi-platform checks
5. Cache dependencies (`actions/cache`, `rust-cache`, etc.)

Example:
```yaml
jobs:
  new-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run check
        run: ./scripts/my-check.sh
```

## Debugging CI Failures

Common failure modes:

1. **Rust format failure:**
   - Fix locally: `cargo fmt --all`
   - Verify: `cargo fmt --all -- --check`

2. **Clippy warnings:**
   - Fix locally: `cargo clippy --fix --workspace --all-targets --all-features`
   - Verify: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

3. **Test failures:**
   - Run locally: `cargo test --workspace --all-features` (Rust) or `pnpm test` (JS)
   - Check for flaky tests (E2E timing issues)

4. **Frozen lockfile mismatch:**
   - Run `pnpm install` locally to regenerate `pnpm-lock.yaml`
   - Commit the updated lockfile

5. **Desktop build failures:**
   - Check matrix-specific logs (Windows vs. macOS)
   - Verify Tauri dependencies are correct in `src-tauri/Cargo.toml`

## CI Performance

Current CI run time (approximate):

- **Rust job:** ~3-5 minutes (with cache)
- **Web job:** ~2-3 minutes
- **Web E2E:** ~3-5 minutes (Playwright install + tests)
- **Desktop builds:** ~5-10 minutes per platform

Total: ~10-15 minutes for full CI pass (jobs run in parallel).

## Future Enhancements

Planned but not implemented:

- **Nightly builds:** Scheduled workflow for continuous integration
- **Release workflow:** Triggered on version tags, publishes artifacts
- **Deployment workflow:** CD pipeline for `apps/hub` and `apps/web`
- **Benchmark tracking:** Performance regression detection
- **Multi-cluster E2E:** Test against real K8s clusters (GKE, AKS, EKS)

## Code Conventions

- Use `actions/checkout@v4` (latest stable)
- Use `dtolnay/rust-toolchain@stable` for Rust setup
- Use `Swatinem/rust-cache@v2` for Cargo caching
- Use `actions/setup-node@v4` with `node-version: 22`
- Enable `corepack` for pnpm support
- Use `--frozen-lockfile` for deterministic installs
- Set `permissions: { contents: read }` (principle of least privilege)

## When to Edit

- **Add a new validation:** Add a job or step in `ci.yml`
- **Change build matrix:** Edit `strategy.matrix` for platform coverage
- **Add deployment:** Create a separate `deploy.yml` workflow (don't mix with CI)
- **Add scheduled checks:** Create a new workflow with `on: schedule`

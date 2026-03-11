# Telescope — Test Plan (Policy)

> ⚠️ **Status: Aspirational** — This describes the *target* test policy. Current coverage is minimal — see TESTING.md status note.

This repository is **test-driven by policy**. PRs that change behavior must ship with the tests and the pipeline updates in the same PR.

## 1) Non-negotiables
- **No merge without tests**.
- **No live Kubernetes/Azure dependencies in PR CI**. Use deterministic fixtures/stubs.
- **E2E + CI pipelines are developed together** (no “we’ll fix CI later”).
- **Flaky tests are quarantined** (tagged) and must be fixed before re-enabling as merge-blocking.

## 2) Required test types by component

### Rust (crates/*)
- Unit tests for:
  - parsing/validation helpers
  - cache/eviction primitives
  - API contract helpers
- Minimum: at least 1 meaningful unit test per new module.

### Web UI (apps/web)
- Unit/component tests (Vitest):
  - state transitions
  - table virtualization helpers
  - rendering of key screens
- E2E tests (Playwright) against a **stub engine API**:
  - open app → cluster list
  - switch cluster/namespace
  - browse resources → open detail
  - logs stream simulation

### Desktop (apps/desktop)
- At minimum:
  - Rust-side unit test(s) for app wiring
  - build + bundle smoke in CI (Windows/macOS)

## 3) CI gates
- Rust: `fmt`, `clippy -D warnings`, `test` (core crates)
- Web: lint + unit tests
- Web E2E: Playwright (deterministic)
- Desktop: build/bundle on Windows + macOS

## 4) PR checklist (must be included in PR description)
- [ ] What changed?
- [ ] What tests were added/updated?
- [ ] What CI jobs validate this change?
- [ ] Any new risks/flakes? (if yes, mitigation)

## 5) Local rapid testing (container only)
Local testing must be run in the provided dev container:
- `./scripts/dev-test.sh` (builds container + runs rust/web/e2e in a deterministic mode)

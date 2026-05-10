# security: architectural hardening and vulnerability remediation (Sprint 2026-05-10)

Target branch: `main`
Source branch: `review/architecture-vulnerability-2026-05-10`

## Executive Summary

This PR closes the 2026-05-10 architecture vulnerability review by replacing fragile task-tail cleanup with ownership-based safety, tightening sensitive data boundaries, and adding release-gate coverage for dependency and integration regressions. The theme across the sprint is Security by Design: RAII guards own cleanup, supervisors own task lifecycles, sensitive operations produce mandatory audit trails, and user-facing errors expose safe categories instead of raw internals.

The largest behavior correction is Helm values handling. Telescope no longer exposes a plaintext Helm values reveal path. Normal values requests return recursively redacted YAML, while `reveal: true` requests are denied before values are fetched, logged as denied audit attempts, and surfaced as sanitized errors.

## Detailed Breakdown

### Memory/Task Safety

- **F-02 - Watcher Lifecycle Auxiliary Task Leak:** Replaced the stored raw watcher `JoinHandle` with `WatchTaskHandle`, an owner that contains the shared `CancellationToken` and every watcher/forwarder task handle. Reconnect, disconnect, namespace switch, replacement, and drop paths now cancel the token and abort owned tasks together.
- **F-05 - Port-forward Active Counter Panic/Abort Safety:** Added `ActiveForwardGuard` with atomic acquisition and `Drop` cleanup so port-forward slots are released on normal completion, early return, panic unwinding, and task abort/drop.

### API & Data Security

- **F-03 - Helm Values Reveal Plaintext Secrets:** Changed raw reveal handling from a UI-confirmed plaintext path into a backend-enforced denial. `get_helm_release_values` validates input, denies `reveal: true` before client creation or value fetch, writes a denied `helm_values_reveal` audit entry, and returns a sanitized denial message. The frontend no longer sends reveal flags or renders reveal controls.
- **F-04 - Raw Command Errors and Paths:** Sanitized Helm rollback/uninstall public error paths and frontend IPC error handling so UI and console output no longer include helper paths, kubeconfig paths, raw stdout/stderr, tokens, or secret-shaped details.
- **F-08 - Resource Cache Raw GVK Allowlist:** Added `validate_cached_resource_gvk` backed by `ALL_WATCHED_GVKS` before cached list/get/count queries. Unsupported cached resource types now return the generic sanitized error `Unsupported cached resource type`; dynamic resource paths remain live API operations.

### Supply Chain Integrity

- **F-01 - Trusted Binary Resolver Test Fixture:** Replaced `current_exe()` trusted-binary test fixtures with dedicated temporary helper files using executable permissions and cleanup-on-drop. Production trusted-binary validation and permission checks were not weakened, including the kubeconfig exec-helper fixture uncovered by the final Rust gate.
- **F-06 - Security and Supply-chain Scanning:** Added a CI security job that installs and runs `cargo audit`, installs pnpm dependencies with `pnpm install --frozen-lockfile`, and runs `pnpm audit --audit-level high`. Added weekly grouped Dependabot updates for Cargo and npm/pnpm workspaces.
- **F-07 - K3D Integration Tests Not PR-gated:** Added a pull request trigger to the K3D integration workflow with path filters for engine/core/K3D fixture/workflow changes while preserving existing `main` push and manual dispatch behavior.

## Validation

### Final Global Gate

Status: Passed

- Pass: `./scripts/dev-test.sh` (container-first gate: Rust fmt/clippy/test, pnpm install, web unit tests, web E2E)
- Pass: `cargo fmt --all -- --check`
- Pass: `cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings`
- Pass: `cargo test --workspace --exclude telescope-desktop --all-features`
- Pass: `./scripts/pnpm.sh -C apps/web test` (8 files, 67 tests)
- Pass: `./scripts/pnpm.sh -C apps/web build`
- Pass: `./scripts/pnpm.sh -C apps/web e2e` (42 tests)

### Containerized Desktop Checks

- Pass in build container: `./scripts/dev-test.sh run bash -lc 'sudo chown -R pwuser:pwuser /home/pwuser/.cargo /home/pwuser/.rustup /home/pwuser/.local/share/pnpm/store && ./scripts/pnpm.sh -C apps/desktop prepare:frontend && cargo check -p telescope-desktop'`
- Pass in build container: `./scripts/dev-test.sh run cargo build -p telescope-desktop`

### Focused Verification

- F-01: trusted binary resolver tests passed for `telescope-core`, `telescope-azure`, and kubeconfig exec-helper coverage in `telescope-engine`.
- F-02: `cargo fmt --all -- --check` passed; host-only desktop check advances past `glib-sys` and remains blocked by missing `gdk-3.0.pc` before project code is checked.
- F-03: `./scripts/pnpm.sh -C apps/web test` passed (8 files, 66 tests) and `./scripts/pnpm.sh -C apps/web e2e -- detail-reload.spec.ts` passed (4 tests).
- F-04: `cargo test -p telescope-engine helm::tests -- --nocapture` passed (29 tests) and `./scripts/pnpm.sh -C apps/web test` passed (8 files, 67 tests).
- F-05: `cargo test -p telescope-engine portforward -- --nocapture` passed (7 unit tests; integration filter ran 0 tests in this environment).
- F-06: `git diff --check` passed and no-network YAML indentation sanity checks passed for `.github/workflows/ci.yml` and `.github/dependabot.yml`. Local `cargo audit` and `pnpm audit --audit-level high` were not run because they require network/tool download; the CI gates now run them.
- F-07: `git diff --check` passed and static workflow inspection confirmed the new pull request trigger and path filters.
- F-08: `cargo fmt --all -- --check`, `git diff --check`, and `./scripts/pnpm.sh -C apps/web e2e -- p2-routes.spec.ts` passed (12 tests).

## Notes for Reviewers

- The nonce/plaintext Helm values challenge discussed during review is not part of the current implementation. The shipped behavior in this branch is stricter: plaintext Helm values reveal is denied and audited.
- Desktop remains the only supported runtime. The SvelteKit frontend continues to communicate with Rust through Tauri IPC, with no HTTP fallback.
# Remediation Status

## F-01 — Trusted Binary Resolver Test Fixture

Status: Fixed

Files changed:
- `crates/core/src/trusted_binary.rs`
- `crates/azure/src/resolve.rs`
- `crates/engine/src/kubeconfig.rs`
- `REMEDIATION_STATUS.md`

Reproduction:
- `cargo test -p telescope-azure resolve::tests::resolve_az_binary_path_accepts_trusted_binary_name -- --nocapture` previously failed because the test used Cargo's current test executable as the trusted binary fixture. In this environment that executable is group-writable, so the production trusted-binary permission check correctly rejected it.
- `cargo test -p telescope-core trusted_binary::tests::resolve_trusted_binary_accepts_trusted_binary_name -- --nocapture` failed for the same fixture reason.
- The final workspace test gate also exposed the same fixture problem in `crates/engine/src/kubeconfig.rs` exec-helper tests, which used `std::env::current_exe()` as a trusted kubeconfig helper.

Remediation:
- Replaced the affected `current_exe()` fixture seam with dedicated std-only temporary helper files in the Rust test modules.
- Each fixture uses a unique temp-file name from `std::env::temp_dir()`, is made executable with Unix mode `0o755`, and is removed on drop.
- Production trusted-binary validation and permission checks were not weakened.

Verification:
- Pass: `cargo test -p telescope-core trusted_binary::tests::resolve_trusted_binary_accepts_trusted_binary_name -- --nocapture`
- Pass: `cargo test -p telescope-azure resolve::tests::resolve_az_binary_path_accepts_trusted_binary_name -- --nocapture`
- Pass: `cargo test -p telescope-core trusted_binary::tests -- --nocapture`
- Pass: `cargo test -p telescope-azure --lib`
- Pass: `cargo fmt --all -- --check`
- Pass: `cargo test -p telescope-engine kubeconfig::tests -- --nocapture`

## F-02 — Watcher Lifecycle Auxiliary Task Leak

Status: Fixed

Files changed:
- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Code inspection confirmed the pre-fix lifecycle shape in `spawn_watch_task`: `AppState` stored only the main pod watcher `JoinHandle<()>`, while the state forwarder and auxiliary watcher handles were captured inside that main task.
- `abort_watch` externally aborted only the stored main handle on reconnect, disconnect, and namespace switch. Because aborting a Tokio task drops its future immediately, the cleanup tail inside the main task could be skipped and auxiliary watcher tasks could continue running.

Remediation:
- Replaced the stored watcher handle with a `WatchTaskHandle` owner that holds a `CancellationToken` and all spawned watcher/forwarder `JoinHandle<()>` values.
- `WatchTaskHandle::cancel` and `Drop` both cancel the token and abort owned tasks, so reconnect, disconnect, namespace switch, and replacement drops cannot orphan auxiliary watchers.
- Updated state forwarding, cluster-scoped watchers, namespaced watchers, and the pod watcher to observe the shared cancellation token with `tokio::select!`.
- Added `tokio-util = { version = "0.7", features = ["rt"] }` to the desktop crate for `CancellationToken`.

Verification:
- Pass: `cargo fmt --all -- --check`
- Follow-up: `pkg-config --modversion glib-2.0` now resolves `2.80.0` on this host.
- Host-only status: `cargo check -p telescope-desktop` now advances past `glib-sys` and fails before checking project code because `gdk-sys` cannot find `gdk-3.0.pc` via `pkg-config`.
- Pass in build container: `./scripts/dev-test.sh run bash -lc 'sudo chown -R pwuser:pwuser /home/pwuser/.cargo /home/pwuser/.rustup /home/pwuser/.local/share/pnpm/store && ./scripts/pnpm.sh -C apps/desktop prepare:frontend && cargo check -p telescope-desktop'`
- Pass in build container: `./scripts/dev-test.sh run cargo build -p telescope-desktop`

## F-03 — Helm Values Reveal Plaintext Secrets

Status: Fixed

Files changed:
- `apps/desktop/src-tauri/src/main.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/routes/helm/[namespace]/[name]/+page.svelte`
- `apps/web/src/lib/api.test.ts`
- `apps/web/tests-e2e/detail-reload.spec.ts`
- `apps/web/tests-e2e/helpers/mock-tauri.ts`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Code inspection confirmed the pre-fix backend fetched Helm release values and returned them unredacted whenever `reveal: true` was supplied to `get_helm_release_values`.
- The Helm detail page exposed a reveal button and confirmation dialog that called `getHelmReleaseValues(namespace, releaseName, true)`.
- The backend had audit coverage for Helm rollback and uninstall, but no audit entry for Helm values reveal attempts.

Remediation:
- `get_helm_release_values` now validates namespace and release name before use, rejects `reveal: true` before creating a Kubernetes client or fetching Helm values, writes a denied audit entry for the reveal attempt, and returns only sanitized denial messages.
- Normal `reveal: false` or omitted reveal behavior still fetches Helm values and redacts sensitive keys before returning YAML.
- The frontend API wrapper no longer accepts or sends a reveal flag, and the Helm detail page no longer renders the reveal button, sensitive-values warning, hide button, or reveal confirmation dialog.
- The Playwright mock now fails accidental reveal calls, and API/E2E coverage asserts the UI requests Helm values without a reveal flag.

Verification:
- Pass: `cargo fmt --all -- --check`
- Pass: `./scripts/pnpm.sh -C apps/web test` (8 files, 66 tests)
- Pass: `./scripts/pnpm.sh -C apps/web e2e -- detail-reload.spec.ts` (4 tests)

## F-04 — Raw Command Errors and Paths

Status: Fixed

Files changed:
- `crates/engine/src/helm.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/api.test.ts`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Code inspection confirmed pre-fix `rollback_release` and `helm_uninstall` returned errors containing `helm_binary.display()` plus raw Helm `stderr` or `stdout` details.
- `map_helm_uninstall_error` appended raw command output to category messages, so local paths or command details could reach the desktop JSON response.
- The frontend IPC wrapper logged raw `Error` objects in `invoke`, and `notifyApiError` logged raw message strings to the console.

Remediation:
- Helm rollback and uninstall now resolve and execute the CLI through sanitized public error paths that return category messages only: release not found, permission denied, operation timed out, command unavailable, or command failed.
- The trusted Helm binary resolver and its tests remain intact; public CLI call sites wrap resolver/execute failures without exposing executable paths or resolver internals.
- Frontend API error handling now converts thrown/listener messages through path and secret redaction before propagation, and console logging records only command-level failure markers without raw `Error` objects or raw messages.
- Added Rust tests proving Helm rollback/uninstall errors omit the mock binary path and raw `stderr` path/secret details, plus frontend tests proving thrown errors, listeners, and console logs do not contain raw paths or secrets.

Verification:
- Pass: `cargo fmt --all -- --check`
- Pass: `cargo test -p telescope-engine helm::tests -- --nocapture` (29 tests)
- Pass: `./scripts/pnpm.sh -C apps/web test` (8 files, 67 tests)

## F-05 — Port-forward Active Counter Panic/Abort Safety

Status: Fixed

Files changed:
- `crates/engine/src/portforward.rs`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Code inspection confirmed pre-fix `start_port_forward` incremented `ACTIVE_FORWARDS` with `fetch_add`, manually decremented it on selected pod verification, bind, and address error paths, then decremented only at the normal end of the spawned forwarding task.
- Because Tokio task abort drops the task future immediately, and panic unwinding may skip explicit tail cleanup when not modeled as ownership, the manual task-tail decrement could leave the active counter elevated and eventually deny new forwards.

Remediation:
- Added a private `ActiveForwardGuard` that acquires a slot with atomic `fetch_update`, refusing new forwards once `MAX_FORWARDS` is reached without incrementing and undoing.
- Implemented `Drop` for the guard so every early return after acquisition releases the active slot automatically.
- Moved the guard into the spawned forwarding task after pod verification and listener bind succeed, so normal completion, panic unwinding, and task abort/drop release the active counter through RAII.
- Added focused unit tests for guard acquire/drop accounting and at-limit rejection without requiring a Kubernetes cluster.

Verification:
- Pass: `cargo fmt --all -- --check`
- Pass: `cargo test -p telescope-engine portforward -- --nocapture` (7 unit tests passed; integration filter ran 0 tests in this environment)

## F-06 — Security and Supply-chain Scanning Absent from Workflows

Status: Fixed

Files changed:
- `.github/workflows/ci.yml`
- `.github/dependabot.yml`
- `.github/workflows/AGENTS.md`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Confirmed pre-fix `.github/dependabot.yml` was missing.
- `grep -RInE "cargo audit|cargo-audit|cargo deny|cargo-deny|pnpm audit|npm audit|codeql|trivy|osv|ossf|scorecard|security scan|supply-chain|dependency review|audit-ci" .github scripts package.json Cargo.toml pnpm-lock.yaml` produced no matches, confirming no existing Cargo audit, cargo-deny, pnpm audit, CodeQL, or equivalent security scan in CI/scripts.
- `.github/workflows/AGENTS.md` explicitly marked security scanning as failed because Dependabot, CodeQL, and audit checks were absent.

Remediation:
- Added a dedicated `security` job to `.github/workflows/ci.yml` that runs on the existing pull request and `main` push triggers alongside current jobs.
- The job installs `cargo-audit`, runs `cargo audit` against the committed Cargo lockfile, installs pnpm dependencies with `pnpm install --frozen-lockfile`, and runs `pnpm audit --audit-level high`.
- Added `.github/dependabot.yml` with weekly grouped Cargo and npm/pnpm workspace dependency update checks.
- Updated `.github/workflows/AGENTS.md` to document the new security job and mark security scanning as enforced.

Verification:
- Pass: `git diff --check`
- Pass: no-network YAML indentation sanity check of `.github/workflows/ci.yml` and `.github/dependabot.yml` with Node
- Not run locally: `cargo audit` and `pnpm audit --audit-level high`; both require installing/downloading tools or querying external advisory/package registries. These gates now execute in CI.

## F-07 — K3D Integration Tests Not PR-gated

Status: Fixed

Files changed:
- `.github/workflows/integration.yml`
- `.github/workflows/AGENTS.md`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Confirmed pre-fix `.github/workflows/integration.yml` had `workflow_dispatch` and a `push` trigger limited to `main` with path filters for `crates/engine/**`, `crates/core/**`, `tools/k3d-fixtures/**`, and `.github/workflows/integration.yml`.
- Confirmed the pre-fix workflow had no `pull_request` trigger, so K3D integration tests did not run before merging PRs that changed engine/core/K3D fixture/workflow paths.

Remediation:
- Added a `pull_request` trigger to `.github/workflows/integration.yml` with the same path filters as the existing `push` trigger.
- Preserved the existing `workflow_dispatch` and `push` trigger behavior.
- Kept the existing K3D install, cluster setup, fixture deployment, integration test, and teardown logic intact.
- Updated `.github/workflows/AGENTS.md` to document that K3D integration tests are PR-gated for engine/core/K3D fixture/workflow changes.

Verification:
- Pass: `git diff --check`
- Pass: static confirmation that `.github/workflows/integration.yml` contains a `pull_request` trigger with path filters for `crates/engine/**`, `crates/core/**`, `tools/k3d-fixtures/**`, and `.github/workflows/integration.yml`.
- Not run locally: K3D integration tests. The K3D gate now runs in GitHub Actions for matching pull requests, `main` pushes, and manual dispatch.

## F-08 — Resource Cache Raw GVK Allowlist

Status: Fixed

Files changed:
- `apps/desktop/src-tauri/src/main.rs`
- `REMEDIATION_STATUS.md`

Reproduction / Inspection:
- Confirmed pre-fix `get_resources` passed caller-supplied `gvk` strings directly to `ResourceStore::list`, and `get_resource` passed caller-supplied `gvk` strings directly to `ResourceStore::get`.
- Confirmed `search_resources` already iterated `ALL_WATCHED_GVKS`, so it did not issue arbitrary cache lookups.
- Confirmed known cached frontend resource routes use watched GVK constants, while secrets, namespaces, and CRD/dynamic resource screens use separate direct or dynamic API commands instead of raw cache reads.

Remediation:
- Added `validate_cached_resource_gvk`, backed by `ALL_WATCHED_GVKS`, for exact allowlist validation before cached `ResourceStore::list`, `ResourceStore::get`, and the related cached `ResourceStore::count` command.
- Unsupported cached resource types now return the generic sanitized error `Unsupported cached resource type` without echoing arbitrary caller-provided GVK values.
- Left `list_dynamic_resources` and `get_dynamic_resource` unchanged so CRD/dynamic lookups continue through the live Kubernetes dynamic API path.

Verification:
- Pass: `cargo fmt --all -- --check`
- Pass: `git diff --check`
- Follow-up: `pkg-config --modversion glib-2.0` now resolves `2.80.0` on this host.
- Host-only status: `cargo check -p telescope-desktop` now advances past `glib-sys` and fails before checking project code because `gdk-sys` cannot find `gdk-3.0.pc` via `pkg-config`.
- Pass in build container: `./scripts/dev-test.sh run bash -lc 'sudo chown -R pwuser:pwuser /home/pwuser/.cargo /home/pwuser/.rustup /home/pwuser/.local/share/pnpm/store && ./scripts/pnpm.sh -C apps/desktop prepare:frontend && cargo check -p telescope-desktop'`
- Pass in build container: `./scripts/dev-test.sh run cargo build -p telescope-desktop`
- Pass: `./scripts/pnpm.sh -C apps/web e2e -- p2-routes.spec.ts` (12 tests)

## Final Global Validation

Status: Passed

Commands:
- Pass: `./scripts/dev-test.sh` (container-first gate: Rust fmt/clippy/test, pnpm install, web unit tests, web E2E)
- Pass: `cargo fmt --all -- --check`
- Pass: `cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings`
- Pass: `cargo test --workspace --exclude telescope-desktop --all-features`
- Pass: `./scripts/pnpm.sh -C apps/web test` (8 files, 67 tests)
- Pass: `./scripts/pnpm.sh -C apps/web build`
- Pass: `./scripts/pnpm.sh -C apps/web e2e` (42 tests)

Notes:
- The first full `cargo test --workspace --exclude telescope-desktop --all-features` run exposed the same hardened trusted-binary fixture issue in `crates/engine/src/kubeconfig.rs`; that test fixture was fixed and the full Rust gate was rerun successfully.
- `pkg-config --modversion glib-2.0` now resolves `2.80.0`; host-only `cargo check -p telescope-desktop` now advances to `gdk-sys` and remains blocked before checking project code because `gdk-3.0.pc` is not available to `pkg-config`.
- The desktop Rust app does validate in the repo build container: after `./scripts/pnpm.sh -C apps/desktop prepare:frontend`, both `cargo check -p telescope-desktop` and `cargo build -p telescope-desktop` passed under `./scripts/dev-test.sh run`.

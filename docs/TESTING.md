# Telescope — Testing Strategy

> **Status: Current suite + future direction** — The repository is no longer at the “3 Rust / 2 Vitest / 2 E2E” stage. The current test inventory is **107 Rust tests**, **5 Vitest files**, and **4 Playwright specs**.

## Goals
- Keep CI **green by default**: no flaky E2E, no hidden cluster dependency in routine PR validation.
- Define tests **per component** with explicit acceptance criteria.
- Keep deterministic fixtures and stubbed browser flows so UI validation remains stable.

## Test pyramid (what runs where)

### 1) Rust engine, core, API, and hub (`crates/*`, `apps/hub`)
**Current Rust coverage: 107 tests**
- `crates/core` exercises the connection-state machine and SQLite-backed resource store.
- `crates/engine` covers actions, audit logging, CRD discovery, exec helpers, Helm parsing/history/value redaction, log helpers, metrics parsing, port-forward helpers, secret redaction, and watcher lifecycle logic.
- `apps/hub` covers OIDC/auth scaffolding, JWT claim decoding, and basic access checks.
- `crates/engine/tests/integration_k3d.rs` provides the real-cluster integration harness for the engine surface.

**Run locally:** `cargo test --workspace --exclude telescope-desktop --all-features`

### 2) UI (`apps/web`)
**Current Vitest coverage: 5 test files**
- `src/lib/azure-utils.test.ts` — AKS URL detection and Azure Portal link generation helpers.
- `src/lib/error-suggestions.test.ts` — friendly error-message suggestions for auth, RBAC, timeout, and connectivity failures.
- `src/lib/hello.test.ts` — minimal sanity/unit smoke test for the sample helper.
- `src/lib/prod-detection.test.ts` — production-context pattern detection.
- `src/lib/version.test.ts` — shared version exposure.

**Run locally:** `pnpm -C apps/web test`

### 3) Browser E2E (`apps/web/tests-e2e`)
**Current Playwright coverage: 4 specs**
- `smoke.spec.ts` — app boots and renders the landing page.
- `clusters.spec.ts` — cluster selection flow navigates into the connected resource views.
- `resources.spec.ts` — expanded resource navigation and detail coverage for StatefulSets, DaemonSets, Jobs, CronJobs, Secrets, Ingresses, PVCs, and the command-palette search flow.
- `settings.spec.ts` — settings/about page exposes the shared application version.

These E2E tests run against deterministic stubbed data, not a live Kubernetes cluster.

**Run locally:** `pnpm -C apps/web e2e`

### 4) Desktop (`apps/desktop`)
**Current desktop validation**
- CI builds the Tauri app on macOS and Windows.
- The desktop shell reuses the `apps/web` build, so most UI behavior is exercised by the web test suite.

**Current gap:** there is still no dedicated Tauri WebDriver-style desktop E2E suite.

## CI policy (current)
- No live AKS/Kubernetes dependency in standard PR CI for the web app.
- Browser E2E uses deterministic fixtures/stubbed responses.
- Rust, web, and desktop validations are split across workflow jobs rather than hidden behind one opaque script.

## “Definition of Done” for a component
- Relevant unit/integration tests written or updated and passing.
- If UI surface changed: update Playwright coverage or deterministic fixtures when it materially affects the user flow.
- CI commands and docs updated in the same PR when the testing surface changes.

## Still planned
- Broader browser-mode parity tests against `apps/hub` write flows.
- More end-to-end coverage for logs, exec, port-forward, Helm, and resource mutation workflows.
- Dedicated desktop E2E coverage once the Tauri surface stabilizes further.

# Telescope — Testing Strategy (v0)

## Goals
- Keep CI **green by default**: no flaky E2E, no “works on my machine”.
- Define tests **per component** with explicit acceptance criteria.
- Ship CI + E2E together so we don’t thrash on broken pipelines.

## Test pyramid (what runs where)

### 1) Rust engine (crates/*)
**Unit tests (fast):** `cargo test`
- Pure logic: GVK typing, query/filtering, cache eviction, diff/patch helpers.

**Integration tests (still fast):** `cargo test -p api -p engine`
- gRPC surface validation (request/response contracts).
- Fake kube-apiserver stubs (no real cluster dependency in CI).

**Contract tests (required):**
- Maintain protobuf definitions and golden JSON fixtures to ensure UI compatibility.

### 2) UI (apps/web, packages/ui)
**Unit/component tests:** (Vitest)
- Rendering, state transitions, table virtualization hooks.

**E2E tests (Playwright):**
- Runs against a mocked engine API (local stub server) to avoid cluster flakiness.
- Core flows:
  - open app → see cluster list
  - switch cluster/namespace
  - browse resources (virtualized list)
  - open workload detail tabs
  - logs stream view (simulated)

### 3) Desktop (apps/desktop)
**Smoke tests (CI):**
- Build the Tauri app on Windows + macOS.

**E2E (later phase, gated):**
- Tauri WebDriver-based tests are valuable but can be flaky. We will add them only after the web E2E suite is stable.

## CI policy (non-negotiable)
- No live AKS/K8s dependency in PR CI.
- E2E uses deterministic fixtures.
- Any flaky test gets quarantined (tagged) and does **not** block merges until fixed.

## “Definition of Done” for a component
- Unit tests written and passing.
- If UI surface changed: update contract fixtures + E2E coverage.
- CI jobs updated in the same PR.


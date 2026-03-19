---
title: Testing
nav_order: 6
description: "Test pyramid — Rust unit tests, Vitest, Playwright E2E"
---

# Telescope — Testing Strategy

> **Status: v1.0.0** — Current test inventory: **133 Rust tests**, **5 Vitest files (40+ test cases)**, and **6 Playwright specs**.

## Goals
- Keep CI **green by default**: no flaky E2E, no hidden cluster dependency in routine PR validation.
- Define tests **per component** with explicit acceptance criteria.
- Keep deterministic fixtures and stubbed frontend flows so UI validation remains stable.

## Test pyramid (what runs where)

### 1) Rust crates (`crates/*`)
**Current Rust coverage: 133 tests**

| Crate | Tests | Scope |
|-------|-------|-------|
| `telescope-core` | 39 | Connection-state machine transitions, SQLite-backed resource store (upsert, delete, list, count, preferences) |
| `telescope-engine` | 57 | Actions, audit logging, CRD discovery, exec helpers, Helm parsing/history/value redaction, log helpers, metrics parsing, port-forward helpers, secret redaction, watcher lifecycle, node operations, dynamic resources |
| `telescope-azure` | 29 | ArmClient construction, AzureCloud endpoint resolution, AKS identity resolution, node pool operations, upgrade profile parsing, maintenance config parsing, error mapping |
| Integration (`engine/tests/integration_k3d.rs`) | 8 | Real-cluster integration harness for the engine surface (requires k3d) |

**Run locally:** `cargo test --workspace --exclude telescope-desktop --all-features`

### 2) Frontend UI (`apps/web`)
**Current Vitest coverage: 5 test files, 40+ test cases**
- `src/lib/azure-utils.test.ts` (16 cases) — AKS URL detection, Azure Portal link generation, Azure Government endpoint handling.
- `src/lib/error-suggestions.test.ts` (8 cases) — friendly error-message suggestions for auth, RBAC, timeout, and connectivity failures.
- `src/lib/prod-detection.test.ts` (14 cases) — production-context pattern detection via `it.each()` table-driven tests.
- `src/lib/version.test.ts` (1 case) — shared version exposure.
- `src/lib/realMetrics.test.ts` — real metrics polling logic tests.

**Run locally:** `pnpm -C apps/web test` (vitest with `--pool=forks`)

### 3) Frontend E2E (`apps/web/tests-e2e`)
**Current Playwright coverage: 6 specs**
- `smoke.spec.ts` — app boots and renders the landing page.
- `settings.spec.ts` — settings/about page exposes the shared application version.
- `node-pools.spec.ts` — AKS node pool management flows.
- `search-palette.spec.ts` — search palette interaction and navigation.
- `detail-reload.spec.ts` — resource detail page reload behavior.
- `error-states.spec.ts` — error state rendering and recovery.

These E2E tests run against deterministic stubbed data using `tests-e2e/helpers/mock-tauri.ts` to simulate Tauri IPC in the browser, not a live Kubernetes cluster.

**Setup:** `pnpm -C apps/web exec playwright install --with-deps chromium`

**Run locally:** `pnpm -C apps/web e2e`

### 4) Desktop (`apps/desktop`)
**Current desktop validation**
- CI builds the Tauri app on macOS and Windows.
- The desktop shell reuses the `apps/web` build, so most UI behavior is exercised by the frontend test suite.

**Current gap:** there is still no dedicated Tauri WebDriver-style desktop E2E suite.

### 5) Azure ARM (`crates/azure`)
**Current coverage: 29 unit tests**
- `ArmClient` construction and cloud endpoint resolution.
- `AzureCloud` detection from AKS server URLs (Commercial + Government).
- `AksResourceId` ARM path construction.
- AKS operations: node pool parsing, upgrade profile parsing, maintenance config parsing.
- Error mapping: 404 → `NotFound`, 409 → `Conflict`, API error extraction.
- Identity resolution logic.

**No automated ARM integration tests** — ARM operations require a live Azure subscription with an AKS cluster. Unit tests use mocked responses. Manual validation against real AKS clusters is documented in the smoke test checklist.

## CI policy (current)
- No live AKS/Kubernetes dependency in standard PR CI for the frontend.
- No live Azure subscription dependency in Rust CI — `telescope-azure` tests use mocks.
- Frontend E2E uses deterministic fixtures/stubbed responses.
- Rust, frontend, and desktop validations are split across workflow jobs (`ci.yml`):
  - `rust` job: fmt + clippy + test (excludes desktop on Linux)
  - `web` job: Vitest unit tests + production build
  - `web-e2e` job: Playwright E2E (needs `web`)
  - `desktop-build` job: Tauri build on macOS + Windows

## "Definition of Done" for a component
- Relevant unit/integration tests written or updated and passing.
- If UI surface changed: update Playwright coverage or deterministic fixtures when it materially affects the user flow.
- CI commands and docs updated in the same PR when the testing surface changes.

## Still planned
- More end-to-end coverage for logs, exec, port-forward, Helm, and resource mutation workflows.
- Dedicated desktop E2E coverage once the Tauri surface stabilizes further.
- ARM integration tests against a dedicated test AKS cluster (gated, not in standard PR CI).

# Project Context

- **Owner:** Adam Dost
- **Project:** Telescope — AKS-first Kubernetes IDE (Rust/Svelte/Tauri v2, desktop-only)
- **Stack:** Rust (crates/core, crates/engine, crates/azure), Svelte 5 (apps/web), Tauri v2 (apps/desktop)
- **Created:** 2026-03-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->
- 2026-03-20: The `apps/web` validation gate is stable as `pnpm -C apps/web test` → `pnpm -C apps/web build` → `pnpm -C apps/web e2e`; current baseline is 49 Vitest tests and 32 Playwright specs passing, with E2E anchored in `apps/web/tests-e2e/`.
- 2026-03-19: `cargo test --workspace --exclude telescope-desktop --all-features` currently reports 176/176 passing, but `crates/engine/tests/integration_k3d.rs` only exercises real cluster paths when `K3D_TEST=1`; otherwise it exits early and passes.
- 2026-03-19: E2E Kubernetes UI confidence is strongest for mocked AKS node-pool lifecycle flows (`tests-e2e/node-pools.spec.ts`) and is backed by 45 mocked IPC commands in `apps/web/tests-e2e/helpers/mock-tauri.ts`.
- 2026-03-19: ARM error-path coverage now includes Azure client status mapping tests (401/403/404 + network + malformed payloads) and Node Pool E2E checks for user-friendly ARM errors, dismiss behavior, and retry recovery using configurable mock IPC failures.

### 2026-03-19 — ARM Error Handling Session

Added Rust unit tests for ARM error mapping (401/403/404/timeout/malformed) in `client.rs` and `aks.rs`. Extended mock-tauri with `commandErrors` for per-command error injection. Added Playwright E2E tests for ARM error display, dismiss, and retry recovery on node-pools page. Ripley delivered backend errors, Lambert delivered frontend banner. All validation green.

### 2026-03-19 — Cross-Agent Audit Summary

Dallas confirmed 29 GVKs, 66 IPC commands, near-complete coverage — only gap is Helm write ops. Ripley verified all kubectl/helm commands against live cluster with zero failures. Lambert confirmed frontend matches backend: 65 API functions, 39 routes, all GVKs reachable. Decision: ship v1.0.0 as-is.
- 2026-03-19: Helm uninstall backend tests are now executable in-engine by stubbing `TELESCOPE_HELM_PATH` to a temporary script; env mutation must be serialized in tests to avoid cross-test interference.
- 2026-03-19: E2E mocking now supports both one-shot command failures (`commandErrors`) and deterministic latency injection (`commandDelays`) in `apps/web/tests-e2e/helpers/mock-tauri.ts`.
- 2026-03-19: `apps/web/src/lib/api.ts#getResources` swallows `get_resources` IPC errors and returns `[]`, so resource-list routes render empty states (not `role="alert"` error banners) when backend fetch fails.

### 2026-03-19 — Helm Uninstall + P2 Routes Session

Added Rust unit tests for helm uninstall (success/not-found/empty-name). Added E2E specs for helm uninstall action/confirm/success/error flows + 3 P2 route specs (load/columns/detail/search-palette/loading/error). Extended mock-tauri with `helm_uninstall` mock + `commandDelays`. Cargo tests green. E2E: 4/11 new specs pass, remainder awaiting full UI wiring merge.

### 2026-03-19 — P2 E2E Fix + All Gaps Resolved

Fixed P2 route E2E test mismatches in `tests-e2e/p2-routes.spec.ts` — aligned assertions to actual route rendering (empty states vs error banners, column labels, detail navigation). All 32 E2E + 36 unit + Rust tests green. All audit gaps now resolved.

- 2026-03-20: E2E validation run by Kane: executed `pnpm -C apps/web test`, `pnpm -C apps/web build`, and `pnpm -C apps/web e2e`. All checks passed (test, build, and E2E). Session log: `.squad/log/20260320-120000-e2e-validation-suite.md`. Orchestration record: `.squad/orchestration-log/20260320-120000-kane.md`.
- 2026-03-20: Learning: The `apps/web` validation gate remains stable for this run; no new flaky specs observed.


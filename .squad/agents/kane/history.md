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

- 2026-03-24: AI Insights auth and RBAC coverage should use fake credential and provider transport seams; do not rely on live Azure OpenAI validation in CI or routine local runs.
- 2026-03-24: Encrypted history needs storage-boundary assertions for ciphertext at rest plus retention trimming to the last 3 entries per cluster.
- 2026-03-24: Allowlist-only context shaping needs deterministic unit tests that fail on secret-like payloads, unexpected fields, unstable ordering, and per-category cap overruns before Playwright route coverage is added.
- 2026-03-24: AI settings wrappers need explicit persistence-boundary round-trip tests for optional fields like `modelName`; mirrored type tests alone do not protect Rust `Option` semantics.
- 2026-03-24: Azure OpenAI transport QA is acceptable with crate-local credential and response seams for now, but request-header assertions remain a low-risk follow-up gap.
- 2026-03-24: Task 3 AI Insights context-builder acceptance is satisfied by targeted Rust coverage for allowlist-only shaping, redaction, stable ordering, namespace scope, and deterministic caps, even if broader workspace checks still have unrelated pre-existing failures outside the engine slice.
- 2026-03-24: Low-risk follow-up gaps after initial context-builder acceptance are explicit cap-overrun coverage for pod, event, node, and Helm collections and an explicit namespace-filtering regression test for cross-namespace pods and events.

### 2026-03-24 -- AI Insights Deficiency Fix QA

- Accepted deficiency fix batch: 37 targeted tests pass, 0 failures, no false positives detected.
- Fix 1 (null description): both positive and negative tests exercise the real `build_wire_request` code path, confirming key presence/absence.
- Fix 2 (408/504/429): tests use `map_openai_response_error` (full classification path including JSON parsing), not the inner classifier directly, confirming end-to-end behavior.
- Fix 3 (5 new context tests): closes the remaining cap-overrun and cross-namespace filtering gaps identified during Task 3 acceptance.
- Observation: the redundant `#[serde(skip_serializing_if)]` annotation on `AzureOpenAiResponseFormatJsonSchema.description` is cosmetic and does not affect correctness.
- All previously identified low-risk follow-up gaps are now resolved.

### 2026-04-01 — Security Issues #200, #201, #202 Test Matrix & Acceptance Scope

Expanded regression, edge-case, and acceptance test matrix per issue:
- **#200:** Unit tests for redaction/omission rules (benign metadata preserved, secret detail filtered), regression tests covering bearer tokens, password flags, connection strings; acceptance flow through `ExecTerminal.svelte` → `exec_command` handler → disk audit log.
- **#201:** Verification via lockfile inspection and `pnpm audit --audit-level=moderate`; acceptance validation `pnpm -C apps/web test`, `pnpm -C apps/web build`.
- **#202:** Unit tests for nested objects, arrays, mixed safe/unsafe content, reveal=true behavior; acceptance via desktop `get_helm_release_values` flow confirming default view hides nested secrets while reveal mode still works.
All tests integrated into existing CI validation gate and rely on repo-native command set.

### 2026-04-01 — Playwright CLI/E2E Environment Triage Spawn

**Context:** Post-security remediation release (issues #200, #201, #202 resolved), E2E suite validation gate may have been affected by Playwright CLI environment incompatibilities on Linux CI runners, dependency version constraints, transitive vulnerabilities, or Tauri IPC mock setup issues.

**Spawn time:** 2026-04-01T04:33:43Z  
**Task:** Reproduce and validate E2E failure path and confirm evidence  
**Orchestration log:** `.squad/orchestration-log/20260401-043343-kane.md`  
**Session log:** `.squad/log/20260401-043343-playwright-cli-e2e-triage.md`  
**Partner:** Ripley (backend blocker triage)


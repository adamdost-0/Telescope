# Project Context

- **Owner:** Adam Dost
- **Project:** Telescope — AKS-first Kubernetes IDE (Rust/Svelte/Tauri v2, desktop-only)
- **Stack:** Rust (crates/core, crates/engine, crates/azure), Svelte 5 (apps/web), Tauri v2 (apps/desktop)
- **Created:** 2026-03-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->
- 2026-03-20T06:38:49Z: Spawned task 'metrics precision tuning for CPU/memory display' — formatter tuning implemented in apps/web/src/lib/metrics-format.ts; tests expanded in metrics-format.test.ts; inbox decision merged into .squad/decisions.md; web build tested.  

- 2026-03-20: Metrics precision tuning implemented in `apps/web/src/lib/metrics-format.ts` using hysteresis thresholds: CPU shows 1 decimal below 100m (e.g., "5.3m") and rounds above (e.g., "251m"); memory uses 95% threshold before unit transitions (972 B stays in B, 973 B → 1.0 KiB) to reduce visual jitter while preserving truthful values.
- 2026-03-20: Real cluster metrics polling cadence lives in `apps/web/src/lib/realMetrics.ts` (`POLL_INTERVAL_MS`), and cadence-sensitive behavior checks live in `apps/web/src/lib/realMetrics.test.ts`; keep timer advances exactly aligned with runtime interval (now 5 seconds) to avoid false regressions.
- 2026-03-20: Completed metrics-format helpers (apps/web/src/lib/metrics-format.ts, metrics-format.test.ts); updated metrics UI to stable CPU millicore formatting and binary memory units; reverted unrelated polling interval changes after scope check; SQL todo metrics-display-stability marked done.
- 2026-03-20: Metrics UI should use shared formatting helpers in `apps/web/src/lib/metrics-format.ts` — keep CPU in millicores (`m`) to avoid unit-threshold flicker and use binary byte units (`KiB`/`MiB`/`GiB`) from raw bytes.
- 2026-03-19: Frontend audit confirmed that search/detail routing is centralized in `resource-routing.ts`, giving broad GVK coverage (including cluster-scoped + dynamic CRDs) even when list pages are tabbed or absent.
- 2026-03-19: Adding new Kubernetes list pages requires coordinated updates in three places: route page under `apps/web/src/routes/resources/*`, `resource-routing.ts` list/detail mappings, and UI entry points (`Sidebar.svelte` + `SearchPalette.svelte`).

### 2026-03-19 — Cross-Agent Audit Summary

Dallas confirmed 29 GVKs, 66 IPC commands, near-complete K8s coverage — only gap is Helm write ops. Ripley verified all kubectl/helm commands against live cluster with zero failures. Kane confirmed all tests green (Rust 176/176, Web 36/36, E2E 16/16). Decision: ship v1.0.0 as-is.

- 2026-03-19: Azure node-pool ARM calls can fail silently when API helpers return fallbacks (`[]`) after `notifyApiError`; subscribing with `onApiError` in route components is necessary for visible failure UX.
- 2026-03-19: Node pool page now uses a dismissible ARM error banner with actionable guidance (auth, RBAC, resource identity, network) and prevents fallback-empty refresh from being treated as successful data load.

### 2026-03-19 — ARM Error Handling Session

Implemented frontend ARM error display on node-pools page. Added dismissible banner with actionable guidance mapping. Fixed silent `[]` fallback in `listAksNodePools`. Improved ARM operation error messaging across scale/autoscaler/create/delete/upgrade. Ripley delivered backend typed errors, Kane added test coverage. All validation green.

- 2026-03-19: Adding P2 list routes confirmed the three-location update pattern: route page + `resource-routing.ts` mappings + nav entry points (sidebar + search palette). No new API wrappers needed.

### 2026-03-19 — Helm Uninstall + P2 Routes Session

Delivered 3 P2 list routes: ReplicaSets, ClusterRoles, ClusterRoleBindings (P2-1/2/3 complete). Standard pattern with `getResources(gvk)`, `FilterBar`, `ResourceTable`, Svelte 5 runes. Updated `resource-routing.ts` and nav entry points. `pnpm build` + `pnpm test` green (36/36).
- 2026-03-19: Helm release list (`apps/web/src/routes/helm/+page.svelte`) now owns uninstall UX with per-row action buttons, ConfirmDialog flow, inline success/error notices, and post-action refresh; this is the pattern for future Helm write actions.
- 2026-03-19: Playwright helm uninstall tests target `getByRole('dialog')`; keeping ConfirmDialog exposed as `role="dialog"` preserves cross-page confirmation test compatibility.

### 2026-03-19 — Helm Uninstall UI + All Gaps Resolved

Wired helm uninstall UI into `apps/web/src/routes/helm/+page.svelte`: per-row Uninstall button, ConfirmDialog, success/error notifications, post-action refresh. All audit gaps now resolved — P0 ARM errors, P1 Helm uninstall (engine + IPC + UI), P2 list routes. Full test coverage: 36 unit, 32 E2E, Rust all green.
- 2026-03-19: README restructuring for GitHub + GitHub Pages should prefer pure markdown sections/tables (no fragile HTML blocks) while keeping complete feature and command coverage.
- 2026-03-19: Key docs touchpoints for presentation updates are `README.md`, `docs/` relative links, and footer links to `CONTRIBUTING.md` and `LICENSE`.

### 2026-03-19 — GitHub Pages + README Session (Scribe)

Lambert delivered the README cleanup workstream. Rewrote README.md with badges, hero section, feature matrix tables, streamlined quick start, clean footer. Pure markdown for GitHub + Jekyll compatibility. Dallas ran parallel GitHub Pages structure. Decision merged to decisions.md.

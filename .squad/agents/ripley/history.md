# Project Context

- **Owner:** Adam Dost
- **Project:** Telescope — AKS-first Kubernetes IDE (Rust/Svelte/Tauri v2, desktop-only)
- **Stack:** Rust (crates/core, crates/engine, crates/azure), Svelte 5 (apps/web), Tauri v2 (apps/desktop)
- **Created:** 2026-03-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->
- 2026-03-19: Completed deep backend audit of Rust K8s/AKS surface. `crates/engine` exposes 30 watcher wrappers while desktop startup registers 29 active watches (using `watch_all_events` as the single events watcher mode). Live AKS context `dassadsawqew` validated all requested kubectl/helm capability checks successfully; zero-count resource classes were state-driven, not API failures.

### 2026-03-19 — Cross-Agent Audit Summary

Dallas confirmed 29 GVKs, 66 IPC commands, near-complete coverage — only gap is Helm write ops. Lambert confirmed frontend matches backend: 65 API functions, 39 routes, all GVKs reachable. Kane confirmed all tests green (Rust 176/176, Web 36/36, E2E 16/16). Decision: ship v1.0.0 as-is.
- 2026-03-19: Verified AKS node pool listing path is true ARM (`managedClusters/{cluster}/agentPools`) via Tauri IPC → azure crate → ArmClient HTTP calls; no K8s label heuristics. Added typed ARM failure variants (token expired, subscription/RG/cluster not found, RBAC denied, timeout) with actionable messages and fixed node-pool deletion polling to stop swallowing non-404 errors.

### 2026-03-19 — ARM Error Handling Session

Delivered typed ARM error variants in `crates/azure` (error.rs, client.rs, aks.rs). Fixed silent-delete bug where non-404 errors were swallowed during node pool deletion polling. Improved IPC error context. Fixed `listAksNodePools` in api.ts to rethrow. Lambert handled frontend banner, Kane added test coverage. All validation green.
- 2026-03-19: Implemented first Helm write op (`helm uninstall`) end-to-end. Added `telescope_engine::helm::helm_uninstall(namespace, name)` with trusted Helm binary resolution, input validation, CLI execution, and categorized error messaging for release-not-found, permission denied, and timeout cases. Wired new Tauri IPC command `helm_uninstall` in `apps/desktop/src-tauri/src/main.rs` with namespace/name validation and audit logging.

### 2026-03-19 — Helm Uninstall + P2 Routes Session

Delivered helm_uninstall engine API + Tauri IPC command (P1-3 complete). Engine tests 94→97 with uninstall error categorization coverage. Kane added E2E specs for uninstall flows. Cargo clippy + tests green.

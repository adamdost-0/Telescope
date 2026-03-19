# Project Context

- **Owner:** Adam Dost
- **Project:** Telescope — AKS-first Kubernetes IDE (Rust/Svelte/Tauri v2, desktop-only)
- **Stack:** Rust (crates/core, crates/engine, crates/azure), Svelte 5 (apps/web), Tauri v2 (apps/desktop)
- **Created:** 2026-03-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->

### 2026-03-19 — K8s Capability Gap Analysis

- **29 GVKs watched** via `ALL_WATCHED_GVKS` in `main.rs` — this is the definitive list of what the cache tracks.
- **66 Tauri IPC commands** in `invoke_handler` — the desktop command surface.
- **65 frontend API functions** in `apps/web/src/lib/api.ts`.
- **39 page routes** in `apps/web/src/routes/`.
- Helm is **read-only + rollback** — no install, upgrade, or uninstall. This is the biggest functional gap.
- ReplicaSets, ClusterRoles, and ClusterRoleBindings are watched but have **no dedicated list routes** (accessible via generic resource detail).
- VPA is not supported; this is acceptable since it's a CRD add-on covered by the dynamic resource browser.
- Legacy v1/Endpoints not supported; superseded by EndpointSlices which are fully covered.
- `spawn_watch_task` in `main.rs` is the orchestrator — 9 cluster-scoped + 19 namespace-scoped watchers + pods = 29 total.
- Azure AKS management plane has **complete CRUD coverage**: cluster lifecycle, node pools, upgrades, maintenance, identity, cloud selection.
- Audit logging is wired into every mutating IPC command via `write_audit_entry`.
- Key file paths: `main.rs` (IPC surface + watcher spawn), `watcher.rs` (watch implementations), `api.ts` (frontend API), `actions.rs` (scale/delete/restart/apply).

### 2026-03-19 — Cross-Agent Audit Summary

All four agents (Dallas, Ripley, Lambert, Kane) completed the K8s capability audit. Results: backend verified against live cluster with zero failures (Ripley). Frontend surface matches backend — 65 API functions, 39 routes, all GVKs reachable (Lambert). All tests green: Rust 176/176, Web 36/36, E2E 16/16 (Kane). Only actionable gap: Helm write ops. Decision: ship v1.0.0 as-is.

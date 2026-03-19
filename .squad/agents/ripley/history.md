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

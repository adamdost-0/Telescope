# Project Context

- **Owner:** Adam Dost
- **Project:** Telescope — AKS-first Kubernetes IDE (Rust/Svelte/Tauri v2, desktop-only)
- **Stack:** Rust (crates/core, crates/engine, crates/azure), Svelte 5 (apps/web), Tauri v2 (apps/desktop)
- **Created:** 2026-03-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->
- 2026-03-19: Frontend audit confirmed that search/detail routing is centralized in `resource-routing.ts`, giving broad GVK coverage (including cluster-scoped + dynamic CRDs) even when list pages are tabbed or absent.

### 2026-03-19 — Cross-Agent Audit Summary

Dallas confirmed 29 GVKs, 66 IPC commands, near-complete K8s coverage — only gap is Helm write ops. Ripley verified all kubectl/helm commands against live cluster with zero failures. Kane confirmed all tests green (Rust 176/176, Web 36/36, E2E 16/16). Decision: ship v1.0.0 as-is.


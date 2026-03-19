# Project Context

- **Owner:** Adam Dost
- **Project:** Telescope — AKS-first Kubernetes IDE (Rust/Svelte/Tauri v2, desktop-only)
- **Stack:** Rust (crates/core, crates/engine, crates/azure), Svelte 5 (apps/web), Tauri v2 (apps/desktop)
- **Created:** 2026-03-19

## Learnings

<!-- Append new learnings below. Each entry is something lasting about the project. -->
- 2026-03-19: `cargo test --workspace --exclude telescope-desktop --all-features` currently reports 176/176 passing, but `crates/engine/tests/integration_k3d.rs` only exercises real cluster paths when `K3D_TEST=1`; otherwise it exits early and passes.
- 2026-03-19: E2E Kubernetes UI confidence is strongest for mocked AKS node-pool lifecycle flows (`tests-e2e/node-pools.spec.ts`) and is backed by 45 mocked IPC commands in `apps/web/tests-e2e/helpers/mock-tauri.ts`.

### 2026-03-19 — Cross-Agent Audit Summary

Dallas confirmed 29 GVKs, 66 IPC commands, near-complete coverage — only gap is Helm write ops. Ripley verified all kubectl/helm commands against live cluster with zero failures. Lambert confirmed frontend matches backend: 65 API functions, 39 routes, all GVKs reachable. Decision: ship v1.0.0 as-is.

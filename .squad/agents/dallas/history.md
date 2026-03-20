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

### 2026-03-19 — Post-Audit Work Prioritization

- Helm write ops classified as **P1** (should fix, next release), not P0. Rationale: v1.0.0 shipped clean, users have CLI fallback, and chart repo management expands scope if rushed.
- **Uninstall is the quick win** — simplest Helm write op (no chart input needed), can ship independently. Recommended as immediate parallel work for Ripley + Lambert.
- Install and upgrade should be designed together since they share chart input UX patterns. Template/dry-run layers on top after those ship.
- Missing list routes (ReplicaSets, ClusterRoles, ClusterRoleBindings) are P2 — data already watched and accessible, just missing dedicated pages. Lambert can batch these anytime.
- ARM error handling excluded from this list — Ripley and Lambert already own that work stream separately.
- Key architectural note for Helm install: start with simple chart reference strings, not a full chart browser/repo manager. Iterate from there.

### 2026-03-19 — ARM Error Handling Session (Lead)

Led prioritization session alongside ARM error handling work by Ripley+Lambert+Kane. ARM error handling delivered: typed backend errors, frontend banner, test coverage, silent-failure bug fix. All tests green. Work priorities decision accepted — Helm writes P1, missing routes P2.

### 2026-03-19 — v1.0.7 Release

- Wrote thorough CHANGELOG entry covering all work since v1.0.5: Helm uninstall end-to-end, three P2 resource list routes, ARM error handling fixes, and E2E test suite doubling.
- Resolved a merge conflict in `apps/web/src/lib/api.ts` during rebase (kept `toError(e)` wrapper over raw `e` — the more robust error handling path).
- Pushed main, tagged `v1.0.7`, pushed tag to trigger release automation.
- CHANGELOG follows Keep a Changelog format with Added, Fixed, Changed, Testing, and Internal sections.
- Lesson: always `git pull --rebase` before pushing when the remote may have advanced — the squad's parallel work means main moves frequently.

### 2026-03-19 — GitHub Pages Documentation Site

- Architecture decision: **just-the-docs** theme (v0.10.1) via `remote_theme` — clean sidebar nav, built-in search, mermaid diagram rendering, dark mode default.
- Navigation hierarchy: 8 public pages ordered by importance (Architecture → Quickstart → Deployment → Security → Testing → Roadmap → UX Reference → Smoke Test). 3 internal pages (PRD, Test Plan, Entra Auth) marked `nav_exclude: true` — accessible by URL but hidden from sidebar.
- AGENTS.md, retrospectives/, and diagrams/ excluded from Jekyll processing via `_config.yml` exclude list.
- GitHub Pages deploys from `docs/` folder on push to main, with `actions/deploy-pages@v4` workflow. Mermaid v11 enabled for architecture diagram rendering.
- Key file paths: `docs/_config.yml` (Jekyll config), `docs/index.md` (landing page), `docs/Gemfile` (Ruby deps), `.github/workflows/docs.yml` (deploy workflow).
- Existing content untouched beyond adding YAML front matter blocks — no content was deleted or restructured.
- CHANGELOG linked from landing page to repo root (external link) rather than duplicated in docs/.

### 2026-03-19 — GitHub Pages + README Session (Scribe)

Dallas led the GitHub Pages docs structure workstream. Created `docs/_config.yml` (just-the-docs, dark mode), `docs/index.md` (landing page), `.github/workflows/docs.yml` (deploy workflow). Added YAML front matter to all 11 docs files — 8 public sidebar, 3 internal nav_exclude. Lambert ran parallel README cleanup. Decision merged to decisions.md.

### 2026-03-20 — architecture-doc-current-state

- **Docs alignment:** Audited `docs/` and top-level guidance files and reconciled content against the shipped code and CI. Updated `docs/ARCHITECTURE.md` to remove an incorrect SvelteKit version marker and to clarify the frontend packaging and workspace shape.
- **Authoritative files confirmed:** `crates/core`, `crates/engine`, `crates/azure`, `apps/web`, `apps/desktop/src-tauri/src/main.rs`, `crates/engine/src/watcher.rs`, `apps/web/src/lib/api.ts`.
- **Pattern / preference:** When docs disagree with code or CI, the code wins. Prefer small, surgical doc fixes that reference concrete files.
- **Outcome:** Documentation now accurately reflects the desktop-only shipped system and workspace shape.


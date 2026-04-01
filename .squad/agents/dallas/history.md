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


### 2026-03-20 — Merged architecture-doc-current-state decision

- Merged decision from .squad/decisions/inbox/dallas-architecture-doc-current-state.md into .squad/decisions.md
- Files produced: docs/ARCHITECTURE.md, .squad/agents/dallas/history.md
- Outcome: Documentation aligned to shipped desktop architecture; small surgical doc edits preferred when code/CI disagree.

### 2026-03-24 — AI Insights Implementation Planning

- Finalized the cross-agent AI Insights implementation plan in `docs/plans/2026-03-24-ai-insights-implementation.md`.
- Durable scope locks: dedicated `/insights` route, explicit Azure login or API key auth, encrypted local history capped to 3 entries per cluster, allowlist-only context shaping, and Settings-only dev diagnostics.
- Keep ownership narrow: `crates/azure` for provider auth/cloud/error handling, `crates/engine` for context and schema, `crates/core` for encrypted history, `apps/desktop/src-tauri` for secure storage and thin commands, `apps/web` for route and settings UX.

### 2026-03-24 — No-Emoji Policy

- Standardize on plain text labels or the icon registry across docs, prompts, orchestration outputs, and UI.
- When existing files are touched, replace emoji-style status markers with markdown checkboxes or neutral headings.

### 2026-03-24 — AI Insights Execution Start

- `modelName: Option<String>` is a contract invariant across the Rust schema and frontend persistence wrapper; storing `""` is only safe if load normalizes it back to `null`.
- Reviewer-gated AI contract work should fail closed on unknown fields and use the dedicated AI settings keys instead of mutating shared Azure preferences.
- Azure OpenAI sovereign-cloud correctness requires the selected cloud to configure both endpoint validation and `DefaultAzureCredential` authority construction.
- Provider-side 401s need separate AI-specific classification from local credential acquisition failures so Settings guidance stays accurate.

### 2026-03-24 — AI Insights Deficiency Fix Review

- Reviewed and approved three deficiency fixes: JSON schema null-description serialization in `response_format_json()`, HTTP 408/504/429 classification in `classify_openai_response_error()`, and five new context-builder cap and filtering tests.
- Key review finding: the `#[serde(skip_serializing_if)]` on `AzureOpenAiResponseFormatJsonSchema.description` is redundant since the struct is never directly serialized to the wire -- the manual `Value` builder controls the actual payload. Harmless but could mislead contributors.
- All fixes are narrow, contract-aligned, and correctly ordered (408/504/429 branches follow 401/403/404 checks). No regressions.

### 2026-04-01 — Security Issues #200, #201, #202 Verification & Validation Planning

Led multi-agent planning pass to scope verification vs. validation test strategy for three filed security issues. Produced unified layered test framework (unit/regression, integration, acceptance) across all three issues with evidence-based closure criteria. Framework emphasizes traceability, correctness, regression resistance, and real-world fitness. Execution order: #200, #202 (code-behavior fixes) before #201 (dependency remediation). Plan delivered in session state at `6d7acee3-3261-4394-897c-b4c39f85426a/plan.md`.


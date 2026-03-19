# Squad Decisions

## Active Decisions

### 2026-03-19: K8s Capability Audit — Ship v1.0.0

**Authors:** Dallas (lead), Ripley, Lambert, Kane  
**Status:** Accepted  
**Type:** Architecture audit

**Context:** Full-stack audit of Telescope v1.0.0 K8s capabilities across engine, IPC, frontend, and test layers. Validated against live AKS cluster `dassadsawqew`.

**Findings:**
- 29 watched GVKs, 66 Tauri IPC commands, 65 frontend API functions, 39 routes — near-complete coverage
- All kubectl/helm live-cluster commands succeeded; zero failures
- All tests green: Rust 176/176, Web 36/36, E2E 16/16
- Frontend builds clean

**Gaps:**
| Gap | Severity | Notes |
|---|---|---|
| Helm install/upgrade/uninstall | Medium | Read-only + rollback today; no chart install or upgrade |
| Helm template/dry-run | Low | Pairs with install/upgrade when those ship |
| ReplicaSets list route | Low | Watched, accessible via generic detail |
| ClusterRoles list route | Low | Watched, accessible via generic detail |
| ClusterRoleBindings list route | Low | Watched, accessible via generic detail |

**Acceptable non-gaps:** VPA (CRD add-on, covered by CRD browser), legacy v1/Endpoints (superseded by EndpointSlices), Helm chart repos (CLI scope).

**Decision:** Ship v1.0.0 as-is. Only post-release priority is Helm write operations.

---

### 2026-03-19: SOTA Models Only

**Author:** Adam Dost (directive)  
**Status:** Active  
**Type:** Team policy

Only use latest SOTA models for all agent spawns: **Opus 4.6** and **GPT-5.3-Codex**. Haiku is forbidden.

---

### 2026-03-19: Post-Audit Work Priorities

**Author:** Dallas (Lead)  
**Status:** Accepted  
**Type:** Prioritization

**Context:** Prioritized backlog from the K8s capability audit. v1.0.0 shipped clean — these are post-release improvements.

**Items:**
| ID | Item | Priority | Owner | Depends on |
|---|---|---|---|---|
| P1-1 | Helm install | P1 | Ripley + Lambert | — |
| P1-2 | Helm upgrade | P1 | Ripley + Lambert | P1-1 |
| P1-3 | Helm uninstall | P1 ✅ | Ripley | — |
| P1-4 | Helm template/dry-run | P1 (lower) | Ripley + Lambert | P1-1, P1-2 |
| P2-1 | ReplicaSets list route | P2 ✅ | Lambert | — |
| P2-2 | ClusterRoles list route | P2 ✅ | Lambert | — |
| P2-3 | ClusterRoleBindings list route | P2 ✅ | Lambert | — |

**Decision:** Helm writes are P1, not P0. Recommended sequence: uninstall first (quick win), then install+upgrade together, then template/dry-run. Missing list routes are P2 — batch when convenient.

---

### 2026-03-19: ARM Node Pool Error Handling

**Authors:** Ripley (backend), Lambert (frontend), Kane (tests)  
**Status:** Accepted  
**Type:** Bug fix + UX improvement

**Context:** ARM node pool failures were surfaced as generic errors or silently swallowed. `listAksNodePools` returned `[]` on error, hiding failures. `delete_node_pool` polling treated any GET error as successful deletion.

**Changes:**
- Backend: Typed ARM error variants (TokenExpired, SubscriptionNotFound, ResourceGroupNotFound, ClusterNotFound, PermissionDenied, Timeout) with actionable messages
- Frontend: Dismissible error banner on node-pools page with guidance mapping; `listAksNodePools` now rethrows after notification
- Bug fix: `delete_node_pool` only treats `NotFound` as successful disappearance; other errors propagate
- Tests: Rust unit tests for error mapping, Playwright E2E for error display/dismiss/retry recovery, mock-tauri error injection support

**Decision:** Keep node pool inventory as authoritative ARM `agentPools` reads. Treat ARM failures as first-class user-visible errors with actionable remediation.

---

### 2026-03-19: Helm Uninstall Implementation

**Authors:** Ripley (engine + IPC), Kane (tests)  
**Status:** Accepted  
**Type:** Feature implementation

**Context:** Helm uninstall was prioritized as the first Helm write operation (P1-3) because it is the smallest safe slice — no chart payloads, only release identity and namespace.

**Changes:**
- Engine: `telescope_engine::helm::helm_uninstall(namespace, name)` with trusted binary resolution, input validation, CLI execution via `tokio::process::Command(kill_on_drop)`, and categorized error messaging (release-not-found, permission denied, timeout)
- Desktop IPC: `helm_uninstall` Tauri command with namespace/name validation, audit logging, registered in `generate_handler!`
- Tests: 3 new Rust unit tests for uninstall error categorization (engine tests 94→97); E2E specs for action/confirm/success/error flows

**Decision:** Follows established Helm rollback pattern. Reuses existing Kubernetes-name validator and trusted Helm binary resolution.

---

### 2026-03-19: P2 Resource List Routes (ReplicaSets, ClusterRoles, ClusterRoleBindings)

**Authors:** Lambert (frontend), Kane (tests)  
**Status:** Accepted  
**Type:** Feature implementation

**Context:** Per the post-audit P2 backlog, these three resource types were already watched by the backend but lacked dedicated frontend list pages.

**Changes:**
- Frontend: 3 new list routes under `apps/web/src/routes/resources/` (replicasets, clusterroles, clusterrolebindings) using standard pattern (`getResources`, `FilterBar`, `ResourceTable`, Svelte 5 runes)
- Routing: Updated `resource-routing.ts` with list/detail mappings for all three types
- Navigation: Sidebar links and search palette entries for all three types
- Tests: E2E specs for route rendering, columns, detail click-through, search palette discovery, loading/error states; `commandDelays` mock-tauri support

**Columns:**
- ReplicaSets: name, namespace, desired/current/ready replicas, age
- ClusterRoles: name, creation timestamp, rules count
- ClusterRoleBindings: name, role ref, subjects, creation timestamp

**Decision:** No new API wrappers needed — `getResources(gvk)` covers all three. Completes the P2 backlog from the post-audit prioritization.

---

### 2026-03-19: Helm Uninstall UI Wiring

**Authors:** Lambert (frontend)  
**Status:** Accepted  
**Type:** Feature implementation (UI layer)

**Context:** Helm uninstall engine + IPC were already delivered (P1-3). This completes the user-facing UI layer — confirmation dialog, success/error notifications, and post-action refresh.

**Changes:**
- Frontend: Wired `helmUninstall` into `apps/web/src/routes/helm/+page.svelte` with per-row Uninstall action button, `$state`-driven ConfirmDialog, success/error notification banners, and post-action list refresh
- UX: ConfirmDialog role set to `dialog` for Playwright E2E compatibility
- Validation: `pnpm build` ✅, `pnpm test` ✅ (36/36), helm uninstall E2E tests pass

**Decision:** Follows the established Helm rollback action pattern. P1-3 (Helm uninstall) is now fully delivered end-to-end: engine → IPC → UI → tests.

---

### 2026-03-19: GitHub Pages Documentation Site Structure

**Author:** Dallas (Lead)  
**Status:** Accepted  
**Type:** Documentation architecture

**Context:** Telescope needed a public-facing documentation site. The `docs/` directory had 11+ markdown files with no navigation, theming, or deployment pipeline.

**Changes:**
- Theme: `just-the-docs` v0.10.1 via `remote_theme` — searchable, sidebar navigation, native mermaid support
- Color scheme: Dark mode default (matches the IDE aesthetic)
- Deployment: GitHub Actions workflow (`docs.yml`) with `actions/deploy-pages@v4`
- Navigation: 8 public sidebar pages (nav_order 2–9), 3 internal pages (`nav_exclude: true`), AGENTS.md/retrospectives/diagrams excluded from Jekyll
- Landing page: `docs/index.md` with project overview, navigation table, tech stack links
- No content changes: Only front matter additions to existing files

**Decision:** `just-the-docs` is the standard for open-source technical docs. Internal planning docs stay accessible by URL but hidden from public navigation. CHANGELOG linked externally to avoid duplication drift.

---

### 2026-03-19: README Cleanup and Restructure

**Author:** Lambert (Frontend Dev)  
**Status:** Accepted  
**Type:** Documentation improvement

**Context:** README needed to be cleaner, more scannable, and compatible with both GitHub rendering and GitHub Pages/Jekyll publishing.

**Changes:**
- Hero section with badges and concise value proposition
- Feature highlight reel replacing long emoji bullet lists
- Scannable feature matrix tables for Kubernetes, Azure, and desktop capabilities
- Streamlined quick start and normalized development commands
- Clean footer with license, docs, and contributing links
- Pure markdown throughout (no raw HTML)

**Decision:** All existing information preserved; restructured for scannability. Markdown-only approach reduces rendering risk across GitHub and Jekyll contexts.

---

## Governance

- All meaningful changes require team consensus
- Document architectural decisions here
- Keep history focused on work, decisions focused on direction

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to follow Semantic Versioning.

GitHub releases use the matching version section from this changelog when present, so curated release notes can include issue references when they improve traceability.

## [Unreleased]

## [v1.0.7] - 2026-03-19

### Added
- **Helm Uninstall (end-to-end):** First Helm write operation — `helm uninstall` is now fully supported from engine through UI. The engine function (`telescope_engine::helm::helm_uninstall`) handles trusted binary resolution, input validation, and categorized error messaging (release-not-found, permission denied, timeout). A new `helm_uninstall` Tauri IPC command exposes the operation to the desktop surface with namespace/name validation and audit logging. The frontend adds a per-row Uninstall action button on the Helm releases page, a `$state`-driven confirmation dialog, success/error notification banners, and automatic list refresh after action completion.
- **ReplicaSets list route:** Dedicated resource list page with columns for name, namespace, desired/current/ready replicas, and age. Includes sidebar navigation link, search palette entry, and click-through to resource detail view.
- **ClusterRoles list route:** Dedicated resource list page with columns for name, creation timestamp, and rules count. Includes sidebar navigation link, search palette entry, and click-through to resource detail view.
- **ClusterRoleBindings list route:** Dedicated resource list page with columns for name, role ref, subjects, and creation timestamp. Includes sidebar navigation link, search palette entry, and click-through to resource detail view.
- Resource routing updated with list/detail mappings for all three new resource types.

### Fixed
- **ARM node pool silent failure:** `listAksNodePools()` previously returned an empty array when the Azure ARM API returned an error, hiding failures from users. The function now propagates typed error variants — `TokenExpired`, `SubscriptionNotFound`, `ResourceGroupNotFound`, `ClusterNotFound`, `PermissionDenied`, `Timeout` — with actionable error messages. The frontend node-pools page displays a dismissible error banner with remediation guidance and supports retry recovery.
- **ARM delete polling swallowed errors:** The `delete_node_pool` polling path previously treated any GET error as successful deletion. Now only `NotFound` responses are treated as confirmation the resource has been removed; all other errors propagate correctly.

### Changed
- Helm uninstall follows the established Helm rollback action pattern and reuses the existing Kubernetes-name validator and trusted Helm binary resolution.
- All three new resource list routes follow the standard frontend pattern using `getResources(gvk)`, `FilterBar`, `ResourceTable`, and Svelte 5 runes — no new API wrappers were needed.

### Testing
- **E2E test suite doubled:** E2E specs grew from 16 to 32 tests.
- **ARM error handling tests:** Rust unit tests for ARM error variant mapping; Playwright E2E tests for error banner display, dismiss interaction, and retry recovery. Includes mock-tauri error injection support.
- **Helm uninstall tests:** 3 new Rust unit tests for uninstall error categorization (engine test count 94 → 97). E2E specs covering the action trigger, confirmation dialog, success notification, and error notification flows.
- **P2 route tests:** E2E coverage for all three new list routes including table column rendering, detail navigation click-through, search palette discovery, and loading/error states. Added `commandDelays` mock-tauri support.

### Internal
- Full K8s capability audit completed across all layers: 29 watched GVKs, 66+ Tauri IPC commands, 65 frontend API functions, 39+ page routes. All audit gaps from v1.0.0 now resolved.
- Confirmed ARM node pool listing uses real Azure ARM REST API calls (not Kubernetes API passthrough).
- 82 files changed, ~8,000 lines of new code across Rust engine, Tauri IPC, SvelteKit frontend, and test layers.

## [v1.0.0] - 2026-03-13
### Added
- Completed the M9 desktop resource expansion with coverage across 16 primary Kubernetes resource blades and 28+ watched resource types in the desktop cache.
- Completed the M10 Azure management-plane milestone with the new `telescope-azure` ARM client, AKS node pool CRUD, cluster start/stop controls, upgrade management, and ARM-backed diagnostics.

### Changed
- Finalized Telescope as a desktop-only Tauri application with the SvelteKit frontend packaged exclusively for native distribution.
- Expanded the desktop IPC surface to 60+ Tauri commands spanning Kubernetes operations, Helm workflows, metrics, preferences, and Azure ARM actions.

### Removed
- Removed the discontinued non-desktop delivery stack and legacy browser-only workflow references.

## [v1.0.0-rc5] - 2026-03-13
### Changed
- Finalized release-candidate polish for the desktop-only packaging flow and Azure ARM-driven AKS management experience.
- Refined release notes, contributor guidance, and top-level docs to match the v1.0.0 crate layout (`core`, `engine`, `azure`, desktop).

## [v1.0.0-rc4] - 2026-03-12
### Changed
- Stamped the release tag into build artifacts and output filenames in CI.

## [v1.0.0-rc3] - 2026-03-12
### Fixed
- Resolved version, secret handling, ingress, and PVC regressions ahead of the release candidate.

## [v1.0.0-rc2] - 2026-03-12
### Changed
- Codified the repository push and release tagging policy.
- Improved AI guidance used for contributors and agents.
- Shipped resource blade updates across the desktop UI.

## [v1.0.0-rc1] - 2026-03-12
### Added
- Cut the first v1.0.0 desktop release candidate with broader desktop validation, packaging, and resource workflow coverage.
- Landed additional desktop-facing Kubernetes resource pages and operational guardrails needed for the release train.

## [v0.9.0-beta] - 2026-03-12
### Added
- Finalized more of the desktop experience with broader watchers, container details, and action coverage.
- Added richer container details, including image, owner, resource, and error-state information.
- Expanded tests around the late beta desktop flow.

## [v0.8.0-beta] - 2026-03-12
### Changed
- Recalibrated the roadmap and added AKS quick-start documentation.

### Fixed
- Audited `unwrap()` usage and improved UI error handling.
- Hardened port-forward limits, exec output caps, and schema migrations.

## [v0.7.0-alpha] - 2026-03-12
### Added
- Added keyboard shortcuts with a help overlay.
- Added breadcrumb navigation and a Ctrl+K search palette.
- Added CRD discovery with auto-generated views.
- Added a preferences page, a reusable filter bar, and tri-state table sorting.
- Added structured audit logging for destructive operations and server-side redaction for sensitive Helm values.

### Fixed
- Addressed multiple security issues.
- Fixed cluster-wide watch handling for AKS kube-system events.
- Resolved UI bugs around Unicode escapes, refresh state, and button flicker.

## [v0.6.0-alpha] - 2026-03-12
### Added
- Added Helm release listing, release detail, values editing, rollback, and copy-upgrade-command flows.
- Added pod and node CPU/memory metrics, plus namespace usage sparklines.

### Changed
- Updated architecture documentation and diagrams to reflect the implemented system.
- Rewrote the README with the feature list, comparison table, quick start, and roadmap.

## [v0.5.0-alpha] - 2026-03-11
### Added
- Added production-context guardrails for destructive actions.
- Added AKS auth detection and badges, cluster info, and Azure Portal links.
- Added AKS add-on status and Azure Workload Identity details in the UI.
- Added node-pool grouping for AKS nodes.
- Wired scale and port-forward dialogs into the app.

### Fixed
- Added missing Tauri commands for resource loading and port forwarding.
- Resolved several UX bugs during the AKS-first milestone.

## [v0.4.0-alpha] - 2026-03-11
### Added
- Added a create-resource page with Kubernetes templates.
- Added YAML editing with apply and dry-run support.
- Added exec terminal support for non-interactive container commands.
- Added rollout restart and rollout status for Deployments.
- Added a cluster overview dashboard, nodes pages, generic resource detail pages, and delete confirmations.

### Changed
- Rewrote the roadmap and PRD around a Lens-parity plan.

### Fixed
- Merged port-forward implementation and conflict fixes.

## [v0.3.0-alpha] - 2026-03-11
### Added
- Added generalized Kubernetes resource watching in the engine.
- Added resource list pages for Deployments, Services, ConfigMaps, and Secrets.
- Added a pod detail page with tabs and clickable resource rows.
- Added broader resource browsing, log viewing, events, sidebar work, and UX polish.

## [v0.2.0-alpha] - 2026-03-11
### Added
- Wired the desktop app to a live Kubernetes connection lifecycle.
- Added the Phase 3 end-to-end Kubernetes connection flow.
- Extracted shared Svelte stores for context and namespace state.
- Added smoke-test documentation and memory measurement scripts.

### Fixed
- Bundled `@tauri-apps/api` in the web app to restore IPC.
- Auto-connected to the active kubeconfig context on startup.
- Resolved audit findings and critical Rust issues in the desktop app.
- Switched cluster listing from HTTP fetches to Tauri IPC.

## [v0.1.0-alpha] - 2026-03-10
### Added
- Introduced the initial Rust and pnpm workspace scaffold for Telescope.
- Added the `/clusters` route, unified cluster API work, and deterministic E2E coverage for stubbed clusters.
- Added context and namespace switching in the web UI.
- Implemented the first design-review phases and Windows desktop build CI.

### Changed
- Migrated the repo to pnpm and a container-first dev loop.

### Fixed
- Corrected Tauri config and frontend output handling for desktop packaging.
- Added valid placeholder icons for Windows bundling.
- Fixed release workflow artifact paths and setup-node caching in CI.

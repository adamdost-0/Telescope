# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project aims to follow Semantic Versioning.

## [Unreleased]

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
- Shipped resource blade updates across the UI.

## [v1.0.0-rc1] - 2026-03-12
### Added
- Added the `telescope-hub` Axum HTTP server for browser mode.
- Wired the shared web API layer to hub HTTP endpoints.
- Added OIDC authentication scaffolding in the hub.
- Added Kubernetes user impersonation and an audit log viewer in hub mode.

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

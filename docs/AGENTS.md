# Agent Guidance — Documentation

## Overview

`docs/` contains architecture notes, product docs, security/testing guidance, and planning material for Telescope — a **desktop-only** AKS-first Kubernetes IDE built with Tauri v2, SvelteKit, and Rust.

**Version:** v1.0.0 — Telescope is a shipped desktop application. There is no browser or hub mode.

## Documentation Files

| File | Status | Notes |
|------|--------|-------|
| `ARCHITECTURE.md` | Current | Crate layering (`desktop → engine + azure + core`; `engine → core`; `azure → core`) and Tauri IPC model |
| `PRD.md` | Current + vision | Product goals, milestone tracking, shipped vs planned features |
| `ROADMAP.md` | Current planning | Shipped, partial, and planned milestones |
| `TESTING.md` | Current | Rust unit tests, Vitest, Playwright E2E suites |
| `SECURITY.md` | Current + planned | Audit logging, secret redaction, production-context warnings; future auth roadmap |
| `DEPLOYMENT.md` | Current | Desktop deployment and release process |
| `UX_NOTES.md` | Current | Route inventory (~39 pages) and UI design patterns |
| `TEST_PLAN.md` | Aspirational | Detailed test scenarios; many not yet automated |
| `SMOKE_TEST.md` | Aspirational | Manual checklist, not automated |
| `AKS_QUICKSTART.md` | Aspirational | Deployment guidance ahead of current packaging |
| `diagrams/architecture.md` | Reference | Architecture diagrams |
| `retrospectives/` | Historical | Iteration retrospectives and learnings |

## How to Read These Docs

**Rule of thumb:** when docs contradict code or CI, the code wins.

### Still aspirational
1. **`TEST_PLAN.md` / `SMOKE_TEST.md`** — more exhaustive than the current automated suite.
2. **`AKS_QUICKSTART.md`** — target deployment guidance, not proof that all packaging exists.
3. **`SECURITY.md`** — production-grade OIDC/JWT validation remains planned.

## Current Feature Inventory (v1.0.0)

### Kubernetes Engine (28+ resource types)
Real watch-backed sync for: pods, deployments, services, configmaps, secrets, events, nodes, statefulsets, daemonsets, replicasets, jobs, cronjobs, ingresses, network policies, endpoint slices, PVCs, resource quotas, limit ranges, roles, cluster roles, role bindings, cluster role bindings, service accounts, HPAs, pod disruption budgets, priority classes, validating/mutating webhooks, storage classes, persistent volumes.

Additional engine capabilities:
- Pod logs (streaming), exec, port-forward
- Create/apply/delete/scale/rollout-restart for resources
- Dynamic resource operations for arbitrary GVKs
- CRD listing and custom resource CRUD
- Helm release browsing, history, values, rollback
- Metrics-server-backed pod and node metrics
- Node ops: cordon, uncordon, drain, taint management
- Namespace CRUD, secret retrieval, audit logging

### Azure ARM Management
- ARM client supporting Azure Public, Government, and air-gapped clouds
- AKS cluster detail, start/stop
- Node pool CRUD: list, create, scale, delete, autoscaler config
- Upgrade profiles: cluster and per-pool version upgrades, node image upgrades
- Maintenance configuration listing
- AKS identity resolution from kubeconfig or stored preferences

### Desktop UI (39 pages, 25 components)
- Overview dashboard, pod/node/event views with detail pages
- 18 dedicated resource-type list pages under `/resources/*`
- CRD browser with group/kind drill-down
- Helm release management with namespace/name detail
- Azure node pool management page
- Settings, create-resource, namespace management
- Search palette, keyboard shortcuts, theme toggle, breadcrumbs
- Production-context visual warnings, connection status indicator

### Automated Test Surface
- Rust unit tests across `core`, `engine`, and `azure` crates
- Vitest unit tests (azure-utils, error-suggestions, prod-detection, version)
- Playwright E2E tests (smoke, settings, node-pools, search-palette, detail-reload, error-states) against mock-tauri stub

## Updating Documentation

### When to Update
1. **Architecture changes:** update `ARCHITECTURE.md` when crate boundaries or IPC model change.
2. **Security changes:** update `SECURITY.md` when audit, redaction, or auth behavior changes.
3. **Testing changes:** update `TESTING.md` when new test suites or coverage areas land.
4. **Route/UX changes:** update `UX_NOTES.md` when page inventory or navigation changes.
5. **Milestone shifts:** update `PRD.md` and `ROADMAP.md` when planned work ships.

### Keeping Docs Grounded
Before merging doc updates:
1. Verify the code path or route actually exists.
2. Confirm CI/workflow commands match `.github/workflows/ci.yml`.
3. Use "implemented / partial / planned" language.
4. Mark future sections with `(Planned)` or `NOTE` admonitions.
5. Validate that any doc change affecting build/test references still passes `./scripts/dev-test.sh`.

## Documentation Conventions
- GitHub-flavored Markdown.
- Link to actual repo files when describing code-backed behavior.
- Use tables for inventories and status summaries.
- Clearly label aspirational content.
- When documenting validation or testing workflows, always reference the container-first path (`./scripts/dev-test.sh`) as the primary method. Host-only commands should be labeled as fallback or desktop-specific.

## Retrospectives
`docs/retrospectives/` contains per-iteration learnings: what worked, what didn't, action items.

# Telescope — PRD

> Working title: **Telescope** (aka "Aurora" in earlier drafts)

## 1) Summary
Telescope is an open-source Kubernetes IDE that targets Lens-style day-1/day-2 workflows while being **AKS-aware**, **lighter weight**, and delivered as a **desktop app** built with Tauri.

## 2) Goals / Success Criteria
### Product goals
- Feature parity with Lens for core operations: cluster/context management, resource explorer, logs/exec/port-forward, Helm, and baseline metrics.
- AKS-first experience: Azure auth hints, node pool visibility, add-on visibility, and AKS-specific troubleshooting clues.
- Memory efficiency: materially lower baseline/peak than Electron-class apps.
- Desktop operator experience: consistent, responsive, local-first workflows for day-2 Kubernetes operations.

### Success metrics (targets)
- **Time-to-first-AKS-cluster**: median < 3 minutes.
- **Desktop memory** (idle, 1 cluster connected): < 250–350MB target (excluding OS webview).
- 90%+ completion of common ops without switching to kubectl for: logs, exec, port-forward, Helm release ops, events, and baseline metrics.

## 3) Personas
- **AKS App Operator**: on-call, needs fast logs/exec/events/port-forward.
- **Platform Engineer**: manages node pools, CRDs, cluster-wide resources; needs Helm + safe operations.
- **Developer (namespace-scoped)**: wants a focused, low-noise view on their workstation.
- **Security/Compliance**: wants clear permissions, auditability, and safe defaults.

## 4) Key workflows (must be excellent)
1. Connect to AKS/import kubeconfigs and switch context/namespace quickly.
2. Explore resources quickly by kind/namespace, including CRDs and nodes.
3. Debug a failing workload with logs + exec + events + port-forward + rollout actions.
4. Manage Helm releases with release list/detail/history/values and rollback, then add upgrade/diff UX.
5. Show baseline metrics without requiring a full Prometheus stack.
6. Deliver a polished, auditable desktop experience for cluster operators.

## 5) Lens Parity Checklist

> Status: ✅ = shipped, 🚧 = partially shipped / desktop-first, 🔲 = planned

### Cluster Management
- ✅ Kubeconfig import/merge and context discovery
- ✅ Context list + switching with connection state machine
- ✅ Namespace switching
- ✅ Production-context detection and stronger confirmations in the UI
- 🔲 Context favorites / pinning

### Resource Explorer
- ✅ Resource browsing for Pods, Deployments, StatefulSets, DaemonSets, Jobs, CronJobs, Services, Ingresses, ConfigMaps, Secrets, PVCs, Events, and Nodes
- ✅ Collapsible sidebar with grouped navigation
- ✅ Cluster overview dashboard
- ✅ Node list/detail with capacity, usage, and conditions
- 🚧 CRD definition discovery and instance browsing
- 🚧 Per-list text filtering
- ✅ Global search palette (cached-resource search)
- 🔲 Rich label filtering and broad table sorting

### Debug Loop
- ✅ Pod detail page (Summary / Logs / Exec / Events / YAML tabs)
- ✅ Log viewer: streaming/snapshot, container selector, search, previous logs, auto-scroll
- ✅ Events viewer with filtering
- 🚧 Exec terminal (currently non-interactive command execution rather than a full terminal)
- 🚧 Port-forward (basic pod-focused flow today)

### Resource Actions
- ✅ Resource YAML viewing
- 🚧 YAML edit + dry-run/apply for supported built-in namespaced resources
- 🚧 Delete resource support (engine coverage is broader than current UI exposure)
- ✅ Scale Deployments/StatefulSets
- 🚧 Rollout restart/status for supported workloads
- ✅ Create resource from templates/YAML

### Helm
- ✅ Release list/detail
- ✅ Values viewer with redaction of known sensitive keys
- ✅ Revision history
- ✅ Rollback
- 🔲 Upgrade with diff preview

### Metrics
- ✅ metrics-server discovery
- ✅ Pod CPU + memory usage
- ✅ Node CPU + memory usage
- ✅ Basic trend charts/sparklines
- 🚧 Namespace usage summaries

### UX & Polish
- ✅ Dark theme
- ✅ Light theme + theme toggle
- ✅ Keyboard shortcuts + help overlay
- ✅ Breadcrumb navigation
- ✅ Search palette
- 🚧 Settings / preferences page
- 🔲 Tabbed workspace
- 🔲 Hotbar / quick actions

### AKS-Specific (our differentiator)
- 🚧 kubelogin / exec-auth hints and troubleshooting
- 🚧 Azure identity and workload identity hints
- ✅ Node pool visibility from AKS labels
- 🚧 AKS add-on status heuristics
- 🚧 Azure Portal deep links
- ✅ Production guardrails

### Extensions
- 🔲 WASM/plugin host
- 🔲 Plugin permissions model
- 🔲 First-party plugins

## 6) Differentiators
- **AKS-aware UI:** node-pool grouping, production-context cues, Azure/workload identity hints, and add-on awareness.
- **Desktop-first depth:** advanced workflows are packaged directly into the Tauri experience.
- **Shared frontend for desktop packaging:** `apps/web` supplies the SvelteKit UI that ships inside the desktop app.
- **Safer ops:** masked secrets, production confirmations, audit logging, and explicit connection-state feedback.
- **No shady telemetry:** still intended as opt-in only.

## 7) Non-goals (v1)
- Full GitOps platform replacement.
- CI/CD pipelines.
- Full policy authoring suite.
- Deep multi-cloud integrations beyond “works with any cluster”.
- Multi-user server deployment.

## 8) Milestones

> See [ROADMAP.md](./ROADMAP.md) for the detailed breakdown.

| Milestone | Title | Status | Summary |
|-----------|-------|--------|---------|
| **M0** | Foundations | ✅ Complete | Shared repo/app architecture, CI, desktop shell, frontend packaging, and deterministic test fixtures |
| **M1** | Connect + Browse | ✅ Complete | Real cluster connectivity, context/namespace switching, watch-backed cache, overview dashboard, and broad built-in resource browsing |
| **M2** | Debug Loop + UX Foundation | ✅ Complete | Pod detail/logs/events/YAML plus search palette, shortcuts, theme toggle, breadcrumbs, and settings shell |
| **M3** | Full Debug Loop + Resource Actions | 🚧 Mostly implemented | Basic exec, basic port-forward, create/apply/delete/scale/restart flows, generic detail pages, nodes, and overview are present; richer depth is still in progress |
| **M4** | AKS-First Experience | 🚧 Partial | AKS heuristics, node-pool grouping, production guardrails, and Azure/workload hints exist; deep Azure auth/integration is still planned |
| **M5** | Helm + Metrics | 🚧 Mostly implemented | Helm list/detail/history/values/rollback plus pod/node metrics and basic trend views are shipped; upgrade/diff remains planned |
| **M6** | Search, CRDs + Advanced UX | 🚧 Partial | Search palette, CRD browsing, themes, settings, shortcuts, and breadcrumbs exist; richer filtering/sorting/favorites/workspace UX remains planned |
| **M7** | Desktop Hardening + Workflow Depth | 🚧 Partial | Interactive exec, port-forward management, audit coverage, and packaging polish remain major milestones |
| **M8** | Plugins v1 | 🔲 Planned | WASM/plugin host and extension ecosystem |

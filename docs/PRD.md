# Telescope — PRD

> Working title: **Telescope** (aka "Aurora" in earlier drafts)

## 1) Summary
Telescope is an open-source **desktop Kubernetes IDE** built with **Tauri + SvelteKit** for day-1/day-2 operator workflows. v1.0.0 ships as a **desktop-only** product with broad Kubernetes resource coverage, Lens-style troubleshooting flows, and a real **AKS Azure ARM management plane** for cluster and node-pool operations.

## 2) Goals / Success Criteria
### Product goals
- Deliver a polished **desktop operator experience** for cluster exploration, troubleshooting, and routine mutations.
- Reach strong **Lens-style parity** for core desktop workflows: cluster/context management, resource browsing, logs, exec, port-forward, YAML/apply/delete flows, Helm, and baseline metrics.
- Provide an **AKS-first** experience with real ARM-backed management-plane actions: node-pool CRUD, cluster start/stop, upgrade management, maintenance/diagnostic visibility, and Azure resource resolution.
- Stay **AKS-first without being AKS-only**: connect to any conformant Kubernetes cluster, with deeper Azure controls lighting up only when Telescope resolves an AKS resource.
- Maintain a materially lighter native footprint than Electron-class desktop apps.

### Success metrics (targets)
- **Time-to-first-cluster**: median < 3 minutes for kubeconfig-based setup.
- **Desktop memory** (idle, 1 cluster connected): < 250–350MB target (excluding OS webview).
- 90%+ completion of common day-2 ops without switching to kubectl for: logs, events, YAML/apply, delete, scale, rollout restart/status, Helm inspection/rollback, and baseline metrics.

## 3) Personas
- **AKS App Operator**: on-call and needs fast logs, events, exec, port-forward, and restart workflows.
- **Platform Engineer**: manages node pools, CRDs, upgrades, cluster-wide resources, and Helm-backed platforms.
- **Developer (namespace-scoped)**: wants a focused, low-noise desktop view for a few workloads.
- **Security/Compliance**: wants safe defaults, explicit confirmations, redaction, and auditability for destructive actions.

## 4) Key workflows (must be excellent)
1. Connect with kubeconfig, switch context/namespace quickly, and understand connection state immediately.
2. Browse built-in resources fast across **28+ watched Kubernetes resource types** plus discovered CRDs.
3. Debug failing workloads with logs, events, YAML inspection/editing, exec, port-forward, scaling, and rollout actions.
4. Manage Helm releases with list/detail/history/values/rollback workflows from the desktop app.
5. Operate AKS clusters through the Azure management plane: resolve the backing ARM resource, inspect maintenance/upgrade state, start/stop clusters, and manage node pools.
6. Deliver a responsive, auditable, desktop-native operator experience without browser/Hub deployment dependencies.

## 5) Lens Parity Checklist

> Status: ✅ = shipped in v1.0.0, 🚧 = intentionally partial in v1.0.0, 🔲 = future milestone

### Cluster Management
- ✅ Kubeconfig import/merge and context discovery
- ✅ Context list + switching with explicit connection-state handling
- ✅ Namespace switching
- ✅ Production-context detection and stronger confirmations in the UI
- 🔲 Context favorites / pinning

### Resource Explorer
- ✅ Desktop cache/watch coverage for **28+ built-in resource types**, including Pods, Deployments, StatefulSets, DaemonSets, ReplicaSets, Jobs, CronJobs, Services, Ingresses, NetworkPolicies, EndpointSlices, ConfigMaps, Secrets, PVCs, ResourceQuotas, LimitRanges, Events, ServiceAccounts, Roles, ClusterRoles, RoleBindings, ClusterRoleBindings, HPAs, PDBs, PriorityClasses, StorageClasses, PVs, Nodes, and admission webhooks
- ✅ 16 primary desktop resource blades plus generic resource detail routing
- ✅ Collapsible sidebar with grouped navigation
- ✅ Cluster overview dashboard
- ✅ Node list/detail with capacity, usage, conditions, and YAML
- ✅ CRD discovery and instance browsing
- ✅ Per-list text filtering and global search palette over cached resources
- 🚧 Rich label filtering and broader table sorting across every list

### Debug Loop
- ✅ Pod detail page (Summary / Logs / Exec / Events / YAML tabs)
- ✅ Log viewer: streaming/snapshot, container selector, search, previous logs, auto-scroll
- ✅ Events viewer with filtering
- 🚧 Exec terminal: reliable non-interactive exec is shipped; full interactive TTY/xterm.js remains future work
- 🚧 Port-forward: basic pod-focused desktop flow is shipped; richer active-session management remains future work

### Resource Actions
- ✅ Resource YAML viewing across built-in and dynamic resource detail flows
- ✅ Server-side apply / create-update workflows for supported built-in resources and CRD instances
- ✅ Delete flows for shipped resource detail pages and supported generic resources
- ✅ Scale actions for Deployments and StatefulSets
- ✅ Rollout restart/status actions for supported workloads
- ✅ Create resource from YAML/templates

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
- ✅ Basic trend charts / sparklines
- 🚧 Namespace-level rollups can still get deeper

### UX & Polish
- ✅ Dark theme
- ✅ Light theme + theme toggle
- ✅ Keyboard shortcuts + help overlay
- ✅ Breadcrumb navigation
- ✅ Search palette
- ✅ Settings / preferences surface
- ✅ Local audit logging for desktop operations
- 🔲 Tabbed workspace / hotbar

### AKS-Specific (our differentiator)
- ✅ ARM-backed AKS identity/resource resolution from the active cluster context
- ✅ Azure Portal deep links and ARM resource awareness
- ✅ Cluster start/stop controls
- ✅ Cluster upgrade profile visibility and control-plane upgrade management
- ✅ Node-pool listing, create/delete, scale, autoscaler updates, version upgrades, and node-image upgrades
- ✅ Maintenance configuration / diagnostics visibility from the ARM management plane
- ✅ Node-pool visibility in Kubernetes inventory views
- ✅ Multi-cloud baseline: Telescope works against any Kubernetes cluster, while AKS clusters unlock Azure-specific controls
- ✅ Production guardrails

### Extensions
- 🔲 Plugin / extension host
- 🔲 Plugin permissions model
- 🔲 First-party plugin extraction

## 6) Differentiators
- **Desktop-first operator focus:** Telescope is intentionally optimized for a native desktop workflow, not discontinued browser/Hub deployment modes.
- **AKS management-plane depth:** The app goes beyond kubeconfig-only browsing with ARM-backed cluster lifecycle, upgrade, and node-pool operations.
- **Broad resource coverage:** 28+ watched resource types, 16 primary blades, generic resource pages, and CRD discovery/instance flows ship in v1.0.0.
- **Safer ops:** masked secrets, confirmation UX, production-context warnings, and local audit logs reduce accidental damage.
- **Shared frontend packaged natively:** `apps/web` supplies the SvelteKit UI that ships inside the Tauri desktop shell.

## 7) Non-goals (v1)
- Browser/Hub deployment as a first-class product mode.
- Multi-user server deployment.
- Full GitOps platform replacement.
- CI/CD pipelines.
- Full policy authoring suite.
- Deep provider-specific management planes beyond the shipped AKS/Azure focus.

## 8) Milestones

> See [ROADMAP.md](./ROADMAP.md) for the post-v1.0.0 roadmap and future milestones.

| Milestone | Title | Status | Summary |
|-----------|-------|--------|---------|
| **M0** | Foundations | ✅ Complete | Monorepo architecture, Rust workspace, desktop shell, shared frontend packaging, CI, and deterministic fixtures |
| **M1** | Connect + Browse | ✅ Complete | Real cluster connectivity, context/namespace switching, watch-backed cache, overview dashboard, and broad built-in resource browsing |
| **M2** | Debug Loop + UX Foundation | ✅ Complete | Pod detail, logs, events, YAML, search palette, shortcuts, themes, breadcrumbs, and settings surface |
| **M3** | Resource Actions + Safe Ops | ✅ Complete | Create/apply/delete flows, scale/restart/status actions, pod-focused exec/port-forward, and operator guardrails |
| **M4** | AKS Visibility + Guardrails | ✅ Complete | AKS detection, node-pool visibility, Azure resource awareness, Portal links, and production-context safeguards |
| **M5** | Helm + Metrics | ✅ Complete | Helm list/detail/history/values/rollback plus pod/node metrics and trend views |
| **M6** | Search, CRDs + Advanced UX | ✅ Complete | Search palette, CRD discovery/instance browsing, generic detail routes, themes, filtering, shortcuts, and breadcrumbs |
| **M7** | Desktop Hardening + Auditability | ✅ Complete | Desktop packaging maturity, local audit logging, safer destructive operations, and operator-facing polish |
| **M8** | Resource Breadth + Operator Workflows | ✅ Complete | Expanded built-in resource coverage, generic actions, cluster-wide inventory depth, and desktop workflow consolidation |
| **M9** | Desktop Resource Expansion | ✅ Complete | 16 primary Kubernetes resource blades and 28+ watched resource types shipped in the desktop cache |
| **M10** | Azure ARM Management Plane | ✅ Complete | `telescope-azure` ARM client, AKS node-pool CRUD, cluster start/stop, upgrade management, and ARM-backed diagnostics |

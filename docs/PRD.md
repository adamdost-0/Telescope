# Telescope — PRD (Draft)

> Working title: **Telescope** (aka "Aurora" in earlier drafts)

## 1) Summary
Telescope is an open-source Kubernetes IDE that matches Lens' core day-1/day-2 workflows while being **AKS-optimized**, **low-memory**, and available as both a **desktop app** and a **web client**.

## 2) Goals / Success Criteria
### Product goals
- Feature parity with Lens for core operations: cluster/context mgmt, resource explorer, logs/exec/port-forward, Helm, basic metrics.
- AKS-first experience: Azure auth, node pools, add-ons visibility, AKS failure-mode hints.
- Memory efficiency: materially lower baseline/peak than Electron-class apps.
- Dual client: consistent Desktop + Web experience via shared core.

### Success metrics (targets)
- **Time-to-first-AKS-cluster**: median < 3 minutes.
- **Desktop memory** (idle, 1 cluster connected): < 250–350MB target (excluding OS webview).
- **Web client memory**: < 150MB in-browser target.
- 90%+ completion of common ops without switching to kubectl for: logs, exec, port-forward, Helm release ops, events, basic metrics.

## 3) Personas
- **AKS App Operator**: on-call, needs fast logs/exec/events/port-forward.
- **Platform Engineer**: manages node pools, CRDs, cluster-wide resources; needs Helm + safe operations.
- **Developer (namespace-scoped)**: wants a focused, low-noise view.
- **Security/Compliance**: wants clear permissions, auditability, safe defaults.

## 4) Key workflows (must be excellent)
1. Connect to AKS (Azure auth) + import kubeconfigs.
2. Explore resources quickly by kind/namespace, including CRDs.
3. Debug a failing workload: logs + exec + events + port-forward + rollout actions.
4. Helm day-2: list releases, upgrade/rollback with diff preview, values view/edit.
5. Metrics at a glance (metrics-server baseline; Prometheus optional).
6. Extensions/plugins with a permissions model.

## 5) Lens Parity Checklist

> Status: ✅ = shipped, 🔲 = planned (milestone noted)

### Cluster Management
- ✅ Kubeconfig import/merge (multi-file, `KUBECONFIG` env)
- ✅ Context list + switching with connection state machine
- ✅ Namespace switching (dropdown, remembers selection)
- 🔲 Context favorites / pinning — M6
- 🔲 Prod guardrails (visual warning for production contexts) — M4

### Resource Explorer
- ✅ Resource browsing: Pods, Deployments, StatefulSets, DaemonSets, Jobs, CronJobs, Services, Ingresses, ConfigMaps, Secrets, PVCs, Events (12 kinds)
- ✅ Collapsible sidebar with resource tree
- 🔲 Node list with capacity/allocatable/conditions — M3
- 🔲 CRD dynamic discovery and display — M6
- 🔲 Resource filter by name + labels — M6
- 🔲 Table column sorting — M6
- 🔲 Global resource search (Ctrl+K) — M6
- 🔲 Cluster overview dashboard — M3

### Debug Loop
- ✅ Pod detail page (Summary / Logs / Events / YAML tabs)
- ✅ Log viewer: streaming, container selector, search, previous logs, auto-scroll
- ✅ Events viewer with Warning/Normal filter
- 🔲 Exec terminal (xterm.js, multi-container) — M3
- 🔲 Port-forward with profiles + auto-reconnect — M3

### Resource Actions
- ✅ Resource YAML read-only viewer
- 🔲 YAML edit + apply with dry-run/diff — M3
- 🔲 Delete resource with confirmation — M3
- 🔲 Scale deployments/statefulsets — M3
- 🔲 Rollout restart/history/rollback — M3
- 🔲 Create resource from YAML — M3

### Helm
- 🔲 Release list/detail — M5
- 🔲 Values viewer/editor — M5
- 🔲 Upgrade with diff preview — M5
- 🔲 Rollback with revision history — M5

### Metrics
- 🔲 metrics-server discovery — M5
- 🔲 Pod/Node CPU + memory top — M5
- 🔲 Basic time-series charts — M5
- 🔲 Resource usage per namespace — M5

### UX & Polish
- ✅ Dark theme, loading skeletons, ARIA labels, focus styles
- 🔲 Light theme + theme toggle — M6
- 🔲 Keyboard shortcuts — M6
- 🔲 Tabbed workspace — M6
- 🔲 Hotbar / quick actions — M6
- 🔲 Breadcrumb navigation — M6
- 🔲 Settings / preferences page — M6

### AKS-Specific (Lens lacks these — our differentiator)
- 🔲 kubelogin exec credential plugin UX — M4
- 🔲 Azure Entra ID awareness (identity display, token refresh) — M4
- 🔲 Node pool visibility (grouping, VM size, autoscale) — M4
- 🔲 AKS add-on status display — M4
- 🔲 Azure Portal deep links — M4
- 🔲 Managed identity awareness — M4

### Extensions
- 🔲 WASM plugin host — M8
- 🔲 Plugin permissions manifest — M8
- 🔲 First-party plugins (Helm, AKS Tools, Prometheus) — M8

## 6) Differentiators
- **AKS-native**: device-code/browser auth, token refresh UX, nodepool/add-on awareness, Azure-linked hints.
- **Memory-first**: on-demand watchers, cache eviction, log/metrics backpressure.
- **Dual client**: desktop + web via shared core backend.
- **Safer ops**: read-only default, diff + dry-run, explicit destructive confirms.
- **No shady telemetry**: opt-in only.

## 7) Non-goals (v1)
- Full GitOps platform replacement.
- CI/CD pipelines.
- Full policy authoring suite.
- Deep multi-cloud integrations beyond "works with any cluster".

## 8) Milestones

> See [ROADMAP.md](./ROADMAP.md) for full feature breakdown per milestone.

| Milestone | Title | Status | Summary |
|-----------|-------|--------|---------|
| **M0** | Foundations | ✅ Complete | Repo, CI, Rust workspace, Tauri desktop shell, SvelteKit web scaffold, stub server |
| **M1** | Connect + Browse | ✅ Complete | Kubeconfig parsing, context/namespace switching, 12-kind resource browsing, SQLite cache, watch-driven sync |
| **M2** | Debug Loop + UX Foundation | ✅ Complete | Pod detail page, log viewer (streaming, search, container selector, previous logs), events viewer, YAML read-only |
| **M3** | Full Debug Loop + Resource Actions | 🔲 Next | Exec terminal, port-forward, YAML edit+apply, delete, scale, rollout, node list, cluster overview |
| **M4** | AKS-First Experience | 🔲 Planned | kubelogin UX, Entra ID awareness, node pools, add-ons, portal links, managed identity, prod guardrails |
| **M5** | Helm + Metrics | 🔲 Planned | Helm release lifecycle, metrics-server, CPU/memory top, sparkline charts |
| **M6** | Search, CRDs + Advanced UX | 🔲 Planned | Global search, CRD discovery, keyboard shortcuts, tabbed workspace, themes, settings |
| **M7** | Web Client + Hub Mode | 🔲 Planned | Hub container image, OIDC auth, per-user access, audit log |
| **M8** | Plugins v1 | 🔲 Planned | WASM plugin host, permissions, marketplace, first-party plugins |

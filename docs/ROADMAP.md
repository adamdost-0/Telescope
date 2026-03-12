# Telescope — Roadmap to Lens Parity

> **Goal:** 100% open-source replacement for Lens IDE with AKS-first focus.
> Track progress: each item is marked ✅ (shipped), 🚧 (in progress), or 🔲 (planned).

## Current Status: v0.3.0-alpha (~25–30% Lens parity)

Telescope today connects to real clusters, browses 12 resource kinds, streams pod
logs with search/filter, shows events, and persists state in SQLite. The desktop
app builds on Windows + macOS. A k3d integration test harness covers 54 Rust tests.
What's missing: exec terminal, port-forwarding, resource mutations, Helm, metrics,
CRDs, and the AKS-specific experience that differentiates us from Lens.

---

## ✅ M0 — Foundations (COMPLETE)

Scaffold phase — repo, CI, build pipeline, project skeleton.

| # | Item | Status |
|---|------|--------|
| 1 | Monorepo: Rust workspace (`core` / `engine` / `api` crates) | ✅ |
| 2 | Tauri v2 desktop shell (boots on Windows + macOS) | ✅ |
| 3 | SvelteKit 2 web scaffold with Svelte 5 runes | ✅ |
| 4 | CI: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` | ✅ |
| 5 | CI: Vitest unit tests + Playwright E2E with stub server | ✅ |
| 6 | CI: Desktop build matrix (Windows + macOS) | ✅ |
| 7 | Deterministic stub server (`tools/devtest/stub-server.mjs`) | ✅ |
| 8 | Repo governance: README, LICENSE, SECURITY.md, PR template | ✅ |

---

## ✅ M1 — Connect + Browse (COMPLETE)

Real cluster connectivity, kubeconfig management, resource browsing.

| # | Item | Status |
|---|------|--------|
| 1 | Kubeconfig parsing (multi-file, `KUBECONFIG` env merge) | ✅ |
| 2 | Context switching with connection state machine + backoff/retry | ✅ |
| 3 | Namespace switching (dropdown, remembers last selection) | ✅ |
| 4 | Resource browsing — 12 kinds: Pods, Deployments, StatefulSets, DaemonSets, Jobs, CronJobs, Services, Ingresses, ConfigMaps, Secrets, PVCs, Events | ✅ |
| 5 | Watch-driven sync: Kubernetes informer watches push to SQLite store | ✅ |
| 6 | SQLite-backed resource cache with on-demand hydration | ✅ |
| 7 | Collapsible left sidebar with resource tree (grouped by category) | ✅ |
| 8 | Dark theme, loading skeletons, ARIA labels, focus styles | ✅ |
| 9 | 54 Rust tests + k3d integration test harness | ✅ |

---

## ✅ M2 — Debug Loop + UX Foundation (COMPLETE)

Pod-level debugging: logs, events, detail pages.

| # | Item | Status |
|---|------|--------|
| 1 | Pod detail page with Summary / Logs / Events / YAML tabs | ✅ |
| 2 | Log viewer: real-time streaming via Kubernetes log API | ✅ |
| 3 | Log viewer: container selector (multi-container pods) | ✅ |
| 4 | Log viewer: text search within logs | ✅ |
| 5 | Log viewer: previous container logs toggle | ✅ |
| 6 | Log viewer: auto-scroll with manual override | ✅ |
| 7 | Events viewer with Warning / Normal severity filter | ✅ |
| 8 | Resource YAML read-only viewer (per-resource YAML tab) | ✅ |

---

## ✅ M3 — Full Debug Loop + Resource Actions

Complete the interactive debug loop: exec into containers, forward ports, and
perform write operations (edit, delete, scale, rollout). This is the milestone
that closes the gap between "viewer" and "IDE".

| # | Item | Detail | Lens parity? |
|---|------|--------|--------------|
| 1 | **Exec terminal** | Embed xterm.js terminal. Connect via WebSocket-over-SPDY exec channel to container shell (`/bin/sh` fallback to `/bin/bash`). Support multi-container pods with container picker. Handle resize (SIGWINCH). | ✅ Core Lens feature |
| 2 | **Port forwarding** | Forward local port → pod/service port. UI shows active forwards with status indicator. Support named profiles (e.g., "frontend:3000→80") saved per context. Auto-reconnect on disconnect with exponential backoff. | ✅ Core Lens feature |
| 3 | **Resource YAML edit + apply** | Inline YAML editor (CodeMirror / Monaco) with syntax highlighting. `kubectl apply` equivalent with `--dry-run=server` preview showing diff before apply. Validation errors shown inline. | ✅ Core Lens feature |
| 4 | **Delete resource** | Delete any browsable resource with confirmation dialog. Shows resource name, kind, namespace. Option for `--grace-period=0` force delete. Cascading delete warnings for owner references. | ✅ Core Lens feature |
| 5 | **Scale deployments** | Adjust replica count for Deployments, StatefulSets, ReplicaSets. Inline number input or +/- stepper on resource row. Shows current vs desired replicas during rollout. | ✅ Core Lens feature |
| 6 | **Rollout actions** | Restart: triggers `kubectl rollout restart`. History: show revision list with change-cause annotations. Rollback: revert to selected revision with confirmation. Status: show rollout progress (available/updated/ready counts). | ✅ Core Lens feature |
| 7 | **Create resource from YAML** | "New Resource" button opens blank YAML editor with kind/apiVersion template picker. Apply creates the resource with dry-run validation. | ✅ Core Lens feature |
| 8 | **Resource detail pages** | Extend pod detail pattern to Deployments, Services, StatefulSets, Jobs — each with Summary/Events/YAML tabs and kind-specific info (e.g., endpoints for Services, pod template for Deployments). | ✅ Core Lens feature |
| 9 | **Node list** | Show all cluster nodes with: status (Ready/NotReady), capacity vs allocatable (CPU, memory, pods), conditions, kernel/OS/container-runtime info, labels/taints. | ✅ Core Lens feature |
| 10 | **Cluster overview dashboard** | Landing page after connecting: cluster health (API server reachable, component statuses), resource counts by kind, recent warning events, node summary. | ✅ Core Lens feature |

---

## ✅ M4 — AKS-First Experience

Features that make Telescope the best IDE for Azure Kubernetes Service. **Lens has
none of these** — this is our primary differentiator.

| # | Item | Detail | Why AKS needs this |
|---|------|--------|--------------------|
| 1 | **kubelogin exec credential plugin** | Telescope detects when kubeconfig uses `exec`-based auth (standard for AKS v1.24+ with `kubelogin`). Provides inline UX for device-code flow: shows the one-time code and URL to visit, polls for completion, displays token refresh status in the connection banner. Supports all kubelogin modes: `devicecode`, `interactive` (browser popup), `spn` (service principal), `azurecli`, `msi`. | AKS clusters default to Entra ID auth via kubelogin. Lens shows a cryptic exec error; Telescope guides the user through it. |
| 2 | **Azure Entra ID (Azure AD) awareness** | Display the signed-in Azure identity (UPN, tenant, object ID) in the connection panel. Show token expiry countdown and one-click re-auth. Detect AAD-enabled clusters and surface helpful context (e.g., "This cluster uses Azure RBAC — your permissions come from Azure role assignments, not Kubernetes RBAC"). | AKS operators need to know *who* they're authenticated as and *why* they can/can't do something. Lens is identity-blind. |
| 3 | **Node pool visibility** | Parse AKS node pool labels (`agentpool`, `kubernetes.azure.com/agentpool`, `node.kubernetes.io/instance-type`). Group nodes by pool in the node list. Show pool-level info: VM size, node count, min/max autoscale, OS type (Linux/Windows), mode (System/User). | AKS users think in node pools, not individual nodes. Lens shows a flat node list with no pool grouping. |
| 4 | **AKS add-on status** | Query the Azure Resource Manager API (or parse well-known labels/pods) to show status of AKS add-ons: monitoring (Container Insights), policy (Azure Policy / Gatekeeper), KEDA, Key Vault CSI, GitOps (Flux), ingress controller. Show enabled/disabled + health. | Operators troubleshoot add-on issues frequently. Today they must switch to Azure Portal. Telescope surfaces this alongside cluster resources. |
| 5 | **Azure Portal deep links** | For every AKS-detected resource, show an "Open in Azure Portal" link that navigates directly to the corresponding blade (cluster overview, node pool, monitoring, etc.). Construct links from subscription ID + resource group + cluster name parsed from kubeconfig or Azure metadata. | Reduces portal round-trips. When Telescope can't do something (e.g., change node pool VM size), it links directly to the Portal blade that can. |
| 6 | **Managed identity awareness** | Detect pods using Azure Workload Identity (federated credentials) or AAD Pod Identity (legacy). Show the identity binding (service account → managed identity) on the pod detail page. Surface token mount paths and expiry. | Workload identity issues are a top AKS support topic. Lens has zero visibility into Azure identity bindings. |
| 7 | **Prod guardrails** | Visual safety indicators for production contexts: red banner, confirmation dialogs for destructive ops, optional read-only mode toggle. Contexts are tagged as "production" via name pattern matching (configurable regex) or explicit user marking. | Prevents accidental `kubectl delete` in prod. Lens has no production-awareness — all contexts look the same. |

---

## ✅ M5 — Helm + Metrics

Helm release lifecycle management and metrics-server integration for resource
monitoring.

| # | Item | Detail | Lens parity? |
|---|------|--------|--------------|
| 1 | **Helm release list** | List all Helm releases across namespaces. Show: name, namespace, chart, app version, revision, status (deployed/failed/pending), updated timestamp. Filter by namespace and status. | ✅ Core Lens feature |
| 2 | **Helm release detail** | Per-release view: current values (computed), chart metadata, release notes, manifest resources list (clickable to resource detail). | ✅ Core Lens feature |
| 3 | **Helm values viewer/editor** | Read-only computed values view. Editable user-supplied values with YAML editor. Diff view showing user overrides vs chart defaults. | ✅ Core Lens feature |
| 4 | **Helm upgrade** | Upgrade release with new values. Dry-run preview showing diff of what will change. Version selector for chart upgrades. | ✅ Core Lens feature |
| 5 | **Helm rollback** | Rollback to previous revision with confirmation. Show revision history with change summaries. Diff between current and target revision. | ✅ Core Lens feature |
| 6 | **Helm release history** | Full revision timeline: revision number, status, chart version, updated time, description/change-cause. | ✅ Core Lens feature |
| 7 | **metrics-server discovery** | Auto-detect metrics-server availability via `metrics.k8s.io` API group. Graceful degradation: show "Metrics unavailable — install metrics-server" when absent. | ✅ Core Lens feature |
| 8 | **Pod CPU + memory top** | Real-time CPU (millicores) and memory (MiB) usage per pod. Sortable table. Percentage of request/limit usage with color coding (green/yellow/red). | ✅ Core Lens feature |
| 9 | **Node CPU + memory top** | Per-node resource usage: CPU, memory, pod count. Percentage of allocatable. Sortable. | ✅ Core Lens feature |
| 10 | **Basic time-series charts** | Mini sparkline charts (last 15 min) for pod and node CPU/memory. Lightweight — poll metrics API every 30s, store in-memory ring buffer. No Prometheus dependency. | ✅ Core Lens feature |
| 11 | **Resource usage per namespace** | Aggregate CPU/memory usage by namespace. Bar chart or table view. Useful for cost attribution at a glance. | ✅ Core Lens feature |

---

## ✅ M6 — Search, CRDs + Advanced UX

Power-user features: global search, CRD support, keyboard-driven workflows,
and visual polish.

| # | Item | Detail | Lens parity? |
|---|------|--------|--------------|
| 1 | **Global resource search (Ctrl+K)** | Command palette / fuzzy search across all resource kinds and names. Type-ahead with kind icons. Navigate to any resource in 2 keystrokes. | ✅ Core Lens feature |
| 2 | **Resource filter by name + labels** | Filter bar on every resource list: text search on name, label selector (key=value, key!=value, key in (v1,v2)). Filters persist in URL for shareability. | ✅ Core Lens feature |
| 3 | **Table column sorting** | Click column headers to sort any resource table (name, age, status, restarts, CPU, memory). Tri-state: ascending → descending → default. | ✅ Core Lens feature |
| 4 | **CRD dynamic discovery** | Discover all CRDs in the cluster via `apiextensions.k8s.io`. Auto-generate list/detail views for any CRD. Show CRD schema (if available) in detail view. | ✅ Core Lens feature |
| 5 | **Keyboard shortcuts** | `Ctrl+K` search, `Ctrl+L` logs, `Ctrl+Shift+T` exec, `Ctrl+P` port-forward, `Ctrl+,` settings. Shortcut help overlay (`?`). All shortcuts user-customizable. | ✅ Core Lens feature |
| 6 | **Tabbed workspace** | Open multiple resources simultaneously in tabs (like browser tabs). Tab bar with resource kind icon + name. Close/reorder tabs. Persist tab state across sessions. | ✅ Core Lens feature |
| 7 | **Hotbar / quick actions** | Configurable quick-launch bar (bottom or side). Pin favorite resources, contexts, or actions. Drag-and-drop reorder. Keyboard number shortcuts (Alt+1..9). | ✅ Core Lens feature |
| 8 | **Light theme + theme toggle** | Light color scheme with accessible contrast ratios. Toggle in header bar. Persist preference. System-auto option (follow OS dark/light). | ✅ Core Lens feature |
| 9 | **Breadcrumb navigation** | Path breadcrumbs showing: Context → Namespace → Kind → Resource name. Each segment clickable for navigation. | ✅ Core Lens feature |
| 10 | **Context favorites / pinning** | Star/pin frequently used contexts to the top of the context list. Persist across sessions. | ✅ Core Lens feature |
| 11 | **Settings / preferences page** | Central settings UI: theme, kubeconfig paths, default namespace, keyboard shortcuts, prod guardrail patterns, telemetry opt-in. | ✅ Core Lens feature |

---

## 🔲 M7 — Web Client + Hub Mode

Multi-user web deployment: headless engine as a container, OIDC auth, access
controls, and audit logging.

| # | Item | Detail |
|---|------|--------|
| 1 | **Hub service** | Package `telescope-engine` as a headless container image (no desktop UI). Exposes HTTP/WebSocket API. Connects to clusters on behalf of authenticated users. Helm chart for deployment. |
| 2 | **OIDC authentication** | Web client authenticates via OIDC (Azure Entra ID, Okta, Dex). JWT validation in hub. Session management with refresh tokens. |
| 3 | **Per-user access controls** | Map OIDC claims to Kubernetes impersonation (`--as` user, `--as-group` groups). Users see only what their K8s RBAC allows. Admin can restrict which clusters are visible per user/group. |
| 4 | **Audit log** | Log all user actions (resource views, exec sessions, delete operations) with timestamp, user identity, action, resource, and outcome. Structured JSON log format. Configurable retention. |
| 5 | **Web client parity** | SvelteKit web client feature-complete with desktop (minus native OS integrations like system tray, file dialogs). Responsive layout for different screen sizes. |

---

## 🔲 M8 — Plugins v1

Extensibility system for custom resource views, actions, and integrations.

| # | Item | Detail |
|---|------|--------|
| 1 | **WASM plugin host** | Plugins compiled to WASM, loaded at runtime. Sandboxed execution with defined API surface (read resources, register menu items, add detail tabs). |
| 2 | **Plugin permissions manifest** | Each plugin declares required permissions (resource kinds it reads/writes, network access, UI surface areas). User approves on install. Principle of least privilege. |
| 3 | **Plugin marketplace / registry** | Browse and install plugins from a registry (initially just a curated list in a GitHub repo). Version management and update notifications. |
| 4 | **First-party plugin: Helm** | Extract Helm UI from core into a plugin — proves the plugin API is sufficient for real workflows. |
| 5 | **First-party plugin: AKS Tools** | AKS-specific features (node pools, add-ons, portal links) as a plugin — allows non-AKS users to skip the weight. |
| 6 | **First-party plugin: Prometheus** | Prometheus metrics integration: PromQL queries, custom dashboards, alert rule viewer. Replaces metrics-server charts when Prometheus is available. |

---

## Parity Summary

| Milestone | Scope | Cumulative Lens Parity | Status |
|-----------|-------|------------------------|--------|
| M0–M1 | Connect + Browse | ~15% — read-only resource listing | ✅ |
| M2 | Debug Loop | ~30% — add logs, events, detail views | ✅ |
| M3 | Resource Actions | ~50% — exec, port-forward, YAML edit, delete, scale | ✅ |
| M4 | AKS-First | ~55% — unique differentiator, not Lens parity | ✅ |
| M5 | Helm + Metrics | ~70% — major Lens workflow complete | ✅ |
| M6 | Search + CRDs + UX | ~80% — polish and power-user features | ✅ |
| M7 | Web + Hub | ~90% — multi-user deployment | 🔲 |
| M8 | Extensions | ~95% — extensibility | 🔲 |

---

## v1.0 Readiness Checklist

- [ ] All features validated against live AKS cluster
- [ ] E2E test coverage for core workflows
- [ ] SQLite schema migration system
- [ ] Landing page and documentation
- [ ] Community channel (Discord/Slack)
- [ ] 30-day dogfooding period

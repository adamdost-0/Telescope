# Telescope — UX Notes

> **Status: Current implementation snapshot** — The route inventory below reflects the actual `apps/web/src/routes/` tree today. Desktop/Tauri exposes the broadest feature set; browser/Hub mode reuses the same UI but still has partial parity for several write operations.

## Navigation
- Left rail: Overview • Pods • Deployments • StatefulSets • DaemonSets • Jobs • CronJobs • Services • Ingresses • ConfigMaps • Secrets • PVCs • Nodes • Events • Helm • CRDs • Create • Settings
- Top bar: context switcher, namespace selector, connection status, production indicator, theme toggle, search/help shortcuts
- Main area: dashboard or list/detail pages; pod pages use tabbed detail views, generic resource pages reuse a shared detail shell

## Current routes

| Route | Purpose |
|------|---------|
| `/` | Landing page with cluster list and connect flow; selects a context, loads namespaces, then routes into the app. |
| `/clusters` | Dedicated cluster/context picker with auth-type badges, server URL, connection state, and reconnect flow. |
| `/overview` | Cluster dashboard with version/auth summary, AKS badge/add-on hints, resource counts, pod phase summary, namespace usage, recent warning events, and Azure Portal action. |
| `/create` | Create-from-template page for common resources (for example Pod, Deployment, Service, ConfigMap, Secret) with dry-run/apply flow. |
| `/events` | Cluster-wide events view with namespace/type/text filtering. |
| `/pods` | Namespace-scoped pod list with filters and pod metrics. |
| `/pods/[namespace]/[name]` | Pod detail page with Summary, Logs, Exec, Events, and YAML tabs plus delete and port-forward actions. |
| `/nodes` | Node inventory with readiness, capacity/allocatable, usage, and AKS node-pool grouping when labels are present. |
| `/nodes/[name]` | Node detail page with Summary, Conditions, Capacity, Metrics, and YAML tabs. |
| `/helm` | Helm release list across namespaces with status, revision, chart, app version, and updated time. |
| `/helm/[namespace]/[name]` | Helm release detail with Info, Values, and History tabs plus rollback and CLI upgrade guidance. |
| `/crds` | CRD definition browser with filter by group/name/kind. |
| `/crds/[group]/[kind]` | CRD instance browser for the selected CRD/group/version/scope/plural query. |
| `/settings` | Settings page for theme, default namespace, auto-refresh interval, production patterns, and app metadata. |
| `/resources/deployments` | Deployment list with readiness, updated/available counts, and image summary. |
| `/resources/statefulsets` | StatefulSet list with service name, pod management policy, and image summary. |
| `/resources/daemonsets` | DaemonSet list with current/ready/updated/available counts and rollout visibility. |
| `/resources/jobs` | Job list with status, completions, active pods, and duration. |
| `/resources/cronjobs` | CronJob list with schedule, suspend state, active runs, and last scheduled time. |
| `/resources/services` | Service list with type, cluster IP, and port summary. |
| `/resources/ingresses` | Ingress list with class, hosts, address, and TLS summary. |
| `/resources/configmaps` | ConfigMap list with key counts and key names. |
| `/resources/secrets` | Secret list fetched on-demand (not from the shared cache) with type and key-count summary. |
| `/resources/pvcs` | PersistentVolumeClaim list with status, bound volume, capacity/request, access modes, and storage class. |
| `/resources/[kind]/[namespace]/[name]` | Shared detail page for built-in resources such as Deployments, Services, StatefulSets, DaemonSets, Jobs, CronJobs, ConfigMaps, Secrets, Ingresses, and PVCs; includes Summary, Events, YAML, and Pods tabs where applicable, plus scale/restart actions for supported workloads. |

## Must-have screens
- **Cluster list + quick health:** Implemented via `/` and `/clusters`, with `/overview` as the connected landing dashboard.
- **Cluster overview:** Implemented and currently focused on quick health/resource summaries rather than a full Lens-style telemetry dashboard.
- **Resource explorer:** Implemented for the major built-in workload/network/config kinds plus nodes, events, Helm releases, and CRDs.
- **Workload detail tabs:** Pods have Summary • Logs • Exec • Events • YAML. Other built-in resources share Summary • Events • YAML, and workload kinds add Pods plus scale/restart actions.
- **Logs viewer:** Implemented with streaming/snapshot log fetch, search, previous logs toggle, auto-scroll, and container selector.
- **Exec:** Present today as non-interactive command execution from the pod detail page; a full interactive terminal remains future work.
- **Helm:** Release list/detail/history/values/rollback are implemented; in-app upgrade/diff workflows are still future work.
- **Search and settings:** Command palette/search, keyboard shortcuts, help overlay, theme toggle, breadcrumbs, and a settings page are all part of the current UI.

## Performance rules
- Virtualize or aggressively filter any list that can grow large.
- Prefer watch-driven or cached data over blind polling; keep metrics polling bounded and lazy.
- Lazy-load expensive views per page/tab, and keep logs/metrics history bounded in memory.

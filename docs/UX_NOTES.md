# Telescope — UX Notes

> **Status: v1.0.0** — The route inventory below reflects the actual `apps/web/src/routes/` tree. The Tauri desktop app is the only supported runtime. 39 routes, 25 components.

## Navigation
- Left rail: Overview • Pods • Deployments • StatefulSets • DaemonSets • Jobs • CronJobs • Services • Ingresses • ConfigMaps • Secrets • PVCs • PVs • Storage Classes • Nodes • Events • Helm • CRDs • Namespaces • Azure Node Pools • Create • Settings
- Top bar: context switcher, namespace selector, connection status, production indicator, theme toggle, search/help shortcuts
- Main area: dashboard or list/detail pages; pod pages use tabbed detail views, generic resource pages reuse a shared detail shell

## Current routes (39 pages)

| Route | Purpose |
|------|---------|
| `/` | Landing page with cluster list and connect flow; selects a context, loads namespaces, then routes into the app. |
| `/overview` | Cluster dashboard with version/auth summary, AKS badge/add-on hints, resource counts, pod phase summary, namespace usage, recent warning events, and Azure Portal action. |
| `/create` | Create-from-template page for common resources (Pod, Deployment, Service, ConfigMap, Secret) with dry-run/apply flow. |
| `/events` | Cluster-wide events view with namespace/type/text filtering. |
| `/namespaces` | Namespace management page with create/delete. |
| `/pods` | Namespace-scoped pod list with filters and pod metrics. |
| `/pods/[namespace]/[name]` | Pod detail page with Summary, Logs, Exec, Events, and YAML tabs plus delete and port-forward actions. |
| `/nodes` | Node inventory with readiness, capacity/allocatable, usage, AKS node-pool grouping, cordon/uncordon/drain/taint actions. |
| `/nodes/[name]` | Node detail page with Summary, Conditions, Capacity, Metrics, and YAML tabs. |
| `/helm` | Helm release list across namespaces with status, revision, chart, app version, and updated time. |
| `/helm/[namespace]/[name]` | Helm release detail with Info, Values, and History tabs plus rollback and CLI upgrade guidance. |
| `/crds` | CRD definition browser with filter by group/name/kind. |
| `/crds/[group]/[kind]` | CRD instance browser for the selected CRD/group/version/scope/plural query. |
| `/settings` | Settings page for theme, default namespace, auto-refresh interval, production patterns, Azure cloud, and app metadata. |
| `/azure/node-pools` | Azure ARM node pool management: list pools, scale, create, delete, upgrade version/image, autoscaler config. |
| `/resources/deployments` | Deployment list with readiness, updated/available counts, and image summary. |
| `/resources/statefulsets` | StatefulSet list with service name, pod management policy, and image summary. |
| `/resources/daemonsets` | DaemonSet list with current/ready/updated/available counts and rollout visibility. |
| `/resources/jobs` | Job list with status, completions, active pods, and duration. |
| `/resources/cronjobs` | CronJob list with schedule, suspend state, active runs, and last scheduled time. |
| `/resources/services` | Service list with type, cluster IP, and port summary. |
| `/resources/ingresses` | Ingress list with class, hosts, address, and TLS summary. |
| `/resources/configmaps` | ConfigMap list with key counts and key names. |
| `/resources/secrets` | Secret list fetched on-demand (not from shared cache) with type and key-count summary. |
| `/resources/pvcs` | PersistentVolumeClaim list with status, bound volume, capacity/request, access modes, and storage class. |
| `/resources/persistentvolumes` | PersistentVolume list with capacity, access modes, reclaim policy, status, and storage class. |
| `/resources/storageclasses` | StorageClass list with provisioner, reclaim policy, volume binding mode, and default annotation. |
| `/resources/hpas` | HorizontalPodAutoscaler list with target, min/max replicas, current replicas, and metrics. |
| `/resources/networkpolicies` | NetworkPolicy list with pod selector, ingress/egress rules. |
| `/resources/poddisruptionbudgets` | PodDisruptionBudget list with min available, max unavailable, allowed disruptions. |
| `/resources/priorityclasses` | PriorityClass list with value, global default, and preemption policy. |
| `/resources/resourcequotas` | ResourceQuota list with hard/used limits. |
| `/resources/limitranges` | LimitRange list with type, default, and max limits. |
| `/resources/endpointslices` | EndpointSlice list with address type, ports, and endpoints. |
| `/resources/serviceaccounts` | ServiceAccount list with secrets and automount status. |
| `/resources/roles` | Role list with rules summary. |
| `/resources/rolebindings` | RoleBinding list with role ref and subjects. |
| `/resources/webhooks` | ValidatingWebhookConfiguration / MutatingWebhookConfiguration list. |
| `/resources/[kind]/[namespace]/[name]` | Shared detail page for built-in resources; includes Summary, Events, YAML, and Pods tabs where applicable, plus scale/restart actions for supported workloads. |

## Components (25)

`AppHeader` · `AzureIdentitySection` · `Breadcrumbs` · `ClusterVitals` · `ConfirmDialog` · `ConnectionStatus` · `ContextSwitcher` · `ErrorMessage` · `EventsTable` · `ExecTerminal` · `FilterBar` · `LoadingSkeleton` · `LogViewer` · `NodePoolHeader` · `PodTable` · `PortForwardDialog` · `ResourceTable` · `ScaleDialog` · `SearchPalette` · `ShortcutHelp` · `Sidebar` · `Sparkline` · `Tabs` · `ThemeToggle` · `YamlEditor`

## Must-have screens
- **Cluster list + quick health:** Implemented via `/` with `/overview` as the connected landing dashboard.
- **Cluster overview:** Quick health/resource summaries, AKS add-on detection, Azure Portal deep links.
- **Resource explorer:** Implemented for 27 built-in resource kinds plus nodes, events, Helm releases, CRDs, and namespaces.
- **Azure ARM management:** Node pool list/scale/create/delete/upgrade via `/azure/node-pools`.
- **Workload detail tabs:** Pods have Summary • Logs • Exec • Events • YAML. Other built-in resources share Summary • Events • YAML, and workload kinds add Pods plus scale/restart actions.
- **Logs viewer:** Streaming/snapshot log fetch, search, previous logs toggle, auto-scroll, and container selector.
- **Exec:** Non-interactive command execution from the pod detail page; full interactive terminal remains future work.
- **Helm:** Release list/detail/history/values/rollback; in-app upgrade/diff workflows are future work.
- **Search and settings:** Command palette/search, keyboard shortcuts, help overlay, theme toggle, breadcrumbs, and settings page.

## Performance rules
- Virtualize or aggressively filter any list that can grow large.
- Prefer watch-driven or cached data over blind polling; keep metrics polling bounded and lazy.
- Lazy-load expensive views per page/tab, and keep logs/metrics history bounded in memory.

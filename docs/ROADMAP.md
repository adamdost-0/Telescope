# Telescope — Roadmap to Lens Parity

> **Goal:** deliver an open-source Kubernetes IDE with strong desktop ergonomics, growing browser/Hub support, and AKS-aware workflows.
> Track progress with ✅ (shipped), 🚧 (partially shipped / substantial alpha), and 🔲 (planned).

## Current Status: substantial alpha (~65–70% Lens parity in desktop/Tauri, lower in browser/Hub mode)

Telescope today goes well beyond the early read-only v0.0.1 shape. The current codebase supports real cluster connection and context switching, namespace management, a cluster overview dashboard, node inventory/detail pages, broad built-in resource browsing, pod logs, non-interactive exec, basic pod port-forwarding, create/apply/delete/scale/restart flows, Helm release list/detail/history/values/rollback, CRD definition browsing plus instance listing, metrics-server integration, a search palette, theme/settings UI, and JSONL audit logging in Hub mode.

The main remaining gap is **feature parity between desktop/Tauri and browser/Hub mode**. Many write flows and richer interactions still exist only through the desktop/Tauri API path or are intentionally basic in the current implementation.

---

## ✅ M0 — Foundations (COMPLETE)

| # | Item | Status |
|---|------|--------|
| 1 | Monorepo: Rust workspace (`core` / `engine` / `api`) plus desktop, web, and hub apps | ✅ |
| 2 | Tauri desktop shell packaging the shared `apps/web` frontend | ✅ |
| 3 | SvelteKit web client with shared UI/routes | ✅ |
| 4 | CI for Rust, web tests/E2E, and desktop builds | ✅ |
| 5 | Deterministic stubbed browser test data | ✅ |

---

## ✅ M1 — Connect + Browse (COMPLETE)

| # | Item | Status |
|---|------|--------|
| 1 | Kubeconfig parsing (including multi-context discovery) | ✅ |
| 2 | Context switching with explicit connection-state transitions | ✅ |
| 3 | Namespace listing/selection | ✅ |
| 4 | Watch-driven sync into SQLite-backed cache/store | ✅ |
| 5 | Broad built-in resource browsing (Pods, Deployments, StatefulSets, DaemonSets, Jobs, CronJobs, Services, Ingresses, ConfigMaps, Secrets, PVCs, Events, Nodes) | ✅ |
| 6 | Collapsible resource sidebar and cluster overview landing page | ✅ |

---

## ✅ M2 — Debug Loop + UX Foundation (COMPLETE)

| # | Item | Status |
|---|------|--------|
| 1 | Pod detail page with Summary / Logs / Exec / Events / YAML tabs | ✅ |
| 2 | Pod logs snapshot + streaming, previous logs, search, auto-scroll, container selector | ✅ |
| 3 | Events browsing/filtering | ✅ |
| 4 | Read-only and editable YAML views for supported resources | ✅ |
| 5 | Search palette, keyboard shortcuts/help, breadcrumbs, theme toggle, and settings page shell | ✅ |

---

## 🚧 M3 — Full Debug Loop + Resource Actions (MOSTLY IMPLEMENTED)

| # | Item | Current state | Status |
|---|------|---------------|--------|
| 1 | Exec terminal | Non-interactive command execution exists from the pod detail page; full interactive TTY/xterm experience is still planned. | 🚧 |
| 2 | Port forwarding | Pod port-forward dialog and engine forwarder exist; active-forward management, saved profiles, and browser/Hub parity are still missing. | 🚧 |
| 3 | YAML edit + apply | Dry-run/apply exists for supported namespaced resource kinds. Coverage is real, but not universal across every kind and not fully wired through Hub mode. | 🚧 |
| 4 | Delete resource | Engine delete support exists for many built-in kinds and the pod detail page exposes delete UI. Generic delete coverage is still incomplete. | 🚧 |
| 5 | Scale workloads | Deployment/StatefulSet scaling is implemented in engine and UI. | ✅ |
| 6 | Rollout actions | Rollout restart/status exists for Deployment/StatefulSet/DaemonSet workloads. Full history/rollback UX for workloads is still planned. | 🚧 |
| 7 | Create resource from YAML/templates | Implemented via `/create` plus apply support. | ✅ |
| 8 | Resource detail pages | Generic detail pages exist for the core built-in kinds, with workload-specific pods/scale/restart affordances. | ✅ |
| 9 | Node list/detail | Implemented, including metrics and capacity/condition views. | ✅ |
| 10 | Cluster overview dashboard | Implemented. | ✅ |

---

## 🚧 M4 — AKS-First Experience (PARTIALLY IMPLEMENTED)

| # | Item | Current state | Status |
|---|------|---------------|--------|
| 1 | kubelogin / exec-auth UX | Auth-type hints and error suggestions exist, but there is no full interactive Azure login flow. | 🚧 |
| 2 | Azure identity awareness | Pod/workload identity hints exist; signed-in Azure identity and token lifecycle UX do not. | 🚧 |
| 3 | Node pool visibility | AKS node-pool grouping/labels are surfaced in the node views. | ✅ |
| 4 | AKS add-on status | Overview/add-on hints exist via cluster heuristics, not deep Azure integration. | 🚧 |
| 5 | Azure Portal deep links | Overview can surface a Portal action, but metadata extraction is still incomplete in common cases. | 🚧 |
| 6 | Managed identity awareness | Workload identity clues are surfaced in pod detail, but this remains heuristic. | 🚧 |
| 7 | Production guardrails | Production-context bannering and stronger confirmations exist in the UI. | ✅ |

---

## 🚧 M5 — Helm + Metrics (SUBSTANTIALLY IMPLEMENTED)

| # | Item | Current state | Status |
|---|------|---------------|--------|
| 1 | Helm release list | Implemented. | ✅ |
| 2 | Helm release detail | Implemented with info, values, and history tabs. | ✅ |
| 3 | Helm values handling | Values extraction and sensitive-value redaction are implemented. | ✅ |
| 4 | Helm rollback | Implemented. | ✅ |
| 5 | Helm upgrade / diff preview | Still planned; current UI provides rollback and upgrade CLI guidance rather than in-app upgrade workflows. | 🔲 |
| 6 | metrics-server discovery | Implemented. | ✅ |
| 7 | Pod CPU + memory usage | Implemented. | ✅ |
| 8 | Node CPU + memory usage | Implemented. | ✅ |
| 9 | Lightweight trend charts | Implemented as sparklines/basic history in the UI. | ✅ |
| 10 | Namespace usage summary | Overview surfaces namespace usage summaries, but not yet a full Lens-style analytics surface. | 🚧 |

---

## 🚧 M6 — Search, CRDs + Advanced UX (PARTIALLY IMPLEMENTED)

| # | Item | Current state | Status |
|---|------|---------------|--------|
| 1 | Global resource search | Ctrl/Cmd+K search palette exists over cached resources. | ✅ |
| 2 | Resource filtering | Text filtering exists across major lists; richer label/filter/query support is still planned. | 🚧 |
| 3 | Table sorting | Not broadly implemented yet. | 🔲 |
| 4 | CRD discovery and browsing | CRD definitions and instance browsing exist; fully generic CRD detail flows are still incomplete. | 🚧 |
| 5 | Keyboard shortcuts / help | Implemented. | ✅ |
| 6 | Theme support | Dark/light theme support and toggle exist. | ✅ |
| 7 | Breadcrumb navigation | Implemented. | ✅ |
| 8 | Settings / preferences | Settings page exists, but browser/Hub preference persistence is still partial. | 🚧 |
| 9 | Context favorites / pinning | Planned. | 🔲 |
| 10 | Tabbed workspace / hotbar | Planned. | 🔲 |

---

## 🚧 M7 — Web Client + Hub Mode (PARTIALLY IMPLEMENTED)

| # | Item | Current state | Status |
|---|------|---------------|--------|
| 1 | Hub service | Axum-based Hub with REST API, SQLite-backed cache, watcher startup, and Dockerfile exists. | ✅ |
| 2 | Browser/web mode | Shared web UI can talk to Hub for core read flows, but many write flows are still deferred. | 🚧 |
| 3 | OIDC authentication | Auth routes and middleware exist, but login/callback are placeholders and JWTs are not cryptographically validated yet. | 🚧 |
| 4 | Access controls | Basic cluster-access hook and user impersonation path exist; real policy/authorization is still planned. | 🚧 |
| 5 | Audit logging | JSONL audit log plus `/api/v1/audit` endpoint exist, with partial action coverage. | 🚧 |
| 6 | WebSockets / live multi-user workflows | WebSocket support is still skeletal. | 🔲 |

---

## 🔲 M8 — Plugins v1

Still planned:
- WASM plugin host
- Plugin permissions manifest
- Marketplace/registry story
- First-party extension extraction (for example Helm / AKS tools)

---

## Parity Summary

| Milestone | Scope | Approx. parity contribution | Status |
|-----------|-------|-----------------------------|--------|
| M0–M1 | Foundations + connect/browse | ~25% | ✅ |
| M2 | Debug loop basics + shared UX | ~40% | ✅ |
| M3 | Resource actions + richer detail pages | ~55% | 🚧 |
| M4 | AKS-aware workflows | Differentiator more than parity; partial today | 🚧 |
| M5 | Helm + metrics | ~65–70% in desktop/Tauri mode | 🚧 |
| M6 | Search + CRDs + advanced UX | pushes polish and breadth further, still partial | 🚧 |
| M7 | Browser/Hub parity + auth/audit | major remaining gap | 🚧 |
| M8 | Plugins | future extensibility layer | 🔲 |

---

## v1.0 Readiness Checklist

- [ ] Browser/Hub write-path parity for core workflows
- [ ] Production-grade OIDC/JWT validation and real authorization model
- [ ] Interactive exec and more complete port-forward UX
- [ ] In-app Helm upgrade/diff support
- [ ] Broader generic CRD detail/edit experience
- [ ] More complete E2E coverage for real user workflows

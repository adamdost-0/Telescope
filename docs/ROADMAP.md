---
title: Roadmap
nav_order: 8
description: "Post-v1.0.0 milestones and priorities"
---

# Telescope — Post-v1.0.0 Roadmap

> **Goal:** keep extending the shipped desktop Kubernetes IDE with deeper operator workflows while preserving the native Tauri-first experience.
> Historical milestones are marked as **Closed** or **Complete** now that v1.0.0 has shipped.

## Current Status: v1.2.0 SHIPPED

Telescope v1.2.0 is now the shipped baseline: a **desktop-only** Kubernetes IDE with broad built-in resource coverage, CRD workflows, Helm + metrics, local audit logging, a real **Azure ARM management plane** for AKS lifecycle and node operations, and **AI Insights** powered by BYOK Azure OpenAI.

### What v1.0.0 delivered
- Desktop-only Tauri packaging with the SvelteKit frontend bundled for native distribution
- **28+ watched Kubernetes resource types** and **16 primary desktop resource blades**
- Real cluster connection, context switching, namespace management, and watch-backed desktop caching
- Logs, events, YAML inspection/edit/apply flows, delete/scale/restart actions, and basic exec + port-forward workflows
- Helm release list/detail/history/values/rollback and metrics-server-backed pod/node metrics
- CRD discovery, instance browsing, and desktop CRUD-oriented apply/delete flows
- Search palette, themes, keyboard shortcuts, settings surface, and local audit logging
- **Azure ARM management-plane support**: AKS identity/resource resolution, Portal links, node-pool CRUD, autoscaler updates, cluster start/stop, upgrade management, and maintenance/diagnostic visibility
- A desktop-first operator workflow that no longer targets discontinued browser/Hub delivery modes
- **AI Insights**: BYOK Azure OpenAI integration with dual auth (Entra ID / API Key), multi-cloud support, allowlist-only context builder, structured JSON responses, per-cluster history, and a dedicated `/insights` route

---

## Shipped Milestones

| Milestone | Title | Status | Delivered scope |
|-----------|-------|--------|-----------------|
| **M0** | Foundations | [x] Closed | Workspace architecture, Tauri shell, shared frontend packaging, CI, and deterministic fixtures |
| **M1** | Connect + Browse | [x] Closed | Kubeconfig import, context switching, namespace selection, overview dashboard, and watch-backed browsing |
| **M2** | Debug Loop + UX Foundation | [x] Closed | Pod detail, logs, events, YAML, search, shortcuts, themes, breadcrumbs, and settings |
| **M3** | Resource Actions + Safe Ops | [x] Closed | Create/apply/delete, scale, rollout restart/status, non-interactive exec, basic pod port-forward, and guardrails |
| **M4** | AKS Visibility + Guardrails | [x] Closed | AKS detection, node-pool awareness, Portal links, workload identity hints, and production-context UX |
| **M5** | Helm + Metrics | [x] Closed | Helm release operations, redacted values/history/rollback, and pod/node metrics with trend views |
| **M6** | Search, CRDs + Advanced UX | [x] Closed | Cached-resource search, CRD discovery/instance browsing, generic routes, filtering, and polished navigation |
| **M7** | Desktop Hardening + Auditability | [x] Closed | Desktop packaging maturity, local audit logging, safer destructive actions, and operator polish |
| **M8** | Resource Breadth + Operator Workflows | [x] Closed | Broader built-in resource coverage, generic detail/action flows, and desktop workflow consolidation |
| **M9** | Desktop Resource Expansion | [x] Complete | 16 primary resource blades and 28+ watched resource types across the desktop cache and UI |
| **M10** | Azure ARM Management Plane | [x] Complete | `telescope-azure` ARM client, AKS node-pool CRUD, cluster start/stop, upgrade profiles, pool/node-image upgrades, and ARM-backed diagnostics |
| **M11** | AI Insights | [x] Complete | BYOK Azure OpenAI integration, dual auth (Entra ID / API Key), multi-cloud support, allowlist-only context builder, structured JSON responses, per-cluster history, and `/insights` route |

---

## v1.0.0 Feature Summary by Area

### Desktop operator workflows
- Real cluster connection + namespace switching
- Broad resource inventory across workloads, networking, storage, RBAC, autoscaling, scheduling, and admission resources
- Pod detail workflows for logs, events, YAML, exec, and delete/restart actions
- Workload scaling and rollout restart/status flows
- Generic resource detail pages and create/apply workflows

### AKS/Azure differentiators
- ARM resource resolution from the active AKS cluster context
- Azure Portal deep links from the desktop UI
- AKS cluster start/stop controls
- Cluster upgrade profile inspection and control-plane upgrade actions
- Node-pool list/create/delete/scale/autoscaler/version/node-image operations
- Maintenance configuration visibility and ARM-backed diagnostics
- AKS-first depth while still supporting any conformant Kubernetes cluster

### Desktop UX delivered in v1.0.0
- Global search palette
- Dark/light themes
- Keyboard shortcuts + help overlay
- Breadcrumbs and grouped sidebar navigation
- Settings/preferences surface
- Local audit logging for operator actions

### AI Insights delivered in v1.2.0
- BYOK Azure OpenAI integration with user-configured endpoint and deployment
- Dual auth: Azure Entra ID (`DefaultAzureCredential`) and session-only API Key
- Multi-cloud support: Commercial, Government, Secret, and Top Secret
- Allowlist-only context builder with redaction and deterministic size caps
- Structured JSON responses: summary, risks, observations, recommendations, references
- Per-cluster history storage with clear-all control
- Dedicated `/insights` route with test, generate, history, and clear actions
- Settings page for endpoint, deployment, cloud profile, and auth mode

### Known deliberate gaps after v1.2.0
- Fully interactive exec terminal with TTY/xterm.js
- Richer port-forward session management
- In-app Helm upgrade + diff workflows
- Broader table sorting, richer label filtering, and more advanced analytics/visualization surfaces
- Plugin/extensibility model

---

## Future Milestones (M12+)

### [ ] M12 — Interactive Terminal + Deeper Live Operations
Focus on making the desktop debug loop fully self-contained.

Planned scope:
- xterm.js-backed interactive exec / TTY sessions
- richer port-forward lifecycle management and active-session visibility
- stronger copy/paste, resize, and reconnect ergonomics for live workflows
- deeper streaming UX around long-running operator actions

### [ ] M13 — Plugin System
Open Telescope up for controlled extension without regressing desktop safety.

Planned scope:
- plugin host / extension runtime
- permissions and trust model
- packaging/discovery story for internal and community plugins
- extraction of optional first-party integrations into plugins where it improves maintainability

### [ ] M14 — Advanced Visualizations + Operations Analytics
Build richer operator insight surfaces on top of the shipped inventory and metrics baseline.

Planned scope:
- workload and topology visualizations
- richer namespace / cluster rollups
- more advanced trend charts and comparative views
- better surfacing of relationships between workloads, services, storage, and policies

---

## Planning Notes for Post-v1

- Telescope planning is now **desktop-first**. Discontinued browser/Hub delivery modes are no longer roadmap drivers.
- AKS remains the deepest provider integration, but the product should continue to work well across non-AKS clusters.
- Future milestones should extend the shipped v1.0.0 operator core rather than reopen discontinued deployment modes.

---
title: Product Requirements
nav_exclude: true
description: "Product requirements document — internal planning reference"
---

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

> Status legend: [x] shipped in v1.0.0, [~] intentionally partial in v1.0.0, [ ] future milestone

### Cluster Management
- [x] Kubeconfig import/merge and context discovery
- [x] Context list + switching with explicit connection-state handling
- [x] Namespace switching
- [x] Production-context detection and stronger confirmations in the UI
- [ ] Context favorites / pinning

### Resource Explorer
- [x] Desktop cache/watch coverage for **28+ built-in resource types**, including Pods, Deployments, StatefulSets, DaemonSets, ReplicaSets, Jobs, CronJobs, Services, Ingresses, NetworkPolicies, EndpointSlices, ConfigMaps, Secrets, PVCs, ResourceQuotas, LimitRanges, Events, ServiceAccounts, Roles, ClusterRoles, RoleBindings, ClusterRoleBindings, HPAs, PDBs, PriorityClasses, StorageClasses, PVs, Nodes, and admission webhooks
- [x] 16 primary desktop resource blades plus generic resource detail routing
- [x] Collapsible sidebar with grouped navigation
- [x] Cluster overview dashboard
- [x] Node list/detail with capacity, usage, conditions, and YAML
- [x] CRD discovery and instance browsing
- [x] Per-list text filtering and global search palette over cached resources
- [~] Rich label filtering and broader table sorting across every list

### Debug Loop
- [x] Pod detail page (Summary / Logs / Exec / Events / YAML tabs)
- [x] Log viewer: streaming/snapshot, container selector, search, previous logs, auto-scroll
- [x] Events viewer with filtering
- [~] Exec terminal: reliable non-interactive exec is shipped; full interactive TTY/xterm.js remains future work
- [~] Port-forward: basic pod-focused desktop flow is shipped; richer active-session management remains future work

### Resource Actions
- [x] Resource YAML viewing across built-in and dynamic resource detail flows
- [x] Server-side apply / create-update workflows for supported built-in resources and CRD instances
- [x] Delete flows for shipped resource detail pages and supported generic resources
- [x] Scale actions for Deployments and StatefulSets
- [x] Rollout restart/status actions for supported workloads
- [x] Create resource from YAML/templates

### Helm
- [x] Release list/detail
- [x] Values viewer with redaction of known sensitive keys
- [x] Revision history
- [x] Rollback
- [ ] Upgrade with diff preview

### Metrics
- [x] metrics-server discovery
- [x] Pod CPU + memory usage
- [x] Node CPU + memory usage
- [x] Basic trend charts / sparklines
- [~] Namespace-level rollups can still get deeper

### UX & Polish
- [x] Dark theme
- [x] Light theme + theme toggle
- [x] Keyboard shortcuts + help overlay
- [x] Breadcrumb navigation
- [x] Search palette
- [x] Settings / preferences surface
- [x] Local audit logging for desktop operations
- [ ] Tabbed workspace / hotbar

### AKS-Specific (our differentiator)
- [x] ARM-backed AKS identity/resource resolution from the active cluster context
- [x] Azure Portal deep links and ARM resource awareness
- [x] Cluster start/stop controls
- [x] Cluster upgrade profile visibility and control-plane upgrade management
- [x] Node-pool listing, create/delete, scale, autoscaler updates, version upgrades, and node-image upgrades
- [x] Maintenance configuration / diagnostics visibility from the ARM management plane
- [x] Node-pool visibility in Kubernetes inventory views
- [x] Multi-cloud baseline: Telescope works against any Kubernetes cluster, while AKS clusters unlock Azure-specific controls
- [x] Production guardrails

### Extensions
- [ ] Plugin / extension host
- [ ] Plugin permissions model
- [ ] First-party plugin extraction

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
| **M0** | Foundations | [x] Complete | Monorepo architecture, Rust workspace, desktop shell, shared frontend packaging, CI, and deterministic fixtures |
| **M1** | Connect + Browse | [x] Complete | Real cluster connectivity, context/namespace switching, watch-backed cache, overview dashboard, and broad built-in resource browsing |
| **M2** | Debug Loop + UX Foundation | [x] Complete | Pod detail, logs, events, YAML, search palette, shortcuts, themes, breadcrumbs, and settings surface |
| **M3** | Resource Actions + Safe Ops | [x] Complete | Create/apply/delete flows, scale/restart/status actions, pod-focused exec/port-forward, and operator guardrails |
| **M4** | AKS Visibility + Guardrails | [x] Complete | AKS detection, node-pool visibility, Azure resource awareness, Portal links, and production-context safeguards |
| **M5** | Helm + Metrics | [x] Complete | Helm list/detail/history/values/rollback plus pod/node metrics and trend views |
| **M6** | Search, CRDs + Advanced UX | [x] Complete | Search palette, CRD discovery/instance browsing, generic detail routes, themes, filtering, shortcuts, and breadcrumbs |
| **M7** | Desktop Hardening + Auditability | [x] Complete | Desktop packaging maturity, local audit logging, safer destructive operations, and operator-facing polish |
| **M8** | Resource Breadth + Operator Workflows | [x] Complete | Expanded built-in resource coverage, generic actions, cluster-wide inventory depth, and desktop workflow consolidation |
| **M9** | Desktop Resource Expansion | [x] Complete | 16 primary Kubernetes resource blades and 28+ watched resource types shipped in the desktop cache |
| **M10** | Azure ARM Management Plane | [x] Complete | `telescope-azure` ARM client, AKS node-pool CRUD, cluster start/stop, upgrade management, and ARM-backed diagnostics |
| **M11** | AI Insights | [x] Complete | BYOK Azure OpenAI integration, Entra ID and API Key auth, multi-cloud support, allowlist-only context builder, structured JSON responses, per-cluster history, and dedicated `/insights` route |

## 9) AI Insights — Shipped in v1.2.0

> Status: Shipped in v1.2.0. This section describes the AI Insights feature as implemented and available in the desktop app.

### Summary
AI Insights is a dedicated route (`/insights`) in the desktop app that summarizes current Kubernetes and AKS state, highlights notable risks, and suggests next actions using Azure OpenAI. The feature is BYOK (bring your own key/credential), advisory only, and does not execute changes. It respects existing RBAC and namespace scope and avoids sending secrets or unsafe raw payloads to the model.

### Product intent
- Give operators a fast, grounded readout of cluster posture without replacing deterministic troubleshooting views.
- Use the existing local cache, watcher state, and AKS ARM detail as the source material for synthesis.
- Make insight generation feel first-party and operator-oriented, not like a generic chatbot bolted onto the product.

### Implementation decisions
- **Authentication:** two user-selectable auth modes: Azure Entra ID via `DefaultAzureCredential` and API Key (session-only, never persisted to disk).
- **Provider scope:** Azure OpenAI is the only supported provider.
- **Multi-cloud support:** Azure Commercial, Government, Secret, and Top Secret cloud environments are supported with configurable cloud profile selection in the Settings page.
- **UI placement:** AI Insights is a dedicated `/insights` route with its own sidebar entry, not an additional tab inside Overview.
- **Persistence:** generated insights are stored in history scoped per cluster, with a clear-all control for the current cluster scope.
- **Context shaping:** the cluster context builder uses allowlist-only field selection with redaction. Broad contextual coverage across workloads, events, node conditions, Helm state, and AKS posture is included, with deterministic size caps per category before serialization.
- **Redaction:** allowlist-only input policy sends curated summary fields rather than raw resource objects. Secrets, token-like values, kubeconfigs, and credential material are never included.
- **Response format:** structured JSON response schema (summary, risks, observations, recommendations, references) rendered deterministically in the UI.

### Shipped capabilities
- Dedicated `/insights` route in the desktop UI with generate, test connection, history, and clear actions.
- Settings page configuration for Azure OpenAI endpoint, deployment/model name, cloud profile, and auth mode.
- Test connection flow that validates a real chat-completions-capable request path using the configured endpoint and auth mode.
- Scoped context payload built from cached Kubernetes resources, recent failures, cluster conditions, Helm state, and AKS ARM details when available.
- Allowlist-only context builder with deterministic per-category caps and redaction of sensitive fields.
- Structured output rendered in the UI: summary, risks, observations, recommendations, and related resource references.
- History storage per cluster scope with clear-all control.

### Non-goals
- General-purpose chat assistant behavior.
- Auto-remediation or direct execution of model-suggested actions.
- Persisting raw prompts or raw model outputs by default.
- Cross-tenant AI routing logic.

### Data and trust boundaries
- Uses only data already available to the app through the current user context, watcher cache, and authorized ARM reads.
- Never expands scope just for the AI feature.
- Never includes Kubernetes Secret payloads, service-account tokens, connection strings, or obvious credential material in model input.
- Prefers sending normalized summaries and selected safe fields over raw Kubernetes object bodies.
- Treats AI output as advisory text that links back to product-visible state.

### Authentication details
Two user-selectable authentication paths are supported:

1. **Azure Entra ID:** uses `DefaultAzureCredential` to authenticate against the user-provided Azure OpenAI endpoint. If the credential resolves but the user lacks RBAC for chat completions, the product fails gracefully with explicit guidance to switch to API Key mode.
2. **API Key:** session-only fallback for local development, explicit service configuration, and environments without a usable Azure login context. Keys are never persisted to disk.

Auth mode is a UX-visible setting on the Settings page. Cloud profile selection supports sovereign cloud environments without feature redesign.

### System prompt contract
The system prompt enforces product behavior and response boundaries. The model acts as a constrained Kubernetes and AKS operations summarizer that:

- prioritizes accuracy over completeness;
- grounds every conclusion in the provided context only;
- avoids speculation when evidence is weak or missing;
- never asks the user for secrets or additional credentials;
- never recommends destructive action without naming the observed condition that justifies it;
- produces output in a strict, renderable schema.

Output schema:
```text
{
	"summary": string,
	"risks": [{"title": string, "detail": string, "impact": "low" | "medium" | "high"}],
	"observations": [{"area": string, "detail": string}],
	"recommendations": [{"action": string, "rationale": string, "confidence": number}],
	"references": [{"kind": string, "name": string, "namespace": string | null}]
}
```

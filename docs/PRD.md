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

## 9) Planned Addendum — AI Insights

> Status: Planned. This section defines the intended v1 scope for AI-generated operational insights. It does not describe shipped behavior.

### Summary
AI Insights will provide a separate route in the desktop app that summarizes current Kubernetes and AKS state, highlights notable risks, and suggests next actions using Azure OpenAI. The feature is advisory only: it does not execute changes, it must respect existing RBAC and namespace scope, and it must avoid sending secrets or unsafe raw payloads to the model.

### Product intent
- Give operators a fast, grounded readout of cluster posture without replacing deterministic troubleshooting views.
- Use the existing local cache, watcher state, and AKS ARM detail as the source material for synthesis.
- Make insight generation feel first-party and operator-oriented, not like a generic chatbot bolted onto the product.

### v1 decisions
- **Authentication:** support two user-selectable auth paths in v1: Azure login context via Azure RBAC when available, or API key fallback.
- **Provider scope:** Azure OpenAI is the only in-scope provider for v1.
- **Cloud environment pinning:** tenant pinning is not required in v1, but the product should expose cloud profile selection in the UI and support future Azure Government / Secret / Top Secret endpoint handling.
- **UI placement:** AI Insights ships as a separate route, not as an additional tab inside Overview.
- **Prompt preview:** no user-facing sanitized prompt preview in v1.
- **Persistence:** persist generated insights in encrypted local history, retaining only the last 3 entries per cluster.
- **Context shaping:** prefer broad contextual coverage across the available cluster and AKS state, while still enforcing hard size limits before provider calls.
- **Redaction:** use an allowlist-only input policy; send curated summary fields rather than raw resource objects.
- **Diagnostics:** expose dev-mode metadata only, not a general-user debugging panel.

### Goals
- Generate a concise summary of cluster and AKS state using local product context.
- Surface risks, observations, and recommended next actions with references back to real resources.
- Fail safely and clearly when AI configuration or provider access is unavailable.

### Non-goals
- General-purpose chat assistant behavior.
- Auto-remediation or direct execution of model-suggested actions.
- Persisting raw prompts or raw model outputs by default.
- Cross-tenant AI routing logic in v1.

### Functional outline
- Add a dedicated route for Insights in the desktop UI.
- Provide Settings support for Azure OpenAI endpoint, deployment/model name, cloud profile, and auth mode configuration.
- During the "Test connection" flow, use the configured endpoint and the selected auth mode to validate a real chat-completions-capable request path.
- Build a scoped context payload from cached Kubernetes resources, recent failures, cluster conditions, Helm state, and AKS ARM details when available, with broad coverage and deterministic size caps.
- Redact or omit secrets, token-like values, kubeconfigs, and other sensitive fields before any model call using an allowlist-only context builder.
- Return structured output that the UI can render deterministically: summary, risks, observations, recommendations, and related resource references.
- Persist generated insights locally in encrypted storage for offline review and later comparison, retaining only the last 3 entries per cluster and exposing a clear-all control for the current cluster.

### Data and trust boundaries
- Use only data already available to the app through the current user context, watcher cache, and authorized ARM reads.
- Never expand scope just for the AI feature.
- Never include Kubernetes Secret payloads, service-account tokens, connection strings, or obvious credential material in model input.
- Prefer sending normalized summaries and selected safe fields over raw Kubernetes object bodies.
- Treat AI output as advisory text that must link back to product-visible state.

### Authentication and configuration intent
The v1 design should support two user-selectable authentication paths:

1. **Azure login context:** use an existing authenticated Azure user context and Azure RBAC when it is already available to the app environment.
2. **API key:** fallback path for local development, explicit service configuration, and environments without a usable Azure login context.

This should be a UX-visible setting rather than an implicit transport decision. For Azure login context, the product should use `DefaultAzureCredential()` to authenticate against the user-provided Azure OpenAI endpoint during the "Test connection" flow. If the credential resolves successfully but the user lacks RBAC to perform the chat completions request, the product must fail gracefully, explain that Azure RBAC access to the endpoint is insufficient, and suggest switching to API key authentication. The configuration surface should also expose cloud profile selection so sovereign cloud variants can be added without redesigning the feature.

### Context shaping intent
The model should receive broad but curated context, not a narrow incident snapshot and not an unbounded dump of raw objects. The goal is to give the model enough surrounding signal to reason about interacting issues while still constraining payload size and protecting sensitive data.

This implies:

- broad coverage across workloads, events, node conditions, Helm state, and AKS posture when available;
- fixed deterministic caps per category before serialization;
- normalized summaries and rankings over raw object blobs;
- stable ordering so repeated requests produce comparable inputs.

### System prompt intent
The system prompt is not meant to be a hidden personality layer or an open-ended assistant instruction set. Its purpose is to enforce product behavior and response boundaries. The intent is to make the model act like a constrained Kubernetes and AKS operations summarizer that:

- prioritizes accuracy over completeness;
- grounds every conclusion in the provided context only;
- avoids speculation when evidence is weak or missing;
- never asks the user for secrets or additional credentials;
- never recommends destructive action without naming the observed condition that justifies it;
- produces output in a strict, renderable schema.

In other words, the prompt should shape the model into a deterministic synthesis step inside Telescope, not a freeform chat persona.

### Proposed system prompt contract
The exact wording can change during implementation, but the prompt should encode these rules:

```text
You are Telescope AI Insights, an operations summarizer for a desktop Kubernetes IDE.

Your job is to analyze the provided Kubernetes and AKS context and return a concise, evidence-based assessment for the current cluster or namespace scope.

Rules:
- Use only the supplied context. If something is missing, say that it is not available.
- Do not invent resources, states, incidents, or metrics.
- Treat all output as advisory. Do not claim an action has been executed.
- Never request secrets, credentials, tokens, kubeconfigs, or hidden configuration.
- Prefer concrete observations over generic best practices.
- If recommending an action, explain why it follows from the observed state.
- If risk is uncertain, lower confidence and say why.
- Respect the provided scope. If the context is namespace-limited, do not make cluster-wide claims.
- Return valid JSON matching the required schema.

Output schema:
{
	"summary": string,
	"risks": [{"title": string, "detail": string, "impact": "low" | "medium" | "high"}],
	"observations": [{"area": string, "detail": string}],
	"recommendations": [{"action": string, "rationale": string, "confidence": number}],
	"references": [{"kind": string, "name": string, "namespace": string | null}]
}
```

### Why this prompt shape is the right intent
- It keeps the model subordinate to Telescope's actual data model.
- It reduces hallucination risk by banning unstated assumptions.
- It makes UI rendering stable because the output contract is explicit.
- It supports future provider changes because the prompt describes product behavior, not vendor-specific tricks.
- It fits the product direction: first-party operator workflow, not a generic conversational assistant.

### Diagnostics intent
Because v1 excludes prompt preview, supportability should come from dev-mode metadata only. That metadata should be shown on the Settings page in dev mode and include prompt version, redaction policy version, cloud profile, auth mode, context-size statistics, schema-validation failures, and provider error classification without exposing prompt bodies or sensitive payloads.

### Acceptance criteria for planning
- The implementation plan supports Azure login context and API key authentication paths, selectable in the UI.
- The route architecture assumes a dedicated Insights page.
- The prompt contract is schema-first and evidence-bound.
- Cloud profile selection is exposed in v1 and configuration design does not block sovereign cloud endpoint support.
- Persistence behavior assumes encrypted local history, retaining only the last 3 entries per cluster, with a clear-all control for the current cluster.
- The context builder uses allowlist-only serialization and fixed deterministic per-category caps even when broad contextual coverage is enabled.
- Azure login path uses `DefaultAzureCredential()` during test-connect validation and surfaces explicit RBAC failure guidance when chat completions access is denied.
- Debug support is limited to dev-mode metadata on the Settings page and excludes prompt preview.

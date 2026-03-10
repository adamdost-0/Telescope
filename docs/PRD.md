# Telescope — PRD (Draft)

> Working title: **Telescope** (aka “Aurora” in earlier drafts)

## 1) Summary
Telescope is an open-source Kubernetes IDE that matches Lens’ core day-1/day-2 workflows while being **AKS-optimized**, **low-memory**, and available as both a **desktop app** and a **web client**.

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

## 5) Lens parity checklist (MVP → v1)
- Kubeconfig import/merge; context list + favorites.
- Clear context/namespace banner; “prod guardrails”.
- Resource explorer for core kinds + dynamic CRDs.
- Logs: follow, container selector, previous logs, search/export.
- Exec terminal (multi-container).
- Port-forward: profiles, auto-reconnect.
- Helm: releases, history, diff, rollback.
- Metrics: top nodes/pods + basic charts.

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
- Deep multi-cloud integrations beyond “works with any cluster”.

## 8) Milestones
- **M0 Foundations**: repo + build, core engine skeleton, desktop boots, kubeconfig import.
- **M1 MVP Debug Loop**: explorer + logs/exec/events/port-forward + AKS connect flow + caching discipline.
- **M2 Helm + Metrics**.
- **M3 Web client + self-host server**.
- **M4 Plugin system v1 (WASM)**.

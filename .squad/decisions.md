# Squad Decisions

## Active Decisions

### 2026-03-19: K8s Capability Audit — Ship v1.0.0

**Authors:** Dallas (lead), Ripley, Lambert, Kane  
**Status:** Accepted  
**Type:** Architecture audit

**Context:** Full-stack audit of Telescope v1.0.0 K8s capabilities across engine, IPC, frontend, and test layers. Validated against live AKS cluster `dassadsawqew`.

**Findings:**
- 29 watched GVKs, 66 Tauri IPC commands, 65 frontend API functions, 39 routes — near-complete coverage
- All kubectl/helm live-cluster commands succeeded; zero failures
- All tests green: Rust 176/176, Web 36/36, E2E 16/16
- Frontend builds clean

**Gaps:**
| Gap | Severity | Notes |
|---|---|---|
| Helm install/upgrade/uninstall | Medium | Read-only + rollback today; no chart install or upgrade |
| Helm template/dry-run | Low | Pairs with install/upgrade when those ship |
| ReplicaSets list route | Low | Watched, accessible via generic detail |
| ClusterRoles list route | Low | Watched, accessible via generic detail |
| ClusterRoleBindings list route | Low | Watched, accessible via generic detail |

**Acceptable non-gaps:** VPA (CRD add-on, covered by CRD browser), legacy v1/Endpoints (superseded by EndpointSlices), Helm chart repos (CLI scope).

**Decision:** Ship v1.0.0 as-is. Only post-release priority is Helm write operations.

---

### 2026-03-19: SOTA Models Only

**Author:** Adam Dost (directive)  
**Status:** Active  
**Type:** Team policy

Only use latest SOTA models for all agent spawns: **Opus 4.6** and **GPT-5.3-Codex**. Haiku is forbidden.

---

## Governance

- All meaningful changes require team consensus
- Document architectural decisions here
- Keep history focused on work, decisions focused on direction

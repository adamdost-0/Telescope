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

### 2026-03-19: Post-Audit Work Priorities

**Author:** Dallas (Lead)  
**Status:** Accepted  
**Type:** Prioritization

**Context:** Prioritized backlog from the K8s capability audit. v1.0.0 shipped clean — these are post-release improvements.

**Items:**
| ID | Item | Priority | Owner | Depends on |
|---|---|---|---|---|
| P1-1 | Helm install | P1 | Ripley + Lambert | — |
| P1-2 | Helm upgrade | P1 | Ripley + Lambert | P1-1 |
| P1-3 | Helm uninstall | P1 | Ripley + Lambert | — |
| P1-4 | Helm template/dry-run | P1 (lower) | Ripley + Lambert | P1-1, P1-2 |
| P2-1 | ReplicaSets list route | P2 | Lambert | — |
| P2-2 | ClusterRoles list route | P2 | Lambert | — |
| P2-3 | ClusterRoleBindings list route | P2 | Lambert | — |

**Decision:** Helm writes are P1, not P0. Recommended sequence: uninstall first (quick win), then install+upgrade together, then template/dry-run. Missing list routes are P2 — batch when convenient.

---

### 2026-03-19: ARM Node Pool Error Handling

**Authors:** Ripley (backend), Lambert (frontend), Kane (tests)  
**Status:** Accepted  
**Type:** Bug fix + UX improvement

**Context:** ARM node pool failures were surfaced as generic errors or silently swallowed. `listAksNodePools` returned `[]` on error, hiding failures. `delete_node_pool` polling treated any GET error as successful deletion.

**Changes:**
- Backend: Typed ARM error variants (TokenExpired, SubscriptionNotFound, ResourceGroupNotFound, ClusterNotFound, PermissionDenied, Timeout) with actionable messages
- Frontend: Dismissible error banner on node-pools page with guidance mapping; `listAksNodePools` now rethrows after notification
- Bug fix: `delete_node_pool` only treats `NotFound` as successful disappearance; other errors propagate
- Tests: Rust unit tests for error mapping, Playwright E2E for error display/dismiss/retry recovery, mock-tauri error injection support

**Decision:** Keep node pool inventory as authoritative ARM `agentPools` reads. Treat ARM failures as first-class user-visible errors with actionable remediation.

---

## Governance

- All meaningful changes require team consensus
- Document architectural decisions here
- Keep history focused on work, decisions focused on direction

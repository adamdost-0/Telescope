# Agent Guidance — Documentation

## Overview

`docs/` contains architecture documentation, design docs, and testing guides. **Important:** Much of this documentation is **aspirational** — it describes the target system, not always the current implementation.

**Rule of thumb:** When docs contradict code or CI, the code wins.

## Documentation Files

| File | Status | Notes |
|---|---|---|
| `ARCHITECTURE.md` | Partially aspirational | Layered architecture is real; gRPC API layer is NOT built |
| `PRD.md` | Vision | Product requirements and goals |
| `ROADMAP.md` | Planning | Feature timeline and milestones |
| `TESTING.md` | Partially aspirational | Testing strategy; some practices enforced (Vitest, Playwright, k3d), others not |
| `SECURITY.md` | Partially aspirational | OIDC scaffolded only, no signature validation; no authorization model |
| `TEST_PLAN.md` | Aspirational | Detailed test scenarios; many not automated |
| `SMOKE_TEST.md` | Aspirational | Smoke test checklist; not automated |
| `AKS_QUICKSTART.md` | Aspirational | Deployment guide; no Helm chart or K8s manifests exist |
| `UX_NOTES.md` | Design notes | UI/UX considerations |
| `retrospectives/` | Historical | Past iteration learnings; one file per milestone |

## What's Currently Real (vs. Aspirational)

### Real and Implemented

- Rust crates with real kube-rs integration (watchers, exec, logs, port-forward, Helm, metrics, CRDs)
- SQLite `ResourceStore` + `ResourceWatcher` architecture in both desktop and hub
- SvelteKit web app with Svelte 5, 20+ pages, 20+ components
- Tauri desktop with 35+ IPC commands
- `apps/hub` Axum server with full `/api/v1/*` REST API
- CI enforcing Rust fmt/clippy/test, web tests/build/E2E, desktop builds
- k3d integration tests for engine crates
- Audit logging (JSONL) for destructive actions

### Documented but NOT Implemented

- Production OIDC with JWT signature validation
- gRPC API server
- Helm charts and Kubernetes deployment manifests
- Authorization model (per-user/group cluster access control)
- Write-operation hub parity (scale, delete, exec, port-forward in web mode)
- Performance benchmarks and load tests
- Chaos engineering and resilience tests
- Automated smoke tests
- Security audit automation
- `packages/ui` shared component library

## Aspirational Sections to Watch For

1. **`ARCHITECTURE.md`:**
   - gRPC API layer — **not built**
   - Advanced caching/reconciliation — **partially implemented** (basic SQLite cache exists)
   - Layered architecture is **real** (`core → engine → api/hub`)

2. **`SECURITY.md`:**
   - OIDC flow with Azure Entra ID — **scaffolding only** (no signature validation, no session management)
   - Authorization model — **not implemented**

3. **`TESTING.md`:**
   - Vitest unit tests — **real and CI-enforced**
   - Playwright E2E (against stub server) — **real and CI-enforced**
   - k3d integration tests — **real, triggered on engine/core changes**
   - Integration tests with real AKS clusters — **not in CI**
   - Performance benchmarks — **don't exist**

4. **`AKS_QUICKSTART.md`:**
   - Kubernetes deployment with Helm — **no Helm chart exists**
   - Multi-cluster management — **partially implemented** (kubeconfig context switching works)

## Marking Aspirational Content

When documenting unimplemented features, use:

```markdown
## Feature X _(Planned)_

**Status:** Not implemented. Target: Milestone N.
```

Or inline:

```markdown
> ⚠️ **Note:** This section describes planned functionality.
> Current implementation is scaffolding only.
```

## Updating Documentation

1. **After implementing an aspirational feature:** Remove the "Planned" marker, update the status table above
2. **When adding a new feature:** Update `ARCHITECTURE.md` and the relevant area docs
3. **When security model changes:** Update `SECURITY.md` with real implementation details
4. **Verify commands before documenting them** — all CLI examples should be tested

## Documentation Conventions

- Use GitHub-flavored Markdown
- Include syntax-highlighted code blocks (` ```rust`, ` ```typescript`, ` ```bash`)
- Link to actual repo files (e.g., `[ci.yml](.github/workflows/ci.yml)`)
- Use tables for structured information
- Keep docs concise — link to external references

## What's NOT Here

- API reference docs (no auto-generated API docs)
- Contributor guide (no `CONTRIBUTING.md`)
- End-user guides
- Changelog (`CHANGELOG.md`)
- Architecture Decision Records (ADRs)

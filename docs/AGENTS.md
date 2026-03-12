# Agent Guidance — Documentation

## Overview

`docs/` contains architecture documentation, design docs, and testing guides. **Important:** Much of this documentation is **aspirational** — it describes the target system, not always the current implementation.

## Documentation Files

| File | Status | Notes |
|------|--------|-------|
| `ARCHITECTURE.md` | Aspirational | Describes target layered architecture |
| `PRD.md` | Vision | Product requirements and goals |
| `ROADMAP.md` | Planning | Feature timeline and milestones |
| `TESTING.md` | Partially aspirational | Testing strategy; not all practices enforced yet |
| `SECURITY.md` | Partially aspirational | Security model; OIDC is scaffolded, not production-ready |
| `TEST_PLAN.md` | Aspirational | Detailed test plan; many tests not implemented |
| `SMOKE_TEST.md` | Aspirational | Smoke test checklist; not automated |
| `AKS_QUICKSTART.md` | Aspirational | Deployment guide; no Helm chart or K8s manifests exist |
| `UX_NOTES.md` | Design notes | UI/UX considerations |
| `retrospectives/` | Historical | Past iteration learnings |

## How to Read These Docs

**Rule of thumb:** When docs contradict code or CI, the code wins. These docs describe **where we're going**, not always **where we are**.

### Aspirational Sections to Watch For

1. **`ARCHITECTURE.md`:**
   - Describes gRPC API layer — **not built yet**
   - Mentions advanced caching and reconciliation — **partially implemented**
   - Layered architecture is real (core → engine → api/hub), but api layer is minimal

2. **`SECURITY.md`:**
   - OIDC flow with Azure Entra ID — **scaffolding only**, no signature validation
   - Authorization model — **not implemented**
   - Secret rotation — **not implemented**

3. **`TESTING.md`:**
   - Comprehensive test coverage targets — **partially met**
   - Integration tests with real clusters — **not in CI**
   - Performance benchmarks — **don't exist**

4. **`TEST_PLAN.md`:**
   - Detailed test scenarios — **many not automated**
   - Load testing and chaos engineering — **not implemented**

5. **`AKS_QUICKSTART.md`:**
   - Kubernetes deployment with Helm — **no Helm chart exists**
   - Production-ready manifests — **don't exist**
   - Multi-cluster management — **partially implemented**

## Current vs. Aspirational

### What's Currently Real (v0.0.1)

- Rust crates with real K8s client integration (kube-rs)
- SvelteKit web app with Svelte 5 and Vitest/Playwright tests
- Tauri desktop app packaging
- `apps/hub` Axum server with basic REST endpoints
- CI enforcing Rust fmt/clippy/test and web tests/E2E
- Basic tracing and logging

### What's Aspirational (Documented but Not Implemented)

- Production-grade OIDC with signature validation
- gRPC API server
- Helm charts and K8s deployment manifests
- Comprehensive authorization model
- Advanced resource diffing and reconciliation
- Performance benchmarks and load tests
- Chaos engineering and resilience tests
- Automated smoke tests
- Security audit automation

## Updating Documentation

### When to Update

1. **Major architecture changes:** Update `ARCHITECTURE.md` to reflect new layers or services
2. **Security model changes:** Update `SECURITY.md` when adding auth/authz logic
3. **Testing strategy changes:** Update `TESTING.md` when adding new test types
4. **Deployment changes:** Update `AKS_QUICKSTART.md` when adding Helm/K8s manifests

### How to Mark Aspirational Content

Use clear markers for unimplemented features:

```markdown
## Feature X (Planned)

**Status:** Not implemented yet. Target: Milestone 2.

Description of the planned feature...
```

Or use admonition-style blocks:

```markdown
> **⚠️ Note:** This section describes planned functionality.
> Current implementation is scaffolding only.
```

### Keeping Docs Grounded

Before merging doc updates:

1. Check if the described feature exists in code
2. Verify CI actually enforces described validations
3. Test commands to ensure they work as documented
4. Add "Status" or "Current State" sections to clarify implementation level

## Documentation Conventions

- Use GitHub-flavored Markdown
- Include code examples with syntax highlighting (` ```rust`, ` ```typescript`, etc.)
- Link to actual files in the repo (e.g., `[ci.yml](.github/workflows/ci.yml)`)
- Use tables for structured information
- Add TOC for long documents (use `<!-- toc -->` if using markdown-toc)
- Keep docs concise — link to external resources for deep dives

## CI and Documentation

**Current state:** No automated doc validation in CI.

**Potential additions:**
- Markdown linting (`markdownlint`)
- Link checking (ensure internal links aren't broken)
- Code example validation (extract and test code snippets)
- Spell checking

## Retrospectives

`docs/retrospectives/` contains historical learnings:

- One file per iteration or milestone
- Capture what worked, what didn't, action items
- Not API documentation — internal team reflections

## When to Edit

- **Add a new feature:** Update relevant docs (ARCHITECTURE, TESTING, etc.)
- **Change deployment model:** Update AKS_QUICKSTART and any deployment docs
- **Implement aspirational features:** Remove "planned" markers, update status
- **Find drift:** When code contradicts docs, update docs to match reality or file an issue

## What's NOT Here

- API reference docs (no auto-generated API docs yet)
- Contributor guide (no CONTRIBUTING.md)
- User-facing guides (no end-user documentation structure)
- Changelog (no CHANGELOG.md or release notes automation)

These are future additions when the project matures beyond v0.0.1.

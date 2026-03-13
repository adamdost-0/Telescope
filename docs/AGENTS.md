# Agent Guidance — Documentation

## Overview

`docs/` contains architecture notes, product docs, security/testing guidance, and planning material. Some documents remain aspirational, but several have now been refreshed to reflect the **current substantial alpha** codebase rather than the original v0.0.1 scaffold.

## Documentation Files

| File | Status | Notes |
|------|--------|-------|
| `ARCHITECTURE.md` | Mixed / somewhat aspirational | Layering is real, but some target architecture sections still describe future direction |
| `PRD.md` | Mixed current + vision | Product goals plus milestone tracking; now reflects shipped vs partial features |
| `ROADMAP.md` | Current planning doc | Tracks shipped, partial, and planned milestones against current code reality |
| `TESTING.md` | Current suite + future direction | Documents the real Rust/Vitest/Playwright footprint and remaining gaps |
| `SECURITY.md` | Mixed current + planned | Describes implemented auth/audit/redaction controls and the remaining security roadmap |
| `TEST_PLAN.md` | Aspirational | Detailed scenarios; many are still not automated |
| `SMOKE_TEST.md` | Mostly aspirational | Useful manual checklist, not full automation |
| `AKS_QUICKSTART.md` | Aspirational / partial | Deployment guidance still runs ahead of the current repo packaging story |
| `UX_NOTES.md` | Current route inventory + design notes | Reflects actual `apps/web/src/routes` pages and current UI patterns |
| `retrospectives/` | Historical | Past iteration learnings |

## How to Read These Docs

**Rule of thumb:** when docs contradict code or CI, the code wins. `docs/` should explain the current system clearly, while still calling out future intent where relevant.

### Sections that are still likely to be aspirational
1. **`ARCHITECTURE.md`**
   - Some layering and service-boundary sections still describe the target end state.
   - `crates/api` is still much thinner than the aspirational architecture suggests.

2. **`SECURITY.md`**
   - Auth middleware, audit logging, and secret redaction are real.
   - Production-grade OIDC, JWT validation, and real authorization are still not complete.

3. **`TEST_PLAN.md` / `SMOKE_TEST.md`**
   - These remain more exhaustive than the currently automated suite.

4. **`AKS_QUICKSTART.md`**
   - Treat as target deployment guidance, not proof that all packaging/manifests exist today.

## Current vs. Aspirational

### What's Currently Real
- Real Kubernetes connectivity, context switching, namespace management, and watch-backed sync
- Web/desktop UI with overview, pods, nodes, events, create, settings, Helm, CRDs, and broad `/resources/*` coverage
- Pod logs, non-interactive exec, basic pod port-forward, create/apply/delete/scale/restart support (desktop/Tauri has the broadest coverage)
- Helm release browsing/history/values/rollback and metrics-server-backed pod/node metrics
- Search palette, keyboard shortcuts, theme toggle, breadcrumbs, and production-context UI warnings
- Hub service with REST API, auth scaffolding, audit logging, and SQLite-backed cache
- Real automated test surface: 107 Rust tests, 5 Vitest files, and 4 Playwright specs

### What's Still Aspirational or Partial
- Production-grade OIDC/JWT validation and fine-grained authorization
- Full browser/Hub parity for write operations and richer live workflows
- Fully interactive exec terminal and more complete port-forward UX
- In-app Helm upgrade/diff and broader generic CRD detail/edit support
- Plugin system, richer AKS integrations, and deployment packaging/docs beyond the current alpha level

## Updating Documentation

### When to Update
1. **Major architecture changes:** update `ARCHITECTURE.md` when layers or app boundaries materially change.
2. **Security changes:** update `SECURITY.md` when auth/authz, redaction, or audit behavior changes.
3. **Testing changes:** update `TESTING.md` when new suites, counts, or coverage areas land.
4. **Route/UX changes:** update `UX_NOTES.md` when actual route inventory or key navigation patterns change.
5. **Milestone shifts:** update `PRD.md` and `ROADMAP.md` when planned work becomes shipped or partial reality.

### How to Mark Aspirational Content
Use explicit markers for unimplemented or partial work:

```markdown
## Feature X (Planned)

**Status:** Not implemented yet. Target: Milestone 2.

Description of the planned feature...
```

Or an admonition-style note:

```markdown
> **⚠️ Note:** This section describes planned functionality.
> Current implementation is partial.
```

### Keeping Docs Grounded
Before merging doc updates:
1. Check the code path or route actually exists.
2. Verify CI/workflow commands are real and still relevant.
3. Prefer “implemented / partial / planned” language over vague future-tense prose.
4. Call out desktop-vs-browser differences when parity is incomplete.

## Documentation Conventions
- Use GitHub-flavored Markdown.
- Link to actual files in the repo when describing code-backed behavior.
- Use tables for route inventories, milestones, and status summaries.
- Keep future sections clearly labeled rather than mixing them into “current state” prose.

## CI and Documentation
**Current state:** there is still no dedicated markdown lint/link-check pipeline.

**Potential additions:**
- Markdown linting
- Link checking
- Code-snippet validation
- Spell checking

## Retrospectives
`docs/retrospectives/` contains historical learnings:
- one file per iteration or milestone,
- what worked / what didn't / action items,
- internal team reflections rather than API docs.

## When to Edit
- **Add or remove routes/features:** update `UX_NOTES.md`, `PRD.md`, and `ROADMAP.md`.
- **Change security posture:** update `SECURITY.md`.
- **Grow the automated suite:** update `TESTING.md`.
- **Find drift:** update docs to match code, and clearly mark anything that remains future work.

## What's NOT Here
- Auto-generated API reference docs
- A mature contributor guide
- Full end-user documentation set
- Formal changelog/release-notes automation

Those remain future documentation workstreams as the project matures beyond the current alpha stage.

# Decision: GitHub Pages Documentation Site Structure

**Author:** Dallas (Lead)
**Status:** Accepted
**Type:** Documentation architecture

## Context

Telescope needed a public-facing documentation site. The `docs/` directory had 11+ markdown files with no navigation, no theming, and no deployment pipeline.

## Decision

- **Theme:** `just-the-docs` v0.10.1 via `remote_theme` — clean, searchable, sidebar navigation, native mermaid support.
- **Color scheme:** Dark mode default (matches the IDE aesthetic).
- **Deployment:** GitHub Actions workflow (`docs.yml`) triggers on `docs/` changes to main branch. Standard `actions/deploy-pages@v4` pattern.
- **Navigation tiering:**
  - **Public sidebar (nav_order 2–9):** Architecture, AKS Quickstart, Deployment, Security, Testing, Roadmap, UX Reference, Smoke Test
  - **Hidden from nav (nav_exclude: true):** PRD, Test Plan, Entra Auth Feasibility — accessible by direct URL only
  - **Excluded from Jekyll entirely:** AGENTS.md, retrospectives/, diagrams/
- **Landing page:** `docs/index.md` with project overview, navigation table, tech stack diagram, and links to GitHub releases/changelog.
- **No content changes:** Only front matter was added to existing files. No content was deleted, moved, or restructured.

## Rationale

- `just-the-docs` is the standard for open-source technical docs — searchable, accessible, low maintenance.
- Dark mode matches the desktop IDE identity.
- Internal planning docs stay accessible but don't clutter the public navigation.
- CHANGELOG stays in repo root and is linked externally — avoids duplication drift.
- Mermaid support renders the 5 architecture diagrams natively without any content changes.

## Impact

- All agents: docs/ files now have YAML front matter blocks at the top. Account for this when editing docs.
- CI: New `docs.yml` workflow runs independently from existing CI. No impact on `ci.yml` or `release.yml`.

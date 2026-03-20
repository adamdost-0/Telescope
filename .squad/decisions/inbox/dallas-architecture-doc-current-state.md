---
title: Align docs with shipped desktop architecture
author: Dallas
status: accepted
created: 2026-03-20
---

Decision: Updated architecture documentation to reflect the current shipped, desktop-only system and the canonical workspace shape. Small, targeted doc edits are preferred over speculative additions. When docs disagree with code or CI, update docs and record a short decision.

Files changed:
- docs/ARCHITECTURE.md (clarified frontend description)
- .squad/agents/dallas/history.md (appended learning entry)

Rationale: The repository's source of truth is the code and CI. Keeping docs aligned reduces onboarding friction and prevents stale guidance from influencing design decisions.

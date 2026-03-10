# Telescope Squad (AI Agentic Team)

This repo is intended to be built by an *agentic* AI team. This file defines the **roles**, **interfaces**, and **quality gates** that keep the team honest.

## Non‑negotiables
- **No Electron**. Desktop is **Tauri + native WebView**.
- **Deterministic CI**: no live AKS/K8s dependency in PR CI.
- **Container-only local validation** before pushing (use `scripts/dev-test.sh`).
- **No “green CI with zero tests.”** Every milestone increment expands tests.
- **Two-reviewer rule for agent work**: every non-trivial PR gets review by **two independent reviewer agents** (see “Review protocol”).

---

## Roles (the Squad)

### Tech Lead (TL)
Owns architecture, sequencing, and “smallest safe change.”
- Maintains API contracts and keeps UI/engine boundaries clean.
- Ensures memory efficiency constraints are respected.

### Web Engineer
Implements SvelteKit UI + tests.
- Uses URL-driven state and virtualization where lists are large.

### Desktop Engineer
Owns Tauri packaging/build stability.
- Keeps Win/Mac builds green; Linux desktop deps must not leak into Rust CI.

### Engine Engineer (Rust)
Owns deterministic engine behavior and contracts.
- Provides stubs/fixtures for CI and test harnesses.

### QA / Test Engineer
Owns test strategy and anti-flake rules.
- Prefers **stub servers** + fixtures for E2E over brittle request interception.

### Security Engineer
Owns dependency hygiene + SDL basics.
- Tracks vulnerabilities and defines upgrade playbooks.

### Release/CI Engineer
Owns GitHub Actions + release workflow.
- CI should fail fast with actionable output.

### UX / Performance
Owns performance-first UX.
- “Virtualize everything”, skeletons not blocking spinners, keyboard-first.

---

## Review protocol (anti-hallucination)
For each PR:
1) **Author agent** ships the change with tests.
2) **Reviewer agent A** performs a code review focused on correctness + API contract adherence.
3) **Reviewer agent B** performs a code review focused on tests + determinism + flake risks.

Reviewers must:
- Point to concrete lines/files.
- Call out any “made up” behavior (missing endpoints, nonexistent config keys, etc.).
- Reject changes that bypass the container-only dev loop.

---

## Learning capture
When we learn something (CI edge case, Tauri quirk, flaky E2E pattern), we record it in:
- `docs/retrospectives/*` (milestone retros)
- this file (process / team learnings)

Keep it short. Make it actionable.

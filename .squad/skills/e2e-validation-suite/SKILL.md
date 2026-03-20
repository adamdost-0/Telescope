---
name: "e2e-validation-suite"
description: "Run and triage the standard apps/web validation chain"
domain: "testing"
confidence: "high"
source: "observed"
---

## Context
Use this when validating frontend-impacting changes in Telescope. The reliable order is unit tests first, production build second, Playwright E2E last, so failures are isolated early and browser time is spent only after faster checks pass.

## Patterns
- Run from repo root with explicit `-C` paths:
  - `pnpm -C apps/web test`
  - `pnpm -C apps/web build`
  - `pnpm -C apps/web e2e`
- Treat this as a quality gate: stop on first failure and classify whether it is directly related to the change set.
- Fix only failures caused by the current change; otherwise report a blocker with the failing suite and symptom.
- Use `apps/web/tests-e2e/helpers/mock-tauri.ts` as the first stop for IPC-related E2E regressions.

## Examples
- Metrics/docs-adjacent validation: run all three commands in order and compare pass counts against current baseline (Vitest + Playwright).
- E2E-only failures after passing test/build typically indicate UI wiring, selector drift, or mock IPC contract mismatch.

## Anti-Patterns
- Running E2E before `test`/`build`, which slows feedback and obscures root cause.
- Fixing unrelated pre-existing failures while validating a focused change.
- Skipping build after unit tests; this misses bundling-time regressions.

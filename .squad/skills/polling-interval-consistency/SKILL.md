---
name: "polling-interval-consistency"
description: "Keep polling cadence changes safe by updating shared constants and timer-based tests together"
domain: "testing"
confidence: "high"
source: "observed"
---

## Context
Use this pattern when a polling interval changes in frontend runtime code. In Telescope, cluster vitals polling is centralized and tested with fake timers, so cadence updates must keep code and tests synchronized.

## Patterns
- Define polling cadence in one constant (`POLL_INTERVAL_MS`) in runtime logic.
- Drive polling with `setInterval` and guard overlap with an in-flight promise latch.
- In Vitest fake-timer tests, advance timers by the exact cadence value used at runtime.
- Keep behavior checks focused on business outcomes (availability fallback, ring-buffer bounds), not implementation details.
- Validate cadence updates with a targeted test run for the polling module, then a focused frontend build check.

## Examples
- Runtime: `apps/web/src/lib/realMetrics.ts` (`POLL_INTERVAL_MS`, `pollMetricsOnce`, `startMetricsPolling`)
- Tests: `apps/web/src/lib/realMetrics.test.ts` (`vi.advanceTimersByTimeAsync(...)`)
- Validation commands: `pnpm -C apps/web test -- --run src/lib/realMetrics.test.ts` and `pnpm -C apps/web build`

## Anti-Patterns
- Hardcoding different poll durations in tests versus runtime.
- Lowering poll interval without overlap protection, causing concurrent duplicate fetches.
- Changing cadence while silently altering history-limit or fallback semantics in the same patch.

# Agent Guidance — SvelteKit Web App

## Overview

`apps/web` is the SvelteKit 2 web client for Telescope, using Svelte 5 with runes. It serves as:

1. The standalone web UI (SvelteKit dev server or static build)
2. The frontend for `apps/desktop` (Tauri packages the built output)

## Technology Stack

- **Framework:** SvelteKit 2
- **Svelte version:** Svelte 5 (runes-based, NOT legacy syntax)
- **Build tool:** Vite 6
- **Language:** TypeScript
- **Testing:** Vitest (unit), Playwright (E2E)
- **Package manager:** pnpm

## Svelte 5 Rules (Critical)

**Always use Svelte 5 runes:**

```svelte
<script lang="ts">
  // ✅ Correct: use $props() and $state()
  let { cluster } = $props<{ cluster: Cluster }>();
  let expanded = $state(false);
  
  // ❌ Wrong: don't use legacy export let or $:
  // export let cluster;
  // $: expanded = false;
</script>
```

**Event handling:**

```svelte
<!-- ✅ Correct: Svelte 5 event syntax -->
<button onclick={handleClick}>Click me</button>

<!-- ❌ Wrong: don't use legacy on:click -->
<button on:click={handleClick}>Click me</button>
```

## Build and Test Commands

```bash
# Dev server (port 5173)
pnpm -C apps/web dev

# Production build
pnpm -C apps/web build

# Unit tests (Vitest)
pnpm -C apps/web test

# Run specific test file
pnpm -C apps/web exec vitest run src/lib/hello.test.ts

# E2E tests (Playwright)
pnpm -C apps/web e2e

# Install Playwright browsers (required once)
pnpm -C apps/web exec playwright install --with-deps

# Lint (currently a no-op — needs real implementation)
pnpm -C apps/web lint
```

## Current State (v0.0.1)

**What works:**
- SvelteKit routing and pages
- Stub `/api/clusters` endpoint with hardcoded data
- Basic cluster/namespace UI components
- Vitest unit tests for core functions
- Playwright E2E tests against a stub server

**What's NOT real yet:**
- No live connection to `apps/hub` or Rust engine
- `src/lib/engine.ts` is a fetch wrapper with no real engine integration
- E2E tests use `tools/devtest/stub-server.mjs` for deterministic stub data
- No shared component library (`packages/ui` is empty)

## Architecture

### Backend for Frontend (BFF)

`apps/web/src/routes/api/` contains SvelteKit server routes:

- `/api/clusters` — Returns stub cluster data when `PUBLIC_ENGINE_HTTP_BASE` is unset
- Future endpoints will proxy to `apps/hub` or embed Rust engine via WASM

### Client-Side Engine Abstraction

`src/lib/engine.ts` exports a `fetch`-based API client:

```typescript
export async function listClusters(fetchFn = fetch): Promise<Cluster[]> {
  const res = await fetchFn('/api/clusters');
  return res.json();
}
```

The `fetchFn` parameter enables dependency injection for testing (pass a mock fetch).

### Component Structure

- `src/routes/` — SvelteKit pages and API routes
- `src/lib/` — Reusable logic and (eventually) components
- `src/lib/**/*.test.ts` — Vitest unit tests
- `tests-e2e/` — Playwright E2E tests

## Testing Strategy

### Unit Tests (Vitest)

- Test pure functions and business logic
- Use dependency injection (`fetchFn` parameter) for testability
- Run with `pnpm test`

### E2E Tests (Playwright)

- Test against a stub server (`tools/devtest/stub-server.mjs`)
- Deterministic fake data (no live K8s cluster required)
- CI runs E2E in a separate job after web tests pass

**Stub server behavior:**
- Returns fixed cluster/namespace data
- Predictable for snapshot testing
- Does NOT connect to hub or engine

## Desktop Integration

**Important:** The desktop app consumes this web build. Changes to the desktop UI must be made here.

Build flow:
1. `apps/desktop/scripts/prepare-frontend.mjs` runs `pnpm run build` in this directory
2. Copies `apps/web/build/` to `apps/desktop/dist/`
3. Tauri packages `dist/` as the native desktop frontend

## Environment Variables

- `PUBLIC_ENGINE_HTTP_BASE` — Base URL for hub server (if set, routes proxy to it)
- When unset, `/api/clusters` returns stub data

## CI Enforcement

CI validates:
- `pnpm test` — Vitest unit tests must pass
- `pnpm e2e` — Playwright E2E tests must pass
- `pnpm lint` — Currently a no-op (TODO: implement real linting)

## Code Conventions

- **TypeScript everywhere** (`lang="ts"` in script blocks)
- **Svelte 5 runes only** (no legacy reactive syntax)
- **Dependency injection** for testability (pass `fetch` as a param)
- **Minimal server logic** (BFF routes are thin proxies or stub providers)

## What's Missing

- Real linting (ESLint + svelte-eslint-parser needed)
- Connection to live hub/engine
- Shared component library (`packages/ui` is a placeholder)
- Web-to-hub feature parity (some features may be hub-only)

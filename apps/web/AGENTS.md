# Agent Guidance — SvelteKit Web App

## Overview

`apps/web` is the SvelteKit 2 web client for Telescope, using Svelte 5 with runes. It serves as:

1. The standalone web UI (SvelteKit static build served by Vite dev, or embedded by hub)
2. The frontend for `apps/desktop` (Tauri packages the built output)

Both modes use the **same `src/lib/api.ts` facade**: Tauri IPC in desktop, Hub HTTP in browser.

## Technology Stack

- **Framework:** SvelteKit 2
- **Svelte version:** Svelte 5 (runes-based, NOT legacy syntax)
- **Build tool:** Vite 6 (`@sveltejs/adapter-static`)
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
# Dev server (port 5173) — browser mode against hub or stub
pnpm -C apps/web dev

# Production static build (output to apps/web/build/)
pnpm -C apps/web build

# Unit tests (Vitest, runs in forked pool)
pnpm -C apps/web test

# E2E tests (Playwright — spins up Vite dev server + stub server)
pnpm -C apps/web e2e

# Install Playwright browsers (required once; CI installs chromium only)
pnpm -C apps/web exec playwright install --with-deps

# Lint (⚠️ runs the build, not a real linter — ESLint not yet configured)
pnpm -C apps/web lint
```

## Key Source Files

### API Facade (`src/lib/api.ts`)

The **primary interface for all Kubernetes data**. Never call Tauri or hub HTTP directly from components — always use this file.

- Checks `isTauri()` at runtime to dispatch to Tauri IPC or the Hub HTTP fallback
- Hub fallback maps Tauri command names to `/api/v1/*` endpoints
- **Write operations are NOT yet mapped to Hub** — the following return `undefined` in web/hub mode:
  `set_namespace`, `scale_resource`, `delete_resource`, `apply_resource`, `rollout_restart`, `rollout_status`, `start_port_forward`, `exec_command`, `helm_rollback`, `get_helm_release_history`, `get_helm_release_values`, `list_containers`, `start_log_stream`, `active_context`, `get_node_metrics`, `get_preference`, `set_preference`
- Hub URL resolved from `window.__TELESCOPE_HUB_URL__` → `PUBLIC_ENGINE_HTTP_BASE` → `http://localhost:3001`

### Type Definitions (`src/lib/tauri-commands.ts`)

TypeScript types mirroring Tauri IPC command signatures: `ClusterContext`, `ResourceEntry`, `ConnectionState`, `ClusterInfo`, `PodMetrics`, `NodeMetricsData`, `CrdInfo`, `HelmRelease`, `LogChunk`.

Also exports `isTauri(): boolean` — checks for `window.__TAURI_INTERNALS__`.

### Reactive State (`src/lib/stores.ts`)

Svelte writable/derived stores for cross-component state:

- `selectedContext` — current kubeconfig context name
- `selectedNamespace` — current namespace
- `namespaces` — available namespaces for the connected cluster
- `connectionState` — `ConnectionState` (Disconnected → Connecting → Syncing → Ready → Degraded/Error)
- `isConnected` — derived: `connectionState.state === 'Ready'`
- `isProduction` — derived: heuristic based on context name
- `clusterServerUrl` — API server URL
- `isAks` — derived: whether URL matches AKS pattern

### Utility Libraries

- `src/lib/azure-utils.ts` — AKS cluster URL detection, Azure identity parsing
- `src/lib/prod-detection.ts` — Production context heuristics
- `src/lib/error-suggestions.ts` — Human-friendly error messages
- `src/lib/version.ts` — App version helpers
- `src/lib/stores/metrics-history.ts` — Rolling CPU/memory sparkline data

## Routes

All pages live in `src/routes/`:

| Route | Description |
|---|---|
| `/` | Root redirect |
| `/overview` | Cluster dashboard: node pool info, resource counts, metrics sparklines |
| `/pods` | Pod list with filters |
| `/pods/[namespace]/[name]` | Pod detail: logs, exec terminal, events |
| `/nodes/[name]` | Node detail: capacity, allocatable, conditions |
| `/events` | Cluster events table |
| `/resources/deployments` | Deployment list |
| `/resources/statefulsets` | StatefulSet list |
| `/resources/daemonsets` | DaemonSet list |
| `/resources/services` | Service list |
| `/resources/configmaps` | ConfigMap list |
| `/resources/ingresses` | Ingress list |
| `/resources/pvcs` | PersistentVolumeClaim list |
| `/resources/jobs` | Job list |
| `/resources/cronjobs` | CronJob list |
| `/resources/secrets` | Secret list |
| `/helm` | Helm releases list |
| `/helm/[namespace]` | Helm release detail: values, history, rollback |
| `/crds` | Custom Resource Definitions list |
| `/crds/[group]/[kind]` | CRD instance browser |
| `/explore` | Generic resource explorer |
| `/create` | Apply YAML manifest (desktop-only write op) |
| `/settings` | User preferences |
| `/clusters` | Cluster context switcher |

## Component Library (`src/lib/components/`)

The app has a substantial in-app component library:

| Component | Purpose |
|---|---|
| `AppHeader.svelte` | Top navigation bar |
| `Sidebar.svelte` | Left nav sidebar with route links |
| `Breadcrumbs.svelte` | Page breadcrumb trail |
| `FilterBar.svelte` | Search/filter input for resource tables |
| `ResourceTable.svelte` | Generic sortable/filterable resource list |
| `PodTable.svelte` | Pod-specific table with status column |
| `EventsTable.svelte` | Kubernetes events table |
| `LogViewer.svelte` | Scrollable log output viewer |
| `ExecTerminal.svelte` | In-browser exec shell terminal |
| `YamlEditor.svelte` | Syntax-highlighted YAML editor |
| `PortForwardDialog.svelte` | Port-forward configuration modal |
| `ScaleDialog.svelte` | Replica count scaling dialog |
| `ConfirmDialog.svelte` | Generic destructive-action confirmation |
| `SearchPalette.svelte` | Command-K global resource search |
| `ContextSwitcher.svelte` | Kubeconfig context dropdown |
| `ConnectionStatus.svelte` | Connection state indicator |
| `Sparkline.svelte` | Mini CPU/memory usage chart |
| `LoadingSkeleton.svelte` | Placeholder loading state |
| `ThemeToggle.svelte` | Light/dark mode toggle |
| `Tabs.svelte` | Tab navigation component |
| `NodePoolHeader.svelte` | AKS node pool metadata display |
| `AzureIdentitySection.svelte` | Azure identity info panel |
| `AksAddons.svelte` | AKS add-on feature display |
| `ShortcutHelp.svelte` | Keyboard shortcut help overlay |
| `ErrorMessage.svelte` | Inline error display |

**Note:** `packages/ui` (root-level shared component library) is still empty. All components live here in `apps/web`.

## Testing Strategy

### Unit Tests (Vitest)

Files: `src/lib/**/*.test.ts`

- Test pure functions: `azure-utils`, `prod-detection`, `error-suggestions`, `version`, `hello`
- Run with: `pnpm -C apps/web test`
- Use dependency injection (`fetchFn` params) for HTTP-dependent logic

### E2E Tests (Playwright)

Files: `tests-e2e/**/*.spec.ts`

- Playwright config: `playwright.config.ts` (ports 4273 web, 4274 stub)
- The E2E suite starts two servers simultaneously:
  1. **Stub server** (`tests-e2e/stub/stub-server.mjs`) — deterministic fake `/api/v1/*` responses
  2. **Vite dev server** — `apps/web` with `PUBLIC_ENGINE_HTTP_BASE` pointed at the stub
- CI installs **chromium only** for E2E
- Tests cover: cluster connect flow, resource pages, settings

## Desktop Integration

The desktop app consumes this web build. Changes to the desktop UI **must be made here**.

Build flow:
1. `apps/desktop/scripts/prepare-frontend.mjs` runs `pnpm run build` here
2. Copies `apps/web/build/` to `apps/desktop/dist/`
3. Tauri packages `dist/` as the native desktop frontend

## Environment Variables

- `PUBLIC_ENGINE_HTTP_BASE` — Hub URL for web/browser mode (SvelteKit `$env/dynamic/public`)
- `window.__TELESCOPE_HUB_URL__` — Runtime override (higher priority than env var)
- Both default to `http://localhost:3001` when unset

## CI Enforcement

CI validates (`ci.yml` `web` job):
- `pnpm -C apps/web test` — Vitest unit tests must pass
- `pnpm -C apps/web build` — Production static build must succeed

CI validates (`ci.yml` `web-e2e` job, depends on `web`):
- `pnpm -C apps/web e2e` — Playwright E2E against stub server

## Code Conventions

- **TypeScript everywhere** (`lang="ts"` in script blocks)
- **Svelte 5 runes only** (no legacy `export let`, no `$:` reactive statements, no `on:` event syntax)
- **All backend calls through `src/lib/api.ts`** — never call Tauri or fetch directly in components
- **Use stores** in `src/lib/stores.ts` for shared state — don't prop-drill context/namespace/connectionState
- **Dependency injection** for testability (pass `fetch` as a param in utility functions)

## What's Missing

- Real JavaScript linting (ESLint + `svelte-eslint-parser` not configured — `lint` script runs build)
- Write-operation hub parity (scale, delete, exec, port-forward, preferences all return `undefined` in web mode)
- Shared component library (`packages/ui` is a placeholder)
- `get_resource_counts` hub endpoint (computed client-side or returns empty)

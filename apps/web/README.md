# Telescope Web

`apps/web` is the shared SvelteKit frontend for Telescope. The same app powers:

- the **desktop client** packaged by Tauri, and
- the **browser/web client** served against `apps/hub`.

## What it does

This app provides the main Kubernetes IDE UI for cluster connection, resource browsing, workload management, logs, exec, port-forwarding, Helm, CRDs, metrics, search, and settings.

## Desktop vs. web mode

The split between desktop and browser mode is handled in [`src/lib/api.ts`](src/lib/api.ts).

- **Desktop / Tauri mode**
  - `api.ts` checks `isTauri()`.
  - When running inside Tauri, it dynamically imports `@tauri-apps/api/core` and uses `invoke(...)` to call Rust commands over Tauri IPC.
  - This is the most complete path today and talks directly to the Rust backend.

- **Web / browser mode**
  - The same exported API functions fall back to HTTP requests against Telescope Hub under `/api/v1`.
  - `HUB_URL` is resolved from `window.__TELESCOPE_HUB_URL__`, `PUBLIC_ENGINE_HTTP_BASE`, or `http://localhost:3001`.
  - Read-oriented operations such as contexts, resources, pods, events, namespaces, logs, Helm releases, metrics, CRDs, search, and audit-backed state use Hub endpoints.
  - Some write operations are still desktop-first and currently return no-op/placeholder behavior in the web fallback until Hub parity is completed.

## Development

From the repository root:

```bash
pnpm install
pnpm -C apps/web dev
pnpm -C apps/web test
pnpm -C apps/web e2e
```

Notes:

- `pnpm -C apps/web dev` starts the SvelteKit/Vite development server.
- `pnpm -C apps/web test` runs unit tests with Vitest.
- `pnpm -C apps/web e2e` runs Playwright end-to-end tests.

## Build

```bash
pnpm -C apps/web build
```

The app uses `@sveltejs/adapter-static` with an `index.html` fallback. Desktop packaging builds this app first and copies the output into `apps/desktop/dist` for Tauri.

## Key directories and files

- `src/routes/` — SvelteKit pages for overview, pods, workloads, services, ingresses, config, Helm, CRDs, nodes, events, settings, and detail screens
- `src/lib/components/` — reusable Svelte UI components such as tables, dialogs, headers, log viewers, and exec/search helpers
- `src/lib/api.ts` — shared frontend API facade that switches between Tauri IPC and Hub HTTP
- `src/lib/stores.ts` — shared Svelte stores for selected context, namespace, connection state, and cluster metadata

## Testing

- **Vitest** is used for unit tests (`pnpm -C apps/web test`)
- **Playwright** is used for end-to-end tests (`pnpm -C apps/web e2e`)

## Related apps

- [`../desktop`](../desktop) packages this frontend into the Tauri desktop app.
- [`../hub`](../hub) provides the HTTP/WebSocket backend used in browser/web mode.

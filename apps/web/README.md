# Telescope Desktop Frontend

`apps/web` is the SvelteKit frontend packaged into the Telescope desktop application.

## What it does

This app provides the main Kubernetes IDE UI for cluster connection, resource browsing, workload management, logs, exec, port-forwarding, Helm, CRDs, metrics, search, and settings.

## Desktop runtime

The desktop integration lives in [`src/lib/api.ts`](src/lib/api.ts).

- When running inside Tauri, it dynamically imports `@tauri-apps/api/core` and uses `invoke(...)` to call Rust commands over Tauri IPC.
- This path talks directly to the Rust backend used by the desktop app.

## Development

From the repository root:

```bash
pnpm install
pnpm -C apps/web e2e:setup   # first-time Playwright browser + Linux deps setup
pnpm -C apps/web dev
pnpm -C apps/web test
pnpm -C apps/web e2e
```

Notes:

- `pnpm -C apps/web dev` starts the SvelteKit/Vite development server.
- `pnpm -C apps/web test` runs unit tests with Vitest.
- `pnpm -C apps/web e2e` runs Playwright end-to-end tests.
- `pnpm -C apps/web e2e:setup` installs the Playwright Chromium runtime and any required Linux system packages.
- `pnpm -C apps/web e2e:docker` runs the browser suite inside the repo's Playwright-ready dev container when local Linux libraries are unavailable.
- If port `4273` is already in use, run E2E with `PLAYWRIGHT_WEB_PORT=4381 pnpm -C apps/web e2e`.

## Build

```bash
pnpm -C apps/web build
```

The app uses `@sveltejs/adapter-static` with an `index.html` fallback. Desktop packaging builds this app first and copies the output into `apps/desktop/dist` for Tauri.

## Key directories and files

- `src/routes/` — SvelteKit pages for overview, pods, workloads, services, ingresses, config, Helm, CRDs, nodes, events, settings, and detail screens
- `src/lib/components/` — reusable Svelte UI components such as tables, dialogs, headers, log viewers, and exec/search helpers
- `src/lib/api.ts` — frontend API facade for Tauri IPC
- `src/lib/stores.ts` — shared Svelte stores for selected context, namespace, connection state, and cluster metadata

## Testing

- **Vitest** is used for unit tests (`pnpm -C apps/web test`)
- **Playwright** is used for end-to-end tests (`pnpm -C apps/web e2e`)

## Related apps

- [`../desktop`](../desktop) packages this frontend into the Tauri desktop app.

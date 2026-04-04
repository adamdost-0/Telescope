# Agent Guidance — SvelteKit Desktop Frontend

## Overview

`apps/web` is the SvelteKit 2 frontend for Telescope, using **Svelte 5 with runes**. It is packaged into the Tauri v2 desktop app — it does **not** run as a standalone web application.

**Transport:** All backend communication goes through **Tauri IPC** (`@tauri-apps/api/core invoke()`). There is no HTTP fallback or hub/browser mode.

## Build Flow

`apps/web` is consumed by the desktop app:
1. `apps/desktop/scripts/prepare-frontend.mjs` runs `pnpm run build` in `apps/web`
2. Copies `apps/web/build/` to `apps/desktop/dist/`
3. Tauri packages `dist/` as the desktop frontend

UI changes must be made here in `apps/web`, not in desktop-specific files.

## Commands

```bash
pnpm -C apps/web dev       # Dev server (requires Tauri for IPC)
pnpm -C apps/web build     # Production build
pnpm -C apps/web test      # Vitest unit tests
pnpm -C apps/web e2e       # Playwright E2E tests
pnpm -C apps/web lint      # Lint (runs build)
```

## Container-First Validation (Required)

Before pushing any frontend change, validate inside the dev container:

```bash
# Full suite (recommended)
./scripts/dev-test.sh

# Or use the container shell for iterative work
./scripts/dev-test.sh shell
# then inside the container:
./scripts/pnpm.sh -C apps/web test
./scripts/pnpm.sh -C apps/web e2e
```

The dev container includes Playwright browsers and all Linux dependencies pre-installed. This eliminates host-specific issues (missing `libatk`, wrong Chromium version, etc.) and mirrors what CI runs.

**Gate rule:** Do not push frontend changes until both `pnpm -C apps/web test` and `pnpm -C apps/web e2e` pass inside the container.

## Iconography Guidance (No Emoji)

- Do not use emojis in UI text, docs, or orchestration logs.
- Prefer plain text labels or the shared icon registry (SVG/monochrome) for visual indicators.
- When updating legacy content, replace emoji checkmarks/warnings with markdown checkboxes or neutral headings.

## Svelte 5 Patterns

**Mandatory:** Use Svelte 5 runes and modern event syntax exclusively.

| Pattern | Usage |
|---------|-------|
| `$props()` | Component props (typed with destructuring) |
| `$state()` | Reactive local state |
| `$derived()` | Computed values |
| `$effect()` | Side effects |
| `onclick` / `onchange` / `onkeydown` | Direct event props (NOT `on:click`) |
| `Snippet` | Typed children in layouts |

**Do NOT use:** Legacy `on:click` event syntax, `$:` reactive statements, or `export let` props.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib/api.ts` | Tauri IPC client layer — all `invoke()` calls to Rust backend |
| `src/lib/stores.ts` | Shared Svelte stores: context, namespace, connection state, AKS detection |
| `src/lib/azure-utils.ts` | AKS URL parsing, cluster detection, Azure Portal deep links |
| `src/lib/preferences.ts` | Production-context patterns, preferred namespace, auto-refresh |
| `src/lib/resource-routing.ts` | Route map between K8s GVKs and Telescope URLs (~240 entries) |
| `src/lib/error-suggestions.ts` | User-friendly error suggestions |
| `src/lib/prod-detection.ts` | Production context detection logic |

## Route Inventory (~39 pages)

### Top-level pages
- `/` — Home / landing
- `/overview` — Cluster overview dashboard
- `/pods` — Pod list
- `/pods/[namespace]/[name]` — Pod detail
- `/nodes` — Node list
- `/nodes/[name]` — Node detail
- `/events` — Cluster events
- `/namespaces` — Namespace management
- `/helm` — Helm release list
- `/helm/[namespace]/[name]` — Helm release detail
- `/crds` — CRD list
- `/crds/[group]/[kind]` — CRD instance list
- `/create` — Create resource
- `/settings` — User settings
- `/azure/node-pools` — AKS node pool management

### Resource list pages (`/resources/*`)
configmaps, cronjobs, daemonsets, deployments, endpointslices, hpas, ingresses, jobs, limitranges, networkpolicies, persistentvolumes, poddisruptionbudgets, priorityclasses, pvcs, resourcequotas, rolebindings, roles, secrets, serviceaccounts, services, statefulsets, storageclasses, webhooks

### Generic resource detail
- `/resources/[kind]/[namespace]/[name]` — Dynamic detail page for any resource type

## Component Inventory (25 components)

| Component | Purpose |
|-----------|---------|
| `AppHeader.svelte` | Top navigation bar |
| `Sidebar.svelte` | Navigation sidebar |
| `Breadcrumbs.svelte` | Breadcrumb navigation |
| `ClusterVitals.svelte` | Cluster health metrics header |
| `ConnectionStatus.svelte` | Cluster connection indicator |
| `ContextSwitcher.svelte` | Kubeconfig context switcher |
| `SearchPalette.svelte` | Global search (⌘K) |
| `ShortcutHelp.svelte` | Keyboard shortcut overlay |
| `ThemeToggle.svelte` | Light/dark theme toggle |
| `FilterBar.svelte` | Namespace/label filtering |
| `ResourceTable.svelte` | Generic resource list table |
| `PodTable.svelte` | Pod-specific list table |
| `EventsTable.svelte` | Events list table |
| `LogViewer.svelte` | Pod log streaming viewer |
| `ExecTerminal.svelte` | Pod exec terminal |
| `PortForwardDialog.svelte` | Port-forward setup dialog |
| `ScaleDialog.svelte` | Replica scale dialog |
| `ConfirmDialog.svelte` | Confirmation modal |
| `YamlEditor.svelte` | YAML editor for create/apply |
| `ErrorMessage.svelte` | Error display |
| `LoadingSkeleton.svelte` | Loading placeholder |
| `Sparkline.svelte` | Inline metric sparklines |
| `Tabs.svelte` | Tab navigation |
| `NodePoolHeader.svelte` | AKS node pool header |
| `AzureIdentitySection.svelte` | AKS identity info |

## Testing

### Container validation (preferred)

Run all frontend tests inside the dev container to match CI:

```bash
./scripts/dev-test.sh          # Full Rust + web validation
# or for frontend-only iteration:
./scripts/dev-test.sh shell
./scripts/pnpm.sh -C apps/web test
./scripts/pnpm.sh -C apps/web e2e
```

### Unit tests (Vitest)
Located alongside source in `src/lib/`:
- `azure-utils.test.ts` — AKS URL parsing, portal links
- `error-suggestions.test.ts` — Error suggestion logic
- `prod-detection.test.ts` — Production context detection
- `version.test.ts` — Version utilities

### E2E tests (Playwright)
Located in `tests-e2e/`:
- `smoke.spec.ts` — Basic navigation and rendering
- `settings.spec.ts` — Settings page interaction

E2E tests run against `tools/devtest/stub-server.mjs` with deterministic fake data (no live K8s cluster).

## Configuration
- `svelte.config.js` — SvelteKit adapter-static (for Tauri packaging)
- `vite.config.ts` — Vite build config
- `playwright.config.ts` — Playwright config
- `tsconfig.json` — TypeScript config

## When to Edit
- **Add a new resource page:** Create route under `src/routes/resources/`, add GVK mapping in `resource-routing.ts`
- **Add a Tauri command:** Add the `invoke()` call in `api.ts`, create UI in route/component, and wire discoverability (`Sidebar.svelte`, `SearchPalette.svelte`, `src/routes/+layout.svelte` shortcuts when applicable)
- **Add/modify backend API exposure:** Keep contracts aligned in `tauri-commands.ts`/feature types, ensure route remains visible under disconnected policy, and add e2e coverage proving navigation + command invocation
- **Add a component:** Place in `src/lib/components/`
- **Modify preferences:** Edit `preferences.ts` and corresponding Tauri commands

# Copilot Instructions — Telescope

## Project

Memory-efficient Kubernetes IDE (AKS-first) with a Tauri desktop app and a SvelteKit web client, both backed by a shared Rust engine. No Electron.

## Monorepo Layout

- **`crates/core`** — Shared Rust domain types (`telescope-core`)
- **`crates/engine`** — Kubernetes engine: watchers, cache, streaming (`telescope-engine`, depends on core)
- **`crates/api`** — API surface layer (`telescope-api`, depends on engine + core)
- **`apps/desktop`** — Tauri 2 desktop shell (`apps/desktop/src-tauri/` for Rust side)
- **`apps/web`** — SvelteKit 2 web client (Svelte 5, Vite 6)
- **`packages/ui`** — Shared UI components (placeholder — currently empty)

Package manager is **pnpm 9.15.4** with `pnpm-workspace.yaml`. The root `package.json` `workspaces` field is vestigial (pnpm ignores it).

## Build & Test Commands

### Rust

```bash
cargo fmt --all -- --check          # Format check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features   # All crate tests
cargo test -p telescope-engine      # Single crate
cargo test -p telescope-core -- version   # Single test by name filter
```

`telescope-desktop` is excluded on Linux CI because Tauri has platform-specific deps. Desktop builds run on Windows/macOS matrix only.

### Web (SvelteKit)

```bash
pnpm -C apps/web test              # Vitest unit tests
pnpm -C apps/web exec vitest run src/lib/hello.test.ts   # Single test file
pnpm -C apps/web dev               # Dev server
pnpm -C apps/web build             # Production build
pnpm -C apps/web e2e               # Playwright E2E (needs browsers installed)
pnpm -C apps/web exec playwright install --with-deps     # Install browsers + OS deps
```

### Desktop (Tauri)

```bash
pnpm -C apps/desktop build         # Debug build (requires Rust + platform SDK)
pnpm -C apps/desktop bundle        # Release bundle
```

### Workspace-wide

```bash
pnpm -r --if-present test          # Run test script in all packages that have one
pnpm -r --if-present lint          # Lint all (note: web/desktop lint scripts are currently no-ops)
```

## CI Enforces

- `cargo fmt -- --check` — Rust formatting
- `cargo clippy -D warnings` — Rust lints (warnings are errors)
- `cargo test --all --all-features` — Rust tests (excluding desktop on Linux)
- `vitest run` — Web unit tests
- `playwright test` — Web E2E (against dev server with stub data)
- Desktop build on Windows + macOS matrix

## Architecture Conventions

### Rust Crates

- Edition 2021, workspace-inherited
- Dependency chain: `api → engine → core` (core has no internal deps)
- Desktop crate is in workspace `members` but excluded from `default-members` for cross-platform CI
- All crates currently scaffold-only — real K8s client integration (kube-rs, gRPC) is M1 work

### SvelteKit Web App

- **Use Svelte 5 runes** (`$props()`, `$state()`, `$derived()`) — not legacy `export let` or `$:` reactive labels
- **Use Svelte 5 event syntax** (`onclick={handler}`) — not legacy `on:click`
- TypeScript throughout (`lang="ts"` in script blocks)
- Pass `fetch` as a parameter for testability (dependency injection pattern in `engine.ts`)
- E2E tests use a deterministic stub server (`tools/devtest/stub-server.mjs`) — no live K8s cluster required
- Unit tests go in `src/**/*.test.ts`, E2E tests go in `tests-e2e/`

### Tauri Desktop

- Tauri v2 (not v1) — config is `tauri.conf.json` inside `src-tauri/`
- Frontend is prepared via `scripts/prepare-frontend.mjs` (copies static HTML to `dist/`)
- Desktop does not yet share the SvelteKit frontend — it uses a standalone `index.html`

## Current State (important context)

The project is at **v0.0.1** — early scaffold phase. Key things to know:

- Rust crates expose only `version()` functions — no K8s client, gRPC, or real engine logic yet
- `apps/web/src/lib/engine.ts` is a fetch wrapper against the SvelteKit BFF, **not** a real Rust engine client
- `/api/clusters` server route returns hardcoded stub data when `PUBLIC_ENGINE_HTTP_BASE` is unset
- Docs in `docs/` describe **target architecture**, not current implementation — treat them as aspirational
- `packages/ui` is empty — no shared components exist yet

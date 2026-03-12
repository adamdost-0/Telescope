# Copilot Instructions — Telescope

## Project

AKS-first Kubernetes IDE with a Tauri desktop app, a shared SvelteKit frontend, and Rust backends. No Electron.

## Stable Repo Shape

- Cargo workspace: `crates/core`, `crates/engine`, `crates/api`, `apps/desktop/src-tauri`, `apps/hub`
- pnpm workspace: `apps/*`, `packages/*`
- `crates/core` — shared domain/state/storage types
- `crates/engine` — real Kubernetes engine code: client, watchers, logs, exec, port-forward, actions, Helm, metrics, CRDs
- `crates/api` — thin facade over engine/core
- `apps/web` — shared SvelteKit frontend
- `apps/desktop` — Tauri v2 shell for that frontend
- `apps/hub` — Axum HTTP/WebSocket service used by browser/web mode
- `packages/ui` exists but is still minimal; do not assume a mature shared component library

## Desktop vs Web

- Desktop and web share the `apps/web` UI.
- `apps/desktop/scripts/prepare-frontend.mjs` builds `apps/web` and copies its output into `apps/desktop/dist` for Tauri.
- In desktop/Tauri, `apps/web/src/lib/api.ts` talks to Rust through Tauri IPC.
- In browser/web mode, the same API layer falls back to Hub HTTP endpoints under `/api/v1`.

## Current Implementation Reality

- This repository is **not scaffold-only** anymore; real engine, desktop, and UI functionality exists.
- Desktop is the most complete client today.
- Browser/Hub mode exists, but some write operations are still deferred outside Tauri; verify browser-only behavior in code before assuming parity with desktop.
- `packages/ui` is still lightweight compared with the app-local UI in `apps/web`.

## Verified Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features

pnpm -r --if-present lint
pnpm -r --if-present test
pnpm -C apps/web build
pnpm -C apps/web test
pnpm -C apps/web e2e
pnpm -C apps/web exec playwright install --with-deps
pnpm -C apps/desktop build
pnpm -C apps/desktop bundle
```

- Desktop Rust/Tauri build steps are excluded from Linux Rust CI and run separately on Windows/macOS.
- Prefer repo-defined commands and workflow commands over inventing new ones.

## Guidance

- Rust dependency direction is `api → engine → core`.
- Use Svelte 5 runes and modern event syntax in `apps/web`.
- Treat `docs/` as mixed-source documentation: some files describe the current implementation, while others are aspirational or partially stale. Cross-check claims against code and `.github/workflows/*.yml` before relying on them.

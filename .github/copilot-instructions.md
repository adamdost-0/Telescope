# Copilot Instructions — Telescope

## Project

AKS-first Kubernetes IDE with a Tauri desktop app, a packaged SvelteKit frontend, and Rust backends. No Electron.

## Stable Repo Shape

- Cargo workspace: `crates/core`, `crates/engine`, `crates/api`, `apps/desktop/src-tauri`
- pnpm workspace: `apps/*`, `packages/*`
- `crates/core` — shared domain/state/storage types
- `crates/engine` — real Kubernetes engine code: client, watchers, logs, exec, port-forward, actions, Helm, metrics, CRDs
- `crates/api` — thin facade over engine/core
- `apps/web` — SvelteKit frontend packaged into the desktop app
- `apps/desktop` — Tauri v2 shell for that frontend
- `packages/ui` exists but is still minimal; do not assume a mature shared component library

## Desktop Frontend

- `apps/web` contains the UI used by the desktop app.
- `apps/desktop/scripts/prepare-frontend.mjs` builds `apps/web` and copies its output into `apps/desktop/dist` for Tauri.
- In desktop/Tauri, `apps/web/src/lib/api.ts` talks to Rust through Tauri IPC.

## Current Implementation Reality

- This repository is **not scaffold-only** anymore; real engine, desktop, and UI functionality exists.
- Desktop is the supported client surface.
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

## Delivery and Release Behavior

- After a change set is complete and validated, push the branch upstream instead of leaving work only in the local checkout.
- Then create and push a release tag that matches `v*` so `.github/workflows/release.yml` triggers a fresh release/build run.
- Unless the user requests a specific version, continue the existing tag sequence with the next sensible SemVer-style release tag.

## Guidance

- Rust dependency direction is `api → engine → core`.
- Use Svelte 5 runes and modern event syntax in `apps/web`.
- Treat `docs/` as mixed-source documentation: some files describe the current implementation, while others are aspirational or partially stale. Cross-check claims against code and `.github/workflows/*.yml` before relying on them.

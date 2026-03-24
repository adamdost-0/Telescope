# Copilot Instructions — Telescope

## Project

AKS-first Kubernetes IDE with a Tauri v2 desktop app, a packaged SvelteKit frontend, and Rust backends. Desktop-only — no Electron, no browser/hub mode.

## Stable Repo Shape

- Cargo workspace: `crates/core`, `crates/engine`, `crates/azure`, `apps/desktop/src-tauri`
- pnpm workspace: `apps/*`, `packages/*`
- `crates/core` — shared domain types, SQLite-backed ResourceStore, ConnectionState machine
- `crates/engine` — Kubernetes engine: client, 28+ resource watchers, actions, Helm, logs, exec, port-forward, metrics, node ops, CRDs, dynamic resources, secrets, namespaces, audit
- `crates/azure` — Azure ARM client, AKS management-plane operations (cluster, node pools, upgrades, maintenance, identity resolution)
- `apps/web` — SvelteKit frontend packaged into the desktop app
- `apps/desktop` — Tauri v2 shell for that frontend
- `packages/ui` exists but is still minimal; do not assume a mature shared component library

## Desktop Frontend

- `apps/web` contains the UI used by the desktop app.
- `apps/desktop/scripts/prepare-frontend.mjs` builds `apps/web` and copies its output into `apps/desktop/dist` for Tauri.
- In desktop/Tauri, `apps/web/src/lib/api.ts` talks to Rust through Tauri IPC. There is no HTTP fallback.

## Current Implementation Reality

- This is a **shipped v1.0.0 desktop application** with substantial Kubernetes and Azure ARM functionality.
- Desktop is the only supported client surface.
- `packages/ui` is still lightweight compared with the app-local UI in `apps/web`.

## Iconography and Tone Guidance (No Emoji)

- Generated content (docs, prompts, orchestration logs, UI strings) should avoid emojis.
- Prefer plain text labels or the standardized icon registry (monochrome/SVG) when a visual indicator is needed.
- When converting existing materials, replace emoji checkmarks/warnings with markdown checkboxes (`- [x]`) or neutral headings.

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

- Rust dependency direction is `engine → core`, `azure → core`, `desktop → engine + azure + core`.
- Use Svelte 5 runes and modern event syntax in `apps/web`.
- Treat `docs/` as mixed-source documentation: some files describe the current implementation, while others are aspirational or partially stale. Cross-check claims against code and `.github/workflows/*.yml` before relying on them.

# Agent Skills — Telescope

This file defines the specialist capabilities that AI agents should bring when working in this repository. Use it alongside `AGENTS.md` (and the nested `AGENTS.md` files) to match work to the right agent profile.

## Why Specialist Routing Matters

Telescope spans four different technology domains that share data but use different tools and patterns:

- Rust async systems programming (crates)
- Tauri IPC + Kubernetes engine (desktop)
- Axum HTTP services (hub)
- SvelteKit 5 UI with runes (web)

A "general" agent can make mistakes by applying wrong idioms across these boundaries (e.g., using legacy `export let` in Svelte 5, wrong Tauri command registration patterns, missing audit log calls for destructive ops). Specialist routing prevents that.

---

## Skill Profiles

### `rust-engine` — Rust Kubernetes Engine Expert

**Primary paths:** `crates/core/`, `crates/engine/`, `crates/api/`

**Required knowledge:**
- Rust async with Tokio (`tokio::spawn`, `RwLock`, `Mutex`, `watch` channels)
- kube-rs: `kube::Client`, `kube::runtime::watcher`, `Api<K>`, `ResourceExt`
- `ResourceStore` SQLite schema (GVK + namespace + name primary key, WAL mode)
- `ResourceWatcher` concurrency model (Arc<Mutex<ResourceStore>>, `Semaphore` for LIST ops)
- `thiserror` for `EngineError`; `anyhow` in application code
- Watched GVK list (13 types); secrets are never cached — always fetched on-demand
- `AuditEntry` structure and `log_audit()` call requirements for destructive ops
- `ConnectionState` state machine transitions

**Validation commands:**
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

**Watch out for:**
- Never put HTTP handler logic in `crates/` — that belongs in `apps/hub`
- Never cache secrets in `ResourceStore` — they must be fetched on-demand
- Always call `log_audit()` in destructive engine functions (delete, scale, apply, rollout)
- `ResourceStore` uses `std::sync::Mutex` (not `tokio::sync`) due to `rusqlite` `!Sync`

---

### `hub-api` — Axum Hub Server Expert

**Primary paths:** `apps/hub/src/`

**Required knowledge:**
- Axum 0.8: `Router`, `State<Arc<HubState>>`, `Extension<AuthUser>`, `Json<T>`, `StatusCode`
- All `/api/v1/*` routes and their query parameter shapes (see `apps/hub/AGENTS.md`)
- `HubState` structure: `Arc<Mutex<ResourceStore>>`, `Arc<RwLock<ConnectionState>>`, `watch_handle`, `active_context`, `active_namespace`, `audit_log_path`
- `auth_middleware`: OIDC disabled = anonymous user; enabled = JWT claim decode (no sig verify yet)
- `AuthUser` extraction via `Extension`; `actor` field used in audit entries
- All env vars: `PORT` (3001), `DB_PATH`, `AUDIT_PATH`, `OIDC_ENABLED`, `OIDC_ISSUER_URL`, `OIDC_CLIENT_ID`
- Route registration in `main.rs` under `.nest("/api/v1", api_routes)`
- Error response type: `ApiResult<T>` = `Result<Json<T>, (StatusCode, Json<ErrorResponse>)>`

**Validation commands:**
```bash
cargo clippy -p telescope-hub -- -D warnings
cargo test -p telescope-hub
cargo run -p telescope-hub   # manual smoke test
```

**Watch out for:**
- Routes must go under `/api/v1/` prefix — not `/api/`
- Write operations (scale, delete, exec) are not yet exposed in hub — confirm before adding
- `CorsLayer::permissive()` is intentional for now; don't change without security review
- `auth_middleware` must be applied to API routes; `/healthz` and `/auth/*` are explicitly unprotected

---

### `tauri-desktop` — Tauri 2 + Desktop Rust Expert

**Primary paths:** `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/tauri.conf.json`

**Required knowledge:**
- Tauri 2 command registration: `#[tauri::command]`, `tauri::Builder::invoke_handler(tauri::generate_handler![...])`
- `AppState` structure and its `std::sync::Mutex<ResourceStore>` pattern
- All 35+ Tauri commands and which are read-only vs. write/destructive (see `apps/desktop/AGENTS.md`)
- `connect_to_context` flow: create client → clear store → spawn `ResourceWatcher` → track handle
- `disconnect` flow: abort handle → clear all 13 GVK caches
- `set_namespace` flow: clear caches → restart watchers with new namespace filter
- Emitting Tauri events: `app.emit("log-chunk", payload)` for streaming log output
- Desktop audit actor: `"desktop-user@local"`
- `prepare-frontend.mjs` build flow — UI is never in `apps/desktop/src/`
- Tauri v2 config format (not v1)

**Validation commands:**
```bash
pnpm -C apps/desktop build          # debug build
pnpm -C apps/desktop tauri dev      # dev mode (requires frontend ready)
```

**Watch out for:**
- ALL UI changes must go in `apps/web` — never add UI files to `apps/desktop`
- After adding a Tauri command in Rust, also add it to the `generate_handler!` list AND the `webFallback` switch in `apps/web/src/lib/api.ts`
- Write commands must call `log_audit()` — see existing `delete_resource` handler as reference
- Desktop crate is excluded from Linux Rust CI (`--exclude telescope-desktop`)

---

### `svelte-web` — SvelteKit 5 + TypeScript UI Expert

**Primary paths:** `apps/web/src/`

**Required knowledge:**
- Svelte 5 runes: `$props()`, `$state()`, `$derived()`, `$effect()` — never `export let` or `$:`
- Modern event syntax: `onclick={handler}` — never `on:click`
- All API calls go through `src/lib/api.ts` — never call Tauri or `fetch` directly in components
- `stores.ts` writable/derived stores: `selectedContext`, `selectedNamespace`, `namespaces`, `connectionState`, `isConnected`, `isProduction`, `clusterServerUrl`, `isAks`
- `tauri-commands.ts` type definitions — match these to Tauri command signatures exactly
- Hub fallback URL resolution: `window.__TELESCOPE_HUB_URL__` → `PUBLIC_ENGINE_HTTP_BASE` → `http://localhost:3001`
- Write-op gap: `set_namespace`, `scale_resource`, `delete_resource`, `apply_resource`, `exec_command`, `start_port_forward`, `start_log_stream`, `helm_rollback`, `get_preference`, `set_preference`, `get_node_metrics` return `undefined` in web mode
- E2E stub server: `tests-e2e/stub/stub-server.mjs` (not `tools/devtest/`)
- `PUBLIC_ENGINE_HTTP_BASE` env var for `pnpm -C apps/web dev` against a live hub
- Static adapter — this is a fully static SvelteKit build (no SSR server routes)

**Validation commands:**
```bash
pnpm -C apps/web test          # Vitest unit tests
pnpm -C apps/web build         # static build
pnpm -C apps/web e2e           # Playwright E2E
```

**Watch out for:**
- `lint` script runs the build, not a real linter — ESLint is not configured
- CI only installs Chromium for Playwright (not all browsers)
- `packages/ui` is empty — build all components in `apps/web/src/lib/components/`
- After adding a new hub endpoint, update `webFallback()` in `api.ts` to map the Tauri command to the new route

---

### `ci-workflow` — GitHub Actions CI/CD Expert

**Primary paths:** `.github/workflows/`

**Required knowledge:**
- Five workflows: `ci.yml`, `integration.yml`, `build-desktop.yml`, `release.yml`, and `build-desktop.yml`
- `ci.yml` runs four jobs: `rust`, `web`, `web-e2e`, `desktop-build` (Win + macOS matrix)
- `integration.yml` uses k3d; triggered on engine/core path changes or manually
- `build-desktop.yml` produces artifact uploads (14-day retention); Windows release bundle
- `release.yml` stamps versions, builds release bundle, creates GitHub Release on `v*` tags
- Preferred action versions: `actions/checkout@v4`, `setup-node@v4` (node 22), `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2`, `upload-artifact@v4`, `softprops/action-gh-release@v2`
- Concurrency: `cancel-in-progress: true` per branch
- `permissions: { contents: read }` default; release workflow needs `contents: write`
- Desktop excluded from Linux Rust CI (`--exclude telescope-desktop`)
- E2E installs chromium only: `playwright install --with-deps chromium`

**Watch out for:**
- `pnpm -r --if-present lint` is NOT in the current CI — don't assume it runs
- `pnpm -C apps/web build` IS in the `web` job (not just tests)
- Desktop CI does a debug build (`pnpm -C apps/desktop build`); release uses `bundle`
- `release.yml` stamps `RELEASE_VERSION` into all `Cargo.toml` manifests — don't manually set versions in those files before tagging

---

## Skill-to-Path Quick Reference

| File/Folder | Best Skill Profile |
|---|---|
| `crates/core/src/store.rs` | `rust-engine` |
| `crates/core/src/connection.rs` | `rust-engine` |
| `crates/engine/src/watcher.rs` | `rust-engine` |
| `crates/engine/src/actions.rs` | `rust-engine` |
| `crates/engine/src/helm.rs` | `rust-engine` |
| `crates/engine/src/audit.rs` | `rust-engine` |
| `apps/hub/src/main.rs` | `hub-api` |
| `apps/hub/src/routes.rs` | `hub-api` |
| `apps/hub/src/auth.rs` | `hub-api` |
| `apps/hub/src/state.rs` | `hub-api` |
| `apps/desktop/src-tauri/src/main.rs` | `tauri-desktop` |
| `apps/desktop/src-tauri/tauri.conf.json` | `tauri-desktop` |
| `apps/web/src/lib/api.ts` | `svelte-web` + `tauri-desktop` |
| `apps/web/src/lib/tauri-commands.ts` | `svelte-web` + `tauri-desktop` |
| `apps/web/src/lib/stores.ts` | `svelte-web` |
| `apps/web/src/lib/components/*.svelte` | `svelte-web` |
| `apps/web/src/routes/**` | `svelte-web` |
| `apps/web/tests-e2e/**` | `svelte-web` |
| `.github/workflows/*.yml` | `ci-workflow` |

## Cross-Skill Changes

Some changes require coordination across multiple skill profiles:

| Task | Skills Needed |
|---|---|
| Add a new Tauri command | `rust-engine` (engine logic) + `tauri-desktop` (command handler) + `svelte-web` (api.ts + hub fallback) |
| Add a hub API endpoint | `hub-api` (route + handler) + `svelte-web` (webFallback in api.ts) |
| Add a new watched GVK | `rust-engine` (watcher.rs + ALL_WATCHED_GVKS) + `tauri-desktop` (clear_all_resources) + `hub-api` (state.rs ALL_WATCHED_GVKS) |
| Add a new write action | `rust-engine` (actions.rs) + `tauri-desktop` (command + audit) + `svelte-web` (api.ts) |
| Change ResourceEntry schema | `rust-engine` (store.rs schema migration) + `svelte-web` (tauri-commands.ts types) |
| Add a new UI page | `svelte-web` (route + component) — no Rust changes needed if data is already exposed |

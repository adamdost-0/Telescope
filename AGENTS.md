# Agent Guidance — Telescope

## Source-of-Truth Precedence

When AI agents work in this repository, trust sources in this order:

1. **Actual code and CI behavior** — the ground truth of what exists and runs
2. **`AGENTS.md` files** (this file + nested guides) — practical guidance for working with the repo
3. **`.github/copilot-instructions.md`** — global invariants and project context
4. **Documentation in `docs/`** — aspirational architecture and design targets

**Key rule:** When docs contradict code or CI, the code wins. `docs/` describes the **target architecture**, not always the current implementation.

## Path Ownership and Specialist Routing

Route work to the right specialist based on path and task type. Consult `SKILLS.md` for agent capability profiles.

| Path Pattern | Specialist | Guidance File |
|---|---|---|
| `crates/**` | Rust (core/engine/api) | `crates/AGENTS.md` |
| `apps/web/**` | SvelteKit (Svelte 5) | `apps/web/AGENTS.md` |
| `apps/desktop/**` | Tauri 2 desktop | `apps/desktop/AGENTS.md` |
| `apps/hub/**` | Axum server (hub) | `apps/hub/AGENTS.md` |
| `.github/workflows/**` | CI/CD pipelines | `.github/workflows/AGENTS.md` |
| `docs/**` | Architecture docs | `docs/AGENTS.md` |
| Root config files | General context | (this file) |

## Current Implementation Reality

Telescope is a functioning Kubernetes IDE — not a scaffold. Key components are all real:

- **`crates/core`** — `ResourceStore` (SQLite via rusqlite), `ResourceEntry`, `ConnectionState`, `ResourceWatcher`
- **`crates/engine`** — Full kube-rs integration: watchers, log streaming, exec, port-forward, secrets, Helm, metrics, CRDs, actions (scale, delete, apply, rollout)
- **`apps/hub`** — Axum 0.8 HTTP server at port 3001 with 15+ REST endpoints under `/api/v1/`, WebSocket, OIDC scaffolding
- **`apps/web`** — SvelteKit 2/Svelte 5 UI with 20+ pages, 20+ components, a unified `api.ts` facade, and full Playwright E2E coverage
- **`apps/desktop`** — Tauri 2 desktop shell with 35+ IPC commands fully backed by the engine crates

**Desktop is the most complete client.** Hub/web mode supports all read operations but defers most write operations (exec, port-forward, log streaming, scale, delete, apply, Helm rollback) — see `apps/web/src/lib/api.ts` `webFallback` for the authoritative gap list.

**What's NOT yet production-ready:**
- OIDC authentication in hub (scaffolded, no JWT signature validation)
- `packages/ui` shared component library (empty placeholder)
- Hub CD pipeline / Helm chart / Kubernetes manifests
- Linux desktop CI builds (GTK/WebKit deps excluded)

## Core Architecture: ResourceStore + ResourceWatcher

Both desktop and hub share this same pattern:

1. `ResourceStore` (SQLite via `rusqlite`) holds all cached Kubernetes resources as JSON blobs keyed by `(gvk, namespace, name)`.
2. `ResourceWatcher` (kube-rs watch streams) runs background tasks, writes to `ResourceStore`, and emits `ConnectionState` events.
3. On connect: watchers start for 13 standard GVKs (Pod, Event, Node, Deployment, StatefulSet, DaemonSet, ReplicaSet, Service, ConfigMap, Job, CronJob, Ingress, PVC).
4. On disconnect/reconnect: the store is cleared and watchers restart.

Secrets are NOT cached in the store — they are fetched on-demand directly from the Kubernetes API.

## Cross-Cutting Commands

**Rust (workspace level):**
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

**Web (SvelteKit):**
```bash
pnpm -C apps/web test              # Vitest unit tests
pnpm -C apps/web e2e               # Playwright E2E (spins up Vite dev + stub server)
pnpm -C apps/web dev               # Dev server (port 5173)
pnpm -C apps/web build             # Production static build
```

**Desktop (Tauri):**
```bash
pnpm -C apps/desktop build         # Debug build
pnpm -C apps/desktop bundle        # Release bundle (installer)
pnpm -C apps/desktop tauri dev     # Dev mode with hot reload
```

**Hub (Axum server):**
```bash
cargo run -p telescope-hub         # Run locally (port 3001)
cargo test -p telescope-hub        # Run unit tests
```

**Engine integration (k3d required):**
```bash
cargo test -p telescope-engine --test integration_k3d -- --nocapture
```

**Workspace-wide:**
```bash
pnpm install                       # Install/sync all pnpm dependencies
pnpm -r --if-present test          # Run test script in all pnpm packages
```

## Desktop Frontend Build Flow

**Important:** `apps/desktop` does NOT maintain its own frontend source. It consumes the built `apps/web` output:

1. `apps/desktop/scripts/prepare-frontend.mjs` runs `pnpm run build` in `apps/web`
2. Copies `apps/web/build/` to `apps/desktop/dist/`
3. Tauri config points `frontendDist` to `./dist`

**All UI changes must be made in `apps/web`**, not in desktop-specific files.

## CI Workflows Summary

| Workflow | Trigger | What It Does |
|---|---|---|
| `ci.yml` | PR / push to main | Rust fmt+clippy+test, web tests+build, web E2E, desktop debug build (Win/macOS) |
| `integration.yml` | Push to main (engine/core paths), manual | Real k3d cluster engine integration tests |
| `build-desktop.yml` | Push to main (crates/apps paths), manual | Full release desktop bundle + artifact upload |
| `release.yml` | Push `v*` tag | Stamps versions, release bundle, GitHub Release |

## Nested Guidance Files

- **`crates/AGENTS.md`** — Rust workspace: core types, engine modules, ResourceStore/Watcher architecture
- **`apps/web/AGENTS.md`** — SvelteKit web client: Svelte 5 patterns, api.ts facade, routing, testing
- **`apps/desktop/AGENTS.md`** — Tauri desktop: Tauri command list, AppState, IPC patterns
- **`apps/hub/AGENTS.md`** — Axum hub server: exact API routes, env vars, auth, SQLite, audit
- **`.github/workflows/AGENTS.md`** — CI pipelines: what's enforced, workflows, how to extend
- **`docs/AGENTS.md`** — Documentation: what's aspirational vs. current
- **`SKILLS.md`** — Agent capability profiles for Copilot task routing

## Working in This Repo

1. **Check CI first:** `.github/workflows/ci.yml` defines what's enforced on every PR
2. **Run local validations before and after changes:**
   - Rust: `cargo fmt --all && cargo clippy ... && cargo test ...`
   - Web: `pnpm -C apps/web test && pnpm -C apps/web build`
3. **Desktop UI changes go in `apps/web`, not `apps/desktop`**
4. **Hub and desktop share engine crates** — changes to `crates/engine` affect both clients
5. **Write ops in hub mode are not fully implemented** — check `webFallback()` in `api.ts` before claiming feature parity
6. **Update nested AGENTS.md** when making structural changes to a subsystem
7. **After a completed, validated change set:** push the branch, then create and push a `v*` release tag to trigger `release.yml`

## Release Tagging Policy

- `release.yml` triggers on any pushed tag matching `v*`.
- The workflow stamps `RELEASE_VERSION` into all Cargo manifests and `tauri.conf.json`, then builds and publishes Windows/macOS release artifacts.
- Unless specified by the user, continue the existing SemVer-style tag sequence.
- Agent delivery order: finish → validate → push branch → push `v*` tag.

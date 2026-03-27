# Agent Guidance — Telescope

## Source-of-Truth Precedence

When working in this repository, trust sources in this order:

1. **Actual code and CI behavior** — the ground truth of what exists and ships
2. **`AGENTS.md` files** (this file plus nested guides) — practical repo-specific workflow guidance
3. **`.github/copilot-instructions.md`** — global invariants and project context
4. **Documentation in `docs/`** — architecture and roadmap material that can lag implementation

**Key rule:** When docs disagree with code or CI, the code wins.

## Product Snapshot (v1.0.0)

Telescope is a **desktop-only Tauri Kubernetes IDE**.

- **Frontend:** SvelteKit app in `apps/web`, packaged into the desktop app by Tauri
- **Desktop shell:** `apps/desktop` and `apps/desktop/src-tauri`
- **Rust crates:** `crates/core`, `crates/engine`, `crates/azure`
- **Removed:** `crates/api` and the discontinued non-desktop delivery stack are no longer part of the active product shape
- **Kubernetes coverage:** 28+ watched Kubernetes resource types across the desktop cache and UI
- **Desktop IPC surface:** 60+ Tauri commands (currently 66 in `apps/desktop/src-tauri/src/main.rs`)
- **Azure support:** Native Azure ARM management-plane operations for AKS, including node pool CRUD, cluster lifecycle, upgrade flows, maintenance config visibility, and multi-cloud targeting

## Stable Repo Shape

### Cargo workspace
- `crates/core` — shared domain, state, persistence, and resource store types
- `crates/engine` — Kubernetes client, watchers, logs, exec, port-forward, actions, Helm, metrics, CRDs, audit support
- `crates/azure` — Azure ARM client and AKS management-plane logic
- `apps/desktop/src-tauri` — Tauri application crate and desktop command surface

### pnpm workspace
- `apps/web` — SvelteKit frontend source for the desktop UI
- `apps/desktop` — Tauri packaging/build wrapper that prepares and bundles `apps/web`
- `packages/*` — shared workspace packages (currently lightweight)

## Path Ownership and Specialist Routing

| Path Pattern | Specialist | Guidance File |
|--------------|-----------|---------------|
| `crates/**` | Rust (core / engine / azure) | `crates/AGENTS.md` |
| `apps/web/**` | SvelteKit (Svelte 5) | `apps/web/AGENTS.md` |
| `apps/desktop/**` | Tauri 2 desktop | `apps/desktop/AGENTS.md` |
| `.github/workflows/**` | CI/CD pipelines | `.github/workflows/AGENTS.md` |
| `docs/**` | Architecture docs | `docs/AGENTS.md` |
| Root config files | General repo context | (this file) |

## Current Implementation Reality

What is real today:
- Desktop is the only shipped runtime and the primary user experience.
- `apps/web` is not a standalone product mode; it is the UI source consumed by the Tauri desktop shell.
- `crates/engine` contains real watch-driven Kubernetes functionality with broad resource coverage.
- `crates/azure` contains real Azure ARM logic for AKS cluster operations.
- CI validates Rust workspace quality plus frontend tests/builds and desktop builds on macOS/Windows.
- **Iconography:** Follow the no-emoji policy for UI, docs, and orchestration. Use plain text labels or the standard icon registry (when available) instead of inline emojis.

What is not true anymore:
- There is no supported non-desktop runtime.
- `crates/api` is not part of the workspace.
- Agents should not plan work around removed browser-only routes or HTTP façade assumptions as if they are active product architecture.

## Verified Commands

### Rust (CI-enforced)
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

### Frontend
```bash
pnpm -C apps/web test
pnpm -C apps/web build
pnpm -C apps/web e2e
pnpm -C apps/web exec playwright install --with-deps chromium
```

### Desktop
```bash
pnpm -C apps/desktop dev
pnpm -C apps/desktop build
pnpm -C apps/desktop bundle
```

### Workspace-wide
```bash
pnpm -r --if-present lint
pnpm -r --if-present test
pnpm install
```

## Desktop Frontend Build Flow

`apps/desktop` does not maintain a separate frontend implementation.

1. `apps/desktop/scripts/prepare-frontend.mjs` runs the production build in `apps/web`
2. The built frontend is copied into `apps/desktop/dist`
3. Tauri packages that output as the shipped desktop UI

**Rule:** If the desktop UI changes, the implementation almost always belongs in `apps/web`.

## API-to-Desktop Visibility Workflow (Required)

When adding or changing an API surface (Rust backend API, Tauri command, or frontend IPC wrapper), treat the work as incomplete until all layers below are wired:

1. Add/adjust the desktop command handler and register it in `apps/desktop/src-tauri/src/main.rs` (`generate_handler![]`).
2. Expose typed frontend IPC wrappers in `apps/web/src/lib/api.ts` and keep feature contracts in sync (`apps/web/src/lib/tauri-commands.ts` and related `src/lib/*` guards/types).
3. Wire a visible UI path in `apps/web` (route/component plus discoverability in `Sidebar.svelte`, `SearchPalette.svelte`, and shortcut mapping in `src/routes/+layout.svelte` when applicable).
4. Preserve intended disconnected behavior (route availability vs. action disablement) instead of hiding features.
5. Add regression coverage for command wiring and discoverability (unit + Playwright e2e).

Definition of done: a user can find and execute the feature from the desktop UI; backend-only API additions are not considered complete.

## Architecture Notes

- Dependency direction is effectively **desktop app → (`core`, `engine`, `azure`)** with `engine` depending on `core`.
- `crates/azure` owns Azure cloud selection and ARM endpoint handling for Commercial, Government, Secret, and Top Secret environments.
- `apps/desktop/src-tauri/src/main.rs` defines the desktop command surface and the watched GVK list used for cache lifecycle management.
- `crates/engine/src/watcher.rs` is the source of truth for Kubernetes watcher coverage.
- `packages/ui` exists but remains lightweight; do not assume a mature shared component library.

## Working in This Repo

1. **Check CI first:** `.github/workflows/ci.yml` defines what is actually enforced.
2. **Prefer desktop-focused validation:** verify Rust workspace checks, frontend tests/build, and desktop packaging commands relevant to your change.
3. **Do not reintroduce removed architecture:** avoid adding new hub/browser assumptions to docs, plans, or code unless explicitly requested.
4. **Cross-check docs with code:** `docs/` can still contain aspirational material.
5. **Update nested `AGENTS.md` files** when subsystem structure or workflow expectations change significantly.
6. **After a completed, validated change set:** commit it, push upstream, then create a `v*` tag if the task specifically requires release automation.

## Release Behavior

- Release automation is triggered by tags matching `v*` via `.github/workflows/release.yml`.
- Desktop builds run separately from Linux Rust CI because Tauri platform dependencies differ by OS.
- Unless the user specifies otherwise, continue the existing SemVer-style release sequence.

## Quick Heuristics for Agents

- Need Kubernetes resource logic? Start in `crates/engine`.
- Need Azure AKS lifecycle or node pool work? Start in `crates/azure`.
- Need UI changes? Start in `apps/web`.
- Need desktop IPC or packaging changes? Start in `apps/desktop/src-tauri` or `apps/desktop`.
- Need to verify current capabilities? Prefer code and CI over roadmap docs.

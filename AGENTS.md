# Agent Guidance — Telescope

## Source-of-Truth Precedence

When AI agents work in this repository, trust sources in this order:

1. **Actual code and CI behavior** — the ground truth of what exists and runs
2. **`AGENTS.md` files** (this file + nested guides) — practical guidance for working with the repo
3. **`.github/copilot-instructions.md`** — global invariants and project context
4. **Documentation in `docs/`** — aspirational architecture and design targets

**Key rule:** When docs contradict code or CI, the code wins. Documentation in `docs/` describes the **target architecture**, not always the current implementation.

## Path Ownership and Specialist Routing

Route work to the right specialist based on path and task type:

| Path Pattern | Specialist | Guidance File |
|--------------|-----------|---------------|
| `crates/**` | Rust (core/engine/api) | `crates/AGENTS.md` |
| `apps/web/**` | SvelteKit (Svelte 5) | `apps/web/AGENTS.md` |
| `apps/desktop/**` | Tauri 2 desktop | `apps/desktop/AGENTS.md` |
| `apps/hub/**` | Axum server (hub) | `apps/hub/AGENTS.md` |
| `.github/workflows/**` | CI/CD pipelines | `.github/workflows/AGENTS.md` |
| `docs/**` | Architecture docs | `docs/AGENTS.md` |
| Root config files | General context | (this file) |

## Aspirational vs. Current Reality

Many documents in `docs/` describe the **target** system, not the current scaffold:

- `docs/ARCHITECTURE.md`, `docs/PRD.md`, `docs/ROADMAP.md` — describe vision and goals
- `docs/TESTING.md`, `docs/SECURITY.md` — partially aspirational; not all practices are enforced yet
- `docs/SMOKE_TEST.md`, `docs/TEST_PLAN.md` — planned testing approach, not all implemented

**Current state (v0.0.1):**
- Rust crates have real implementations but limited functionality
- `apps/hub` is a working Axum server with partial OIDC scaffolding (dev-only, not production-ready)
- `apps/web` is a functional SvelteKit app with stub data and E2E tests
- `apps/desktop` packages the built `apps/web` frontend via Tauri 2
- CI enforces: Rust fmt/clippy/test (excluding desktop on Linux), web tests + E2E, desktop builds on Win/macOS

**What's NOT yet real:**
- Production-grade authentication in hub (OIDC is scaffolded, no signature validation)
- gRPC or advanced engine features
- `packages/ui` shared component library (currently empty)
- Full feature parity between web and hub

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
pnpm -C apps/web e2e               # Playwright E2E
pnpm -C apps/web dev               # Dev server
pnpm -C apps/web build             # Production build
```

**Desktop (Tauri):**
```bash
pnpm -C apps/desktop build         # Debug build
pnpm -C apps/desktop bundle        # Release bundle
```

**Hub (Axum server):**
```bash
cargo run -p telescope-hub         # Run locally
cargo test -p telescope-hub        # Run tests
```

**Workspace-wide:**
```bash
pnpm -r --if-present test          # Run all package tests
pnpm install                       # Install/sync all dependencies
```

## Desktop Frontend Build Flow

**Important:** `apps/desktop` does NOT maintain its own frontend. It consumes the built `apps/web` output:

1. `apps/desktop/scripts/prepare-frontend.mjs` runs `pnpm run build` in `apps/web`
2. Copies `apps/web/build/` to `apps/desktop/dist/`
3. Tauri packages `dist/` as the desktop frontend

Changes to the desktop UI must be made in `apps/web`, not in desktop-specific files.

## Nested Guidance Files

For detailed context on each area, consult the nested `AGENTS.md` files:

- **`crates/AGENTS.md`** — Rust workspace: core types, engine, API layer
- **`apps/web/AGENTS.md`** — SvelteKit web client: Svelte 5 patterns, routing, testing
- **`apps/desktop/AGENTS.md`** — Tauri desktop app: packaging, platform specifics
- **`apps/hub/AGENTS.md`** — Axum HTTP server: API routes, auth scaffolding, deployment
- **`.github/workflows/AGENTS.md`** — CI pipelines: what's enforced, how to extend
- **`docs/AGENTS.md`** — Documentation maintenance: what's aspirational, what's current

## Working in This Repo

1. **Check CI first:** `.github/workflows/ci.yml` defines what's actually enforced
2. **Run local validations:** `cargo fmt && cargo clippy && cargo test` for Rust; `pnpm -C apps/web test && pnpm -C apps/web e2e` for web
3. **Respect the scaffold state:** This is v0.0.1 — many advanced features are planned but not built
4. **Update nested AGENTS.md files** when making structural changes to a subsystem
5. **Keep docs aspirational:** `docs/` can describe future architecture, but mark unimplemented features clearly
6. **After a completed, validated change set:** commit it, push the branch upstream, then create and push a release tag matching `v*` so `.github/workflows/release.yml` runs

## Release Tagging Policy

- Release automation is triggered by Git tags that match `v*` via `.github/workflows/release.yml`.
- Unless the user specifies a version, use the next sensible SemVer-style tag in the existing sequence.
- Current default behavior for agent-delivered changes:
  1. finish the code/doc change,
  2. run the relevant existing validations,
  3. commit and push upstream,
  4. create and push the next `v*` release tag to trigger a fresh build/release run.

# Agent Guidance — Rust Crates

## Overview

The `crates/` workspace contains three Rust crates forming the core Telescope engine:

- **`telescope-core`** (`crates/core`) — Shared domain types, no internal dependencies
- **`telescope-engine`** (`crates/engine`) — Kubernetes watchers, caching, streaming (depends on core)
- **`telescope-api`** (`crates/api`) — API surface layer (depends on engine + core)

Dependency chain: `api → engine → core` (strict layering).

## Current State (v0.0.1)

- All crates have real implementations beyond just `version()` functions
- `telescope-core` provides cluster, namespace, and resource models
- `telescope-engine` has kube-rs integration with watchers and memory-efficient caching
- `telescope-api` is present but minimal — most API logic lives in `apps/hub`

## Build and Test Commands

```bash
# Format check (CI-enforced)
cargo fmt --all -- --check

# Lint (CI-enforced, warnings = errors)
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings

# Test all crates (CI-enforced)
cargo test --workspace --exclude telescope-desktop --all-features

# Test single crate
cargo test -p telescope-core
cargo test -p telescope-engine
cargo test -p telescope-api

# Test by name filter
cargo test -p telescope-engine -- watch
```

Note: `telescope-desktop` is excluded on Linux CI due to platform-specific GTK/WebKit dependencies.

## Workspace Configuration

Defined in root `Cargo.toml`:

```toml
[workspace]
members = [
  "crates/core",
  "crates/engine",
  "crates/api",
  "apps/desktop/src-tauri",
  "apps/hub"
]
default-members = [
  "crates/core",
  "crates/engine",
  "crates/api",
  "apps/hub"
]
```

`default-members` excludes desktop for cross-platform CI compatibility.

## Code Conventions

- **Edition:** 2021 (workspace-inherited)
- **Error handling:** Use `anyhow` for applications, `thiserror` for libraries
- **Async runtime:** Tokio throughout
- **Kubernetes client:** kube-rs (`kube` crate with `client` and `config` features)
- **Formatting:** `cargo fmt` enforced in CI
- **Linting:** `cargo clippy` with `-D warnings` (all warnings are errors)

## Architecture Notes

- **`telescope-core`** has no external Telescope dependencies — it's the foundation
- **`telescope-engine`** owns Kubernetes client logic, watchers, and cache management
- **`telescope-api`** is intended as a stable API facade but currently underdeveloped
- Most HTTP API logic lives in `apps/hub` (Axum server), not in these crates

## Dependencies

Key external crates in use:

- `kube` (v3.0+) — Kubernetes client and APIs
- `tokio` — Async runtime
- `serde` — Serialization
- `anyhow` / `thiserror` — Error handling
- `tracing` — Observability

## Testing Strategy

- Unit tests in `src/**/*.rs` alongside implementation (`#[cfg(test)]` modules)
- Integration tests in `tests/` subdirectories (not heavily used yet)
- CI runs full test suite with `--all-features`

## When to Edit

- **Add domain types:** Edit `crates/core`
- **Add Kubernetes logic:** Edit `crates/engine`
- **Change API abstractions:** Edit `crates/api` (currently light on logic)
- **Add HTTP endpoints:** Usually edit `apps/hub` instead (Axum server)

## What's NOT Here Yet

- gRPC server implementations (planned but not built)
- Advanced resource diffing or reconciliation logic
- Comprehensive integration test suite
- Performance benchmarks

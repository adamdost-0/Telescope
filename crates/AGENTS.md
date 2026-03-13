# Agent Guidance — Rust Crates

## Overview

The `crates/` workspace contains three Rust crates forming the core Telescope engine:

- **`telescope-core`** (`crates/core`) — Shared domain types and SQLite store; no internal dependencies
- **`telescope-engine`** (`crates/engine`) — Full Kubernetes engine: watchers, client, exec, logs, port-forward, Helm, metrics, CRDs, actions, secrets, audit
- **`telescope-api`** (`crates/api`) — Thin re-export facade (currently minimal; most API logic is in `apps/hub`)

Dependency chain: `api → engine → core` (strict layering; never invert).

## Build and Test Commands

```bash
# Format check (CI-enforced)
cargo fmt --all -- --check

# Lint (CI-enforced, warnings = errors, excludes desktop GTK deps on Linux)
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings

# Test all crates (CI-enforced)
cargo test --workspace --exclude telescope-desktop --all-features

# Test single crate
cargo test -p telescope-core
cargo test -p telescope-engine
cargo test -p telescope-api
cargo test -p telescope-hub

# Run engine integration tests (requires k3d cluster)
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d -- --nocapture
```

Note: `telescope-desktop` is excluded on Linux CI because of GTK/WebKit system dependencies.

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
default-members = [    # used when no -p flag; excludes desktop for Linux CI
  "crates/core",
  "crates/engine",
  "crates/api",
  "apps/hub"
]
[workspace.package]
edition = "2021"
license = "MIT"
```

## `telescope-core` — Shared Domain Types

### Key Types

**`ResourceEntry`** (in `store.rs`) — A stored Kubernetes resource:
```rust
pub struct ResourceEntry {
    pub gvk: String,             // e.g. "apps/v1/Deployment"
    pub namespace: String,       // empty for cluster-scoped
    pub name: String,
    pub resource_version: String,
    pub content: String,         // full JSON of the resource
    pub updated_at: String,      // ISO 8601
}
```

**`ResourceStore`** (in `store.rs`) — SQLite-backed document store via `rusqlite`:
- Schema: `resources` table (PRIMARY KEY: `gvk, namespace, name`), `user_preferences` table, `schema_version` table
- WAL journal mode, NORMAL synchronous writes
- `open(path: &str)` — opens or creates; use `":memory:"` for tests
- Key methods: `upsert`, `delete`, `delete_all_by_gvk`, `get_by_gvk`, `get_by_gvk_ns`, `get_by_name`, `search`, `list_preferences`, `get_preference`, `set_preference`

**`ConnectionState`** (in `connection.rs`) — State machine:
```rust
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Syncing { resources_synced, resources_total },
    Ready,
    Degraded { message },
    Error { message },
    Backoff { attempt, wait },
}
```

**`ConnectionEvent`** (in `connection.rs`) — Events sent on a `tokio::sync::watch` channel to drive UI updates.

**`VersionInfo`** — `{ name: String, version: String }` — version struct used by each crate.

## `telescope-engine` — Kubernetes Engine

### Module Map

| Module | Responsibility |
|---|---|
| `client` | Create `kube::Client` for a named kubeconfig context; `get_cluster_info()` (version, auth type, AKS detection) |
| `kubeconfig` | `list_contexts()`, `active_context()` — parse `~/.kube/config` via `kube::Config` |
| `watcher` | `ResourceWatcher` — kube-rs watch streams → SQLite; exponential backoff; concurrency-limited LIST ops |
| `secrets` | On-demand secret fetch (not cached); `list_secrets()`, `get_secret()` |
| `logs` | `get_pod_logs()`, `stream_pod_logs()` (async stream of log lines) |
| `exec` | `exec_command()` — non-interactive exec in a container |
| `portforward` | `start_port_forward()` — local port → pod port tunnel |
| `actions` | `delete_resource()`, `scale_resource()`, `rollout_restart()`, `rollout_status()`, `apply_resource()` |
| `helm` | `list_releases()`, `get_release_history()`, `get_release_values()`, `rollback()` — wraps `helm` CLI |
| `metrics` | `get_pod_metrics()`, `get_node_metrics()` — queries metrics-server API |
| `crd` | `list_crds()` — fetches all CRD definitions from the cluster |
| `namespace` | `list_namespaces()` — fetches namespace list |
| `audit` | `AuditEntry`, `log_audit()` — append-only JSONL audit file |
| `error` | `EngineError`, `Result<T>` — unified error type |

### `ResourceWatcher`

```rust
pub struct ResourceWatcher {
    client: Client,
    store: Arc<Mutex<ResourceStore>>,
    state_tx: watch::Sender<ConnectionState>,
    list_semaphore: Arc<Semaphore>,  // MAX_CONCURRENT_LISTS = 3
}
```

- `Clone` is cheap (all fields are `Arc`-wrapped)
- Spawns one async task per watched GVK
- Watched GVKs (13 total):
  `v1/Pod`, `v1/Event`, `v1/Node`, `apps/v1/Deployment`, `apps/v1/StatefulSet`, `apps/v1/DaemonSet`,
  `apps/v1/ReplicaSet`, `v1/Service`, `v1/ConfigMap`, `batch/v1/Job`, `batch/v1/CronJob`,
  `networking.k8s.io/v1/Ingress`, `v1/PersistentVolumeClaim`
- Secrets are intentionally excluded — fetched on-demand only

### `AuditEntry`

```rust
pub struct AuditEntry {
    pub timestamp: String,
    pub actor: String,       // user email from auth (or "desktop-user@local")
    pub context: String,
    pub namespace: String,
    pub action: String,      // e.g. "delete", "scale", "rollout-restart"
    pub resource_type: String,
    pub resource_name: String,
    pub result: String,      // "success" or "error: ..."
    pub detail: Option<String>,
}
```

Written as a JSONL line to `AUDIT_PATH` (hub) or `audit_log_path` (desktop).

## `telescope-api` — Facade (Minimal)

Currently re-exports engine and core types. Not used for HTTP logic — that lives in `apps/hub`. Add stable API surface abstractions here when introducing versioned APIs.

## Code Conventions

- **Edition:** 2021 (workspace-inherited via `[workspace.package]`)
- **Error handling:** `thiserror` for library error types (`EngineError`); `anyhow` in application code
- **Async runtime:** Tokio throughout (`#[tokio::main]`, `tokio::spawn`, `tokio::sync::*`)
- **Kubernetes client:** kube-rs (`kube` crate with `client`, `config`, `runtime` features)
- **Serialization:** `serde` + `serde_json`
- **Observability:** `tracing` crate (spans + events); `tracing-subscriber` for filtering
- **Formatting:** `cargo fmt` enforced in CI
- **Linting:** `cargo clippy -D warnings` — all warnings are errors

## Testing Strategy

- Unit tests in `src/**/*.rs` alongside implementation (`#[cfg(test)]` modules)
- Engine integration tests in `crates/engine/tests/integration_k3d.rs` — require real k3d cluster (`K3D_TEST=1`)
- Integration tests run in `integration.yml` workflow (manual trigger or push to main on engine/core paths)
- Use `ResourceStore::open(":memory:")` for in-process store tests

## When to Edit

| Task | File(s) |
|---|---|
| Add a shared domain type | `crates/core/src/` |
| Change the resource store schema | `crates/core/src/store.rs` |
| Add a new Kubernetes watch type | `crates/engine/src/watcher.rs` |
| Add a write action (delete/scale/apply) | `crates/engine/src/actions.rs` |
| Add a new engine capability | New module in `crates/engine/src/` |
| Change the Tauri command API surface | `apps/desktop/src-tauri/src/main.rs` |
| Add a hub HTTP endpoint | `apps/hub/src/routes.rs` + `apps/hub/src/main.rs` |

## What's NOT Here Yet

- gRPC server implementations (planned, not built)
- `telescope-api` expanded with stable versioned API surface
- Advanced resource diffing/reconciliation logic
- Performance benchmarks
- Comprehensive integration test suite (beyond k3d smoke tests)

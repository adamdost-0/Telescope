# Agent Guidance — Rust Crates

## Overview

The `crates/` workspace contains three Rust library crates forming the Telescope backend:

- **`telescope-core`** (`crates/core`) — Shared domain types, SQLite-backed storage, connection state machine
- **`telescope-engine`** (`crates/engine`) — Kubernetes operations: 28+ resource watchers, actions, Helm, logs, exec, port-forward, metrics, node ops, CRDs, secrets, namespaces, audit
- **`telescope-azure`** (`crates/azure`) — Azure ARM client, AKS management-plane operations, cloud identity resolution

These crates are consumed by the desktop binary (`apps/desktop/src-tauri`).

**Dependency graph:**
```
desktop → engine + azure + core
engine  → core
azure   → core
```

There is no `crates/api` or `apps/hub` — those were removed. The desktop Tauri binary is the only consumer.

## Workspace Configuration

Root `Cargo.toml`:
```toml
[workspace]
resolver = "2"
members = [
  "crates/core",
  "crates/engine",
  "crates/azure",
  "apps/desktop/src-tauri",
]
default-members = [
  "crates/core",
  "crates/engine",
  "crates/azure",
]
```

`default-members` excludes desktop for Linux CI compatibility (GTK/WebKit deps).

## Build and Test Commands

**Container-first validation (required before push):**

```bash
# Full validation suite (recommended)
./scripts/dev-test.sh

# Or run Rust checks inside the container shell
./scripts/dev-test.sh shell
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

The dev container includes Rust stable, clippy, rustfmt, and all build dependencies. Always validate Rust changes inside the container before pushing.

**Individual crate testing (inside container or host):**

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
cargo test -p telescope-azure

# Test by name filter
cargo test -p telescope-engine -- watch
```

## Crate Details

### telescope-core (`crates/core`)

Foundation crate with no internal workspace dependencies.

**Modules:**
| Module | Exports |
|--------|---------|
| `connection` | `ConnectionState` (enum: Disconnected, Connecting, Syncing, Ready, Degraded, Error, Backoff), `ConnectionEvent` (enum: Connect, Authenticated, SyncStarted, SyncProgress, SyncComplete, WatchError, Disconnected, RetryReady, UserDisconnect) |
| `store` | `ResourceStore` (SQLite-backed document store over `resources` + `user_preferences` tables), `ResourceEntry` (gvk, namespace, name, resource_version, content, updated_at) |
| `lib` | `VersionInfo` |

`ResourceStore` provides: `open`, `upsert`, `delete`, `delete_all_by_gvk`, `list`, `get`, `count`, `get_preference`, `set_preference`, `delete_preference`.

### telescope-engine (`crates/engine`)

Kubernetes engine crate. Depends on `telescope-core`.

**Modules:**
| Module | Key Exports |
|--------|-------------|
| `client` | `ClusterInfo`, `get_cluster_info`, `create_client`, `create_client_for_context`, `create_client_for_context_as_user` |
| `kubeconfig` | `ClusterContext`, `list_contexts`, `active_context` |
| `watcher` | `ResourceWatcher` with 30+ concrete watchers: pods, deployments, services, configmaps, secrets, events, nodes, statefulsets, daemonsets, replicasets, jobs, cronjobs, ingresses, network policies, endpoint slices, PVCs, resource quotas, limit ranges, roles, cluster roles, role bindings, cluster role bindings, service accounts, HPAs, pod disruption budgets, priority classes, validating/mutating webhooks, storage classes, persistent volumes |
| `actions` | `delete_resource`, `scale_resource`, `rollout_restart`, `rollout_status`, `apply_resource` |
| `dynamic` | `resolve_dynamic_kind`, `list_dynamic_resources`, `get_dynamic_resource`, `apply_dynamic_resource`, `delete_dynamic_resource` |
| `logs` | `LogRequest`, `LogChunk`, `get_pod_logs`, `stream_pod_logs`, `list_containers` |
| `exec` | `ExecRequest`, `ExecResult`, `exec_command` |
| `portforward` | `PortForwardRequest`, `PortForwardStatus`, `start_port_forward`, `active_forward_count` |
| `helm` | `HelmRelease`, `list_releases`, `get_release_history`, `get_release_values`, `extract_values_from_release`, `rollback_release`, `redact_sensitive_values` |
| `metrics` | `PodMetrics`, `ContainerMetrics`, `NodeMetricsData`, `is_metrics_available`, `get_pod_metrics`, `get_node_metrics` |
| `node_ops` | `DrainResult`, `DrainOptions`, `cordon_node`, `uncordon_node`, `add_taint`, `remove_taint`, `drain_node` |
| `crd` | `CrdInfo`, `list_crds` |
| `secrets` | `list_secrets`, `get_secret` |
| `namespace` | `list_namespaces`, `create_namespace`, `delete_namespace` |
| `audit` | `AuditEntry`, `log_audit` |
| `error` | `EngineError`, `Result<T>` |

### telescope-azure (`crates/azure`)

Azure ARM management-plane crate. Depends on `telescope-core` (for preference storage).

**Modules:**
| Module | Key Exports |
|--------|-------------|
| `client` | `ArmClient` — HTTP client for Azure Resource Manager (`get`, `put`, `post`, `delete`); supports Azure Public, Government, and air-gapped clouds |
| `types` | `AzureCloud` (enum with `detect_from_url`, `arm_endpoint`, `auth_endpoint`, `portal_url`, `token_scope`), `AksResourceId` (with `arm_path`, `agent_pool_path`, `upgrade_profile_path`, `maintenance_config_path`), `AKS_API_VERSION` |
| `aks` | Cluster ops: `get_cluster`, `start_cluster`, `stop_cluster`. Node pools: `list_node_pools`, `create_node_pool`, `scale_node_pool`, `update_autoscaler`, `delete_node_pool`, `upgrade_pool_version`, `upgrade_pool_node_image`. Upgrades: `get_upgrade_profile`, `upgrade_cluster`, `get_pool_upgrade_profile`. Maintenance: `list_maintenance_configs`. Types: `AksClusterDetail`, `AksNodePool`, `UpgradeProfile`, `CreateNodePoolRequest`, etc. |
| `resolve` | `resolve_aks_identity`, `resolve_aks_identity_from_preferences` — detect AKS subscription/resource-group/cluster from kubeconfig or stored preferences |
| `error` | `AzureError`, `Result<T>` |

## Code Conventions

- **Edition:** 2021 (workspace-inherited)
- **Error handling:** `thiserror` for library error types
- **Async runtime:** Tokio
- **Kubernetes client:** kube-rs (`kube` crate)
- **Formatting:** `cargo fmt` enforced in CI
- **Linting:** `cargo clippy` with `-D warnings`
- **Module style:** File-based modules (no `mod.rs`)

## Testing Strategy

- Unit tests in `#[cfg(test)]` modules alongside implementation
- CI runs full test suite with `--all-features`
- `telescope-desktop` excluded from Linux CI (platform-specific deps)
- **Container gate:** Run `./scripts/dev-test.sh` before pushing any Rust changes to validate fmt, clippy, and tests in the same environment CI uses

## When to Edit

- **Add domain types:** Edit `crates/core`
- **Add Kubernetes operations:** Edit `crates/engine`
- **Add Azure/AKS operations:** Edit `crates/azure`
- **Expose to desktop UI:** Add a Tauri command in `apps/desktop/src-tauri/src/main.rs` that calls the crate function

# Agent Guidance — Tauri Desktop App

## Overview

`apps/desktop` is a Tauri 2 desktop application that packages the SvelteKit web frontend (`apps/web`) as a native app for Windows, macOS, and Linux. It exposes all Kubernetes operations as Tauri IPC commands backed by the `telescope-engine` and `telescope-core` Rust crates.

**Key principle:** The desktop app does NOT maintain its own UI. All UI lives in `apps/web`.

## Architecture

```
apps/desktop/
├── src-tauri/
│   ├── Cargo.toml            # Crate manifest (name: telescope-desktop)
│   ├── tauri.conf.json       # Tauri v2 config (frontendDist, identifier, window settings)
│   └── src/
│       └── main.rs           # All Tauri commands + AppState
├── scripts/
│   └── prepare-frontend.mjs  # Builds apps/web and copies to apps/desktop/dist/
└── dist/                     # Built web frontend (git-ignored)
```

## Frontend Build Flow

```
pnpm -C apps/desktop build
  └─→ scripts/prepare-frontend.mjs
        ├─→ pnpm run build (in apps/web)    # produces apps/web/build/
        └─→ copies build/ to apps/desktop/dist/
              └─→ Tauri packages dist/ as the desktop frontend
```

**To change the desktop UI:** Edit `apps/web`, not files in this directory.

## Build and Test Commands

```bash
# Debug build (requires Rust toolchain + platform SDK)
pnpm -C apps/desktop build

# Release bundle (installer: MSI/NSIS on Windows, DMG on macOS)
pnpm -C apps/desktop bundle

# Dev mode with hot reload (Vite dev server + Tauri)
pnpm -C apps/desktop tauri dev
```

Required system dependencies:
- **macOS:** Xcode command-line tools
- **Linux:** GTK 3, WebKit2GTK, libssl-dev, librsvg2-dev, etc.
- **Windows:** Windows SDK

## AppState (Tauri Backend)

`main.rs` defines `AppState` — the Tauri-managed application state:

```rust
struct AppState {
    db_path: String,
    audit_log_path: String,
    store: Arc<Mutex<ResourceStore>>,         // SQLite via rusqlite
    connection_state: Arc<RwLock<ConnectionState>>,
    watch_handle: TokioMutex<Option<JoinHandle<()>>>,  // active ResourceWatcher task
    active_context: RwLock<Option<String>>,
    active_namespace: RwLock<String>,
}
```

- `ResourceStore` is in `std::sync::Mutex` (not `tokio::sync`) because `rusqlite::Connection` is `Send` but not `Sync`
- `ResourceWatcher` runs as a background `tokio::spawn` task; its handle is stored in `watch_handle`
- On `connect_to_context`: watchers are started, `ConnectionState` transitions through `Connecting → Syncing → Ready`
- On `disconnect`: the watcher task is aborted, all 13 watched GVK caches are cleared from the store
- Audit log path defaults to `~/.local/share/telescope/audit.log` (platform-specific via Tauri path resolver)
- Desktop actor in audit entries: `"desktop-user@local"`

## Tauri IPC Commands (Complete List)

All commands are defined with `#[tauri::command]` in `src/main.rs` and registered in `tauri::Builder::invoke_handler`.

### Read / Query Commands

| Command | Description |
|---|---|
| `list_contexts` | List kubeconfig contexts from `~/.kube/config` |
| `active_context` | Get current active kubeconfig context (falls back to kubeconfig default) |
| `get_connection_state` | Returns current `ConnectionState` |
| `get_cluster_info` | Server version, auth type, AKS detection for connected context |
| `get_pods` | Fetch pod resources from store (optional namespace filter) |
| `get_resources` | Fetch any GVK from store (`gvk` + optional `namespace`) |
| `get_resource` | Fetch a single resource by GVK, namespace, name |
| `get_resource_counts` | Counts per GVK across all 13 watched types |
| `count_resources` | Count by GVK + namespace |
| `get_events` | Fetch events (optional namespace + involved_object) |
| `search_resources` | Full-text search across resource names/content |
| `get_secrets` | Fetch secrets on-demand from Kubernetes API (NOT from store) |
| `get_secret` | Fetch one secret on-demand |
| `list_namespaces` | List namespace names from store |
| `get_namespace` | Get current active namespace |
| `get_pod_logs` | Fetch historical log output for a pod/container |
| `list_containers` | List container names in a pod (init: prefix for init containers) |
| `list_helm_releases` | List Helm releases (via `helm` CLI) |
| `get_helm_release_history` | All revisions of a Helm release |
| `get_helm_release_values` | User-supplied values for latest Helm release revision |
| `get_pod_metrics` | Pod CPU/memory from metrics-server |
| `get_node_metrics` | Node CPU/memory with allocatable percentages |
| `check_metrics_available` | Whether metrics-server API is reachable |
| `list_crds` | All Custom Resource Definitions on the cluster |
| `get_preference` | Read a user preference by key (from SQLite) |

### Write / Action Commands

| Command | Description |
|---|---|
| `connect_to_context` | Connect to a kubeconfig context; starts ResourceWatcher |
| `disconnect` | Stop watching, clear store |
| `set_namespace` | Change active namespace; clears cached data and restarts watchers |
| `scale_resource` | Scale Deployment or StatefulSet replicas |
| `delete_resource` | Delete a namespaced resource (Pod, Deployment, etc.) |
| `apply_resource` | Apply/create/update resource from YAML or JSON manifest |
| `rollout_restart` | Trigger rolling restart of Deployment, StatefulSet, or DaemonSet |
| `rollout_status` | Get rollout completion status (desired/ready/updated/available) |
| `exec_command` | Non-interactive exec in a container; returns stdout/stderr |
| `start_port_forward` | Local → pod port tunnel; returns local port number |
| `start_log_stream` | Streaming log tail; emits `log-chunk` Tauri events to the frontend |
| `helm_rollback` | Roll back a Helm release to a specific revision |
| `set_preference` | Write a user preference by key (to SQLite) |

**Write operations are desktop-only.** The hub's web fallback in `api.ts` returns `undefined` for all of these.

## CI Integration

Desktop builds run in separate CI jobs:

- `ci.yml` → `desktop-build` job: matrix `[windows-latest, macos-latest]`, debug build only
- `build-desktop.yml`: release bundle + artifact upload (path-triggered or manual)
- `release.yml`: release bundle + GitHub Release attachment

Linux is excluded from CI because GTK/WebKit system deps are not installed in the runner.

## Tauri Configuration

`src-tauri/tauri.conf.json` — Tauri v2 format:
- `frontendDist: "../dist"` — built web output from `prepare-frontend.mjs`
- App identifier: `com.telescope.app`
- Window settings: title, size, resizable
- Bundle: targets MSI/NSIS (Windows), DMG (macOS)

## Platform-Specific Notes

### macOS
- Requires code signing for distribution (Apple Developer ID)
- Outputs DMG + macOS app bundle
- Supports Apple Silicon (arm64) and Intel (x86_64) — Tauri handles universal binary

### Windows
- Outputs MSI installer and NSIS installer
- Windows Defender may flag unsigned builds during development
- Requires Windows SDK

### Linux
- AppImage, `.deb`, or `.rpm` packaging possible
- NOT currently built in CI (manual install of GTK/WebKit required)

## Code Conventions

- **Minimal Rust beyond Tauri boilerplate** — keep engine logic in `crates/engine`, not in `main.rs`
- **No UI code** — all UI lives in `apps/web`
- **Tauri commands should be thin** — call engine functions, handle errors, emit events
- **Audit all destructive actions** — call `engine::audit::log_audit()` for scale, delete, apply, rollout, exec
- **Desktop actor:** use `"desktop-user@local"` as the audit `actor` field

## What's Missing

- Dedicated desktop unit/integration tests (CI relies on `apps/web` E2E)
- Native menu integration
- System tray support
- Auto-update mechanism (`tauri-plugin-updater`)
- Deep OS integrations (notifications, file dialogs)
- Linux CI builds

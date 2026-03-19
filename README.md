# Telescope

AKS-first Kubernetes IDE for desktop — fast, native, and built with Rust + Tauri.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/adamdost-0/Telescope)](https://github.com/adamdost-0/Telescope/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/adamdost-0/Telescope/ci.yml?branch=main&label=CI)](https://github.com/adamdost-0/Telescope/actions/workflows/ci.yml)

> No Electron. Desktop-only architecture with native performance.  
> Typical idle memory: ~100MB (vs Lens ~500MB).

## Why Telescope

|  | Telescope | Lens | k9s |
|---|---|---|---|
| **Memory (idle)** | ~100MB | ~500MB | ~30MB |
| **Technology** | Rust + Tauri | Electron | Go TUI |
| **AKS integration** | ✅ Native | ❌ None | ❌ None |
| **Azure ARM ops** | ✅ Built-in | ❌ None | ❌ None |
| **GUI** | ✅ Desktop | ✅ Desktop | ❌ Terminal |
| **Open source** | ✅ MIT | ❌ Proprietary | ✅ Apache 2.0 |
| **Helm** | ✅ Built-in | ✅ Extension | ❌ No |
| **Metrics** | ✅ Built-in | ✅ Built-in | ✅ Built-in |

## Feature Highlights

- **AKS-first workflows:** Native Azure ARM management-plane operations for cluster lifecycle and node pool management.
- **Deep Kubernetes coverage:** 28+ watched resource types with real-time navigation, filtering, and detail views.
- **Built-in operator tooling:** Logs, exec, port-forward, resource actions, Helm workflows, and metrics in one app.
- **Desktop-native architecture:** Rust backend + Tauri IPC + SvelteKit UI, optimized for responsiveness and memory use.
- **Production-aware UX:** Guardrails for destructive actions, identity visibility, and audit-oriented local action logging.

For deep-dive docs and expanded feature guides, see [`docs/`](docs/).

## Full Feature Matrix

### Cluster Connection and Navigation

| Capability | Details |
|---|---|
| Full cluster connection and context management | Discover kubeconfig contexts, connect/disconnect, track connection state, and switch namespaces |
| Search and settings | Search cached resources quickly and manage user-facing preferences from the UI |
| Audit logging | Record key local actions for traceability in the desktop app |

### Kubernetes Operations

| Capability | Details |
|---|---|
| Broad resource coverage | Browse and manage workloads, networking, configuration, storage, policy, RBAC, and admission resources with 28+ watched Kubernetes resource types |
| CRD browsing | Explore installed CustomResourceDefinitions and dynamic resources with schema/details support |
| Pod operations | View logs, exec into containers, and start port-forwards |
| Resource actions | Scale workloads, delete resources, create namespaces, apply YAML, and trigger rollout operations with safety checks |
| Helm release management | List releases, inspect history/values, and support Helm rollback workflows |
| Node management and metrics | Inspect nodes plus pod and node metrics for cluster health |

### Azure ARM Management Plane

| Capability | Details |
|---|---|
| Native ARM client | Manage AKS from the desktop app without leaving Telescope |
| Node pool CRUD | List, create, scale, autoscale, upgrade, and delete AKS node pools |
| Cluster lifecycle control | Start and stop AKS clusters, inspect upgrade profiles, and manage cluster upgrades |
| ARM-sourced diagnostics | View maintenance configs, upgrade readiness, and platform diagnostics sourced from Azure management APIs |
| Multi-cloud support | Works across Azure Commercial, Government, Secret, and Top Secret cloud environments |

### Desktop Experience

| Capability | Details |
|---|---|
| Native Tauri app | Rust backend commands over IPC with 60+ desktop commands exposed to the UI |
| SvelteKit frontend | `apps/web` contains the frontend source that is packaged into the desktop application by Tauri |
| Ephemeral local cache | Telescope clears the SQLite resource cache on startup, disconnect, and app exit; Secrets stay on-demand only, and cached Pod env literal values are redacted before they hit disk |

### AKS-First Experience

| Capability | Details |
|---|---|
| Auth detection | Identifies exec/token/certificate auth and provides kubelogin guidance |
| Node pool grouping | Nodes organized by AKS agent pool with VM size, OS, and mode |
| Add-on status | Container Insights, Azure Policy, Key Vault CSI, KEDA, and Flux health |
| Azure Portal links | One-click navigation for AKS clusters |
| Workload identity visibility | Azure identity bindings shown on pod detail views |
| Production guardrails | Red banner and type-to-confirm flows for destructive ops in production |

## Screenshots

<!-- TODO: screenshots -->

## Project Structure

### Cargo workspace
- `crates/core` — shared domain, state, and storage types
- `crates/engine` — Kubernetes engine code: client, watchers, logs, exec, port-forward, actions, Helm, metrics, CRDs
- `crates/azure` — Azure ARM management client for AKS lifecycle, node pools, upgrades, diagnostics, and cloud targeting
- `apps/desktop/src-tauri` — Tauri desktop shell and IPC command surface

### pnpm workspace
- `apps/web` — SvelteKit frontend source packaged into the desktop app
- `apps/desktop` — Tauri build/package wrapper for the native desktop distribution
- `packages/*` — shared workspace packages

## Quick Start

### 1) Download a Release

Get the latest build from [GitHub Releases](https://github.com/adamdost-0/Telescope/releases).

- **Windows (recommended):** `telescope-windows-x64-v1.0.x.msi`
- **Windows alternatives:** `telescope-windows-x64-setup-v1.0.x.exe`, `telescope-windows-x64-portable-v1.0.x.exe`
- **macOS (recommended):** `telescope-macos-arm64-v1.0.x.dmg`
- **macOS portable:** `telescope-macos-arm64-portable-v1.0.x`
- **Verification:** every release includes SHA256 checksums as `telescope-{platform}-v1.0.x.sha256`

### 2) Build from Source

```bash
# Prerequisites: Rust, Node.js 22+, pnpm
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
pnpm -C apps/desktop dev     # Development mode
pnpm -C apps/desktop bundle  # Release build
```

### 3) Connect to a Cluster

1. Launch Telescope.
2. Your kubeconfig contexts appear automatically.
3. Select a context — pods, resources, and ARM-backed AKS details load in real time.
4. Navigate using the sidebar: Workloads, Network, Config, Storage, Policy, RBAC, Nodes, Events, Helm, and Azure management views.

## Development

### Prerequisites

- Rust (stable)
- Node.js 22+
- pnpm 9.15+
- Platform SDK:
  - **Windows:** Microsoft Edge WebView2
  - **macOS:** Xcode Command Line Tools
  - **Linux:** WebKitGTK 4.1, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

Linux builds do not require system OpenSSL development packages for the Azure path; the remaining OpenSSL dependency is vendored during the build.

### Command Reference

```bash
# Rust
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features

# Frontend
pnpm -C apps/web test         # Unit tests
pnpm -C apps/web build        # Production build for desktop packaging
pnpm -C apps/web e2e          # E2E tests (needs Playwright)

# Desktop
pnpm -C apps/desktop dev      # Dev mode
pnpm -C apps/desktop build    # Debug desktop build
pnpm -C apps/desktop bundle   # Release bundle

# k3d testing
./scripts/k3d-setup.sh        # Create local cluster
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d
./scripts/k3d-teardown.sh
```

## Architecture and Roadmap

- Architecture: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- Roadmap: [docs/ROADMAP.md](docs/ROADMAP.md)

Today Telescope ships as a **desktop-only Tauri application** with deep Kubernetes coverage and native Azure ARM management for AKS operations.

Current priorities:
- **Desktop depth** — keep expanding advanced exec, port-forward, and mutation workflows
- **Azure management depth** — keep broadening AKS lifecycle, diagnostics, and maintenance operations
- **Operational visibility** — improve live updates, diagnostics, and auditability across cluster and ARM data
- **UX polish and testing** — keep improving performance, usability, and coverage across Rust, frontend, and desktop flows
- **Extensibility over time** — leave room for future integrations without reintroducing a separate web or hub mode

## License and Links

- License: [MIT](LICENSE)
- Contributing: [CONTRIBUTING.md](CONTRIBUTING.md)
- Documentation index: [`docs/`](docs/)

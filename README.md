# 🔭 Telescope

**A memory-efficient, AKS-first Kubernetes IDE** — a **desktop-only** Tauri application for operating Kubernetes and AKS clusters with native Azure ARM management-plane support.

> ⚡ **No Electron.** Native performance with Tauri's WebView. ~100MB idle vs Lens's ~500MB.

## Why Telescope?

| | Telescope | Lens | k9s |
|---|---|---|---|
| **Memory (idle)** | ~100MB | ~500MB | ~30MB |
| **Technology** | Rust + Tauri | Electron | Go TUI |
| **AKS Integration** | ✅ Native | ❌ None | ❌ None |
| **Azure ARM Ops** | ✅ Built-in | ❌ None | ❌ None |
| **GUI** | ✅ Desktop | ✅ Desktop | ❌ Terminal |
| **Open Source** | ✅ MIT | ❌ Proprietary | ✅ Apache 2.0 |
| **Helm** | ✅ Built-in | ✅ Extension | ❌ No |
| **Metrics** | ✅ Built-in | ✅ Built-in | ✅ Built-in |

## Features

### Cluster connection and navigation
- 🔌 **Full cluster connection and context management** — Discover kubeconfig contexts, connect/disconnect, track connection state, and switch namespaces
- 🔎 **Search and settings** — Search cached resources quickly and manage user-facing preferences from the UI
- 🧾 **Audit logging** — Record key local actions for traceability in the desktop app

### Kubernetes operations
- 📦 **Broad resource coverage** — Browse and manage workloads, networking, configuration, storage, policy, RBAC, and admission resources with 28+ watched Kubernetes resource types
- 🧩 **CRD browsing** — Explore installed CustomResourceDefinitions and dynamic resources with schema/details support
- 📋 **Pod operations** — View logs, exec into containers, and start port-forwards
- 🗑️ **Resource actions** — Scale workloads, delete resources, create namespaces, apply YAML, and trigger rollout operations with safety checks
- ⛵ **Helm release management** — List releases, inspect history/values, and support Helm rollback workflows
- 📊 **Node management and metrics** — Inspect nodes plus pod and node metrics for cluster health

### Azure ARM management plane
- ☁️ **Native ARM client** — Manage AKS from the desktop app without leaving Telescope
- 🧱 **Node pool CRUD** — List, create, scale, autoscale, upgrade, and delete AKS node pools
- ⏯️ **Cluster lifecycle control** — Start and stop AKS clusters, inspect upgrade profiles, and manage cluster upgrades
- 🩺 **ARM-sourced diagnostics** — View maintenance configs, upgrade readiness, and platform diagnostics sourced from Azure management APIs
- 🌍 **Multi-cloud support** — Works across Azure Commercial, Government, Secret, and Top Secret cloud environments

### Desktop experience
- 🖥️ **Native Tauri app** — Rust backend commands over IPC with 60+ desktop commands exposed to the UI
- 🎨 **SvelteKit frontend** — `apps/web` contains the frontend source that is packaged into the desktop application by Tauri

### AKS-first experience
- 🔑 **Auth detection** — Identifies exec/token/certificate auth and provides kubelogin guidance
- 🏷️ **Node pool grouping** — Nodes organized by AKS agent pool with VM size, OS, and mode
- 🛡️ **Add-on status** — Container Insights, Azure Policy, Key Vault CSI, KEDA, and Flux health
- 🌐 **Azure Portal links** — One-click navigation for AKS clusters
- 🔐 **Workload identity visibility** — Azure identity bindings shown on pod detail views
- 🚨 **Production guardrails** — Red banner and type-to-confirm flows for destructive ops in production

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

### Download
Get the latest release from [GitHub Releases](https://github.com/adamdost-0/Telescope/releases).

### Build from Source
```bash
# Prerequisites: Rust, Node.js 22+, pnpm
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
pnpm -C apps/desktop dev     # Development mode
pnpm -C apps/desktop bundle  # Release build
```

### Connect to a Cluster
1. Launch Telescope
2. Your kubeconfig contexts appear automatically
3. Select a context — pods, resources, and ARM-backed AKS details load in real time
4. Navigate using the sidebar: Workloads, Network, Config, Storage, Policy, RBAC, Nodes, Events, Helm, and Azure management views

## Development

### Prerequisites
- Rust (stable)
- Node.js 22+
- pnpm 9.15+
- Platform SDK (WebView2 on Windows, WebKitGTK on Linux)

### Commands
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

# k3d Testing
./scripts/k3d-setup.sh        # Create local cluster
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d
./scripts/k3d-teardown.sh
```

### Architecture
See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed diagrams.

## Roadmap
See [docs/ROADMAP.md](docs/ROADMAP.md) for the broader milestone plan.

Today Telescope ships as a **desktop-only Tauri application** with deep Kubernetes coverage and native Azure ARM management for AKS operations.

Current priorities:
- **Desktop depth** — keep expanding advanced exec, port-forward, and mutation workflows
- **Azure management depth** — keep broadening AKS lifecycle, diagnostics, and maintenance operations
- **Operational visibility** — improve live updates, diagnostics, and auditability across cluster and ARM data
- **UX polish and testing** — keep improving performance, usability, and coverage across Rust, frontend, and desktop flows
- **Extensibility over time** — leave room for future integrations without reintroducing a separate web or hub mode

## License
MIT

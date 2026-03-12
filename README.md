# 🔭 Telescope

**A memory-efficient, AKS-first Kubernetes IDE** — the open-source alternative to Lens, built with Rust and Tauri.

> ⚡ **No Electron.** Native performance with Tauri's WebView. ~100MB idle vs Lens's ~500MB.

## Why Telescope?

| | Telescope | Lens | k9s |
|---|---|---|---|
| **Memory (idle)** | ~100MB | ~500MB | ~30MB |
| **Technology** | Rust + Tauri | Electron | Go TUI |
| **AKS Integration** | ✅ Native | ❌ None | ❌ None |
| **GUI** | ✅ Desktop + Web | ✅ Desktop | ❌ Terminal |
| **Open Source** | ✅ MIT | ❌ Proprietary | ✅ Apache 2.0 |
| **Helm** | ✅ Built-in | ✅ Extension | ❌ No |
| **Metrics** | ✅ Built-in | ✅ Built-in | ✅ Built-in |

## Features

### Core Kubernetes IDE
- 📦 **Resource Explorer** — Browse Pods, Deployments, Services, ConfigMaps, Secrets, Nodes, Events, and more
- 📋 **Log Viewer** — Stream pod logs with container selector, search, and previous logs
- 💻 **Exec Terminal** — Run commands in containers
- ⇌ **Port Forward** — TCP tunneling with local/remote port mapping
- ✏️ **YAML Editor** — Edit and apply resources with dry-run preview
- 🗑️ **Safe Operations** — Delete, scale, rollout restart with confirmation dialogs
- 📊 **Cluster Dashboard** — Overview with resource counts, pod phases, warning events

### AKS-First (Features Lens Doesn't Have)
- 🔑 **Auth Detection** — Identifies exec/token/certificate auth, kubelogin guidance
- 🏷️ **Node Pool Grouping** — Nodes organized by AKS agent pool with VM size, OS, mode
- 🛡️ **Add-on Status** — Container Insights, Azure Policy, Key Vault CSI, KEDA, Flux health
- 🌐 **Azure Portal Links** — One-click "Open in Azure Portal" for AKS clusters
- 🔐 **Workload Identity** — Azure identity bindings visible on pod detail
- 🚨 **Production Guardrails** — Red banner, type-to-confirm for destructive ops in production

### Architecture
- **Rust Engine** — kube-rs client, watch-driven sync, SQLite storage
- **Tauri Desktop** — Native WebView, no Electron, 25+ IPC commands
- **SvelteKit Frontend** — Svelte 5 runes, shared state stores, dark theme

## Quick Start

### Download
Get the latest release from [GitHub Releases](https://github.com/adamdost-0/Telescope/releases).

### Build from Source
```bash
# Prerequisites: Rust, Node.js 22+, pnpm
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
pnpm -C apps/desktop dev    # Development mode
pnpm -C apps/desktop bundle  # Release build
```

### Connect to a Cluster
1. Launch Telescope
2. Your kubeconfig contexts appear automatically
3. Select a context — pods and resources load in real-time
4. Navigate using the sidebar: Workloads, Network, Config, Storage, Events

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
cargo clippy --workspace --exclude telescope-desktop -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features

# Web
pnpm -C apps/web test        # Unit tests
pnpm -C apps/web build       # Production build
pnpm -C apps/web e2e          # E2E tests (needs Playwright)

# Desktop
pnpm -C apps/desktop dev      # Dev mode
pnpm -C apps/desktop bundle   # Release build

# k3d Testing
./scripts/k3d-setup.sh        # Create local cluster
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d
./scripts/k3d-teardown.sh
```

### Architecture
See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed diagrams.

## Roadmap
See [docs/ROADMAP.md](docs/ROADMAP.md) for the full milestone plan.

| Milestone | Status | Parity |
|-----------|--------|--------|
| M0 Foundations | ✅ Complete | - |
| M1 Connect + Browse | ✅ Complete | 15% |
| M2 Debug Loop + UX | ✅ Complete | 25% |
| M3 Resource Actions | ✅ Complete | 50% |
| M4 AKS-First | ✅ Complete | 60% |
| M5 Helm + Metrics | 🔄 In Progress | 75% |
| M6 Search + CRDs | 🔲 Planned | 90% |
| M7 Web + Hub | 🔲 Planned | 95% |
| M8 Plugins | 🔲 Planned | 100% |

## License
MIT

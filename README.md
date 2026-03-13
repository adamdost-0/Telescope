# 🔭 Telescope

**A memory-efficient, AKS-first Kubernetes IDE** — the open-source alternative to Lens, built with Rust and Tauri.

> ⚡ **No Electron.** Native performance with Tauri's WebView. ~100MB idle vs Lens's ~500MB.

## Why Telescope?

| | Telescope | Lens | k9s |
|---|---|---|---|
| **Memory (idle)** | ~100MB | ~500MB | ~30MB |
| **Technology** | Rust + Tauri | Electron | Go TUI |
| **AKS Integration** | ✅ Native | ❌ None | ❌ None |
| **GUI** | ✅ Desktop | ✅ Desktop | ❌ Terminal |
| **Open Source** | ✅ MIT | ❌ Proprietary | ✅ Apache 2.0 |
| **Helm** | ✅ Built-in | ✅ Extension | ❌ No |
| **Metrics** | ✅ Built-in | ✅ Built-in | ✅ Built-in |

## Features

### Cluster connection and navigation
- 🔌 **Full cluster connection and context management** — Discover kubeconfig contexts, connect/disconnect, track connection state, and switch namespaces
- 🔎 **Search and settings** — Search cached resources quickly and manage user-facing preferences from the UI
- 🧾 **Audit logging** — Record key local actions for traceability in the desktop app

### Workloads, networking, and configuration
- 📦 **Comprehensive workload management** — Browse and manage Pods, Deployments, StatefulSets, DaemonSets, Jobs, and CronJobs
- 🌐 **Service and networking views** — Inspect Services and Ingresses alongside related cluster state
- ⚙️ **Configuration resources** — Work with ConfigMaps, Secrets, and PersistentVolumeClaims
- 🧩 **CRD browsing** — Explore installed CustomResourceDefinitions and their schemas/details

### Operations and troubleshooting
- 📋 **Pod operations** — View logs, exec into containers, and start port-forwards
- 🗑️ **Resource actions** — Scale workloads, delete resources, and trigger rollout operations with safety checks
- ⛵ **Helm release management** — List releases, inspect history/values, and support Helm workflows
- 📊 **Node management and metrics** — Inspect nodes plus pod and node metrics for cluster health

### Desktop experience
- 🖥️ **Native Tauri app** — Desktop delivery using Rust backend commands over IPC
- 🔁 **Shared frontend** — `apps/web` contains the SvelteKit UI that is packaged into the desktop app

### AKS-first experience
- 🔑 **Auth Detection** — Identifies exec/token/certificate auth, kubelogin guidance
- 🏷️ **Node Pool Grouping** — Nodes organized by AKS agent pool with VM size, OS, mode
- 🛡️ **Add-on Status** — Container Insights, Azure Policy, Key Vault CSI, KEDA, Flux health
- 🌐 **Azure Portal Links** — One-click "Open in Azure Portal" for AKS clusters
- 🔐 **Workload Identity** — Azure identity bindings visible on pod detail
- 🚨 **Production Guardrails** — Red banner, type-to-confirm for destructive ops in production

### Architecture
- **Rust Engine** — kube-rs client, watchers, actions, logs, exec, port-forwarding, Helm, metrics, and CRD support
- **Tauri Desktop** — Native WebView shell for the packaged frontend
- **SvelteKit Frontend** — `apps/web` provides the UI bundled into the desktop application

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
3. Select a context — pods and resources load in real-time
4. Navigate using the sidebar: Workloads (Pods, Deployments, StatefulSets, DaemonSets, Jobs, CronJobs), Network, Config, Storage, Events

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

# Frontend
pnpm -C apps/web test         # Unit tests
pnpm -C apps/web build        # Production build for desktop packaging
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
See [docs/ROADMAP.md](docs/ROADMAP.md) for the broader milestone plan.

Today Telescope ships as a **desktop-first Tauri application** with broad Kubernetes coverage for day-to-day cluster operations.

Current priorities:

- **Desktop depth** — keep expanding advanced workflows such as interactive exec, broader mutation coverage, and richer port-forward management
- **Desktop hardening** — strengthen security, auditability, and native packaging quality across supported platforms
- **More live workflows** — continue expanding streaming updates and operational feedback loops
- **UX polish and testing** — keep improving performance, usability, and coverage across Rust, frontend, and desktop flows
- **Extensibility over time** — leave room for future integrations and plugin-style capabilities without blocking the current core IDE experience

## License
MIT

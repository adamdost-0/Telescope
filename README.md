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
| **AKS integration** | Native | None | None |
| **Azure ARM ops** | Built-in | None | None |
| **GUI** | Desktop | Desktop | Terminal |
| **Open source** | MIT | Proprietary | Apache 2.0 |

## Highlights

- **AKS-first:** Native Azure ARM management for cluster lifecycle and node pools
- **28+ resource types:** Real-time watch-backed browsing with detail views
- **Operator tooling:** Logs, exec, port-forward, Helm, metrics, resource actions
- **Desktop-native:** Rust + Tauri + SvelteKit, optimized for responsiveness
- **Production-aware:** Guardrails, identity visibility, and local audit logging

For the full feature matrix, see [docs/FEATURES.md](docs/FEATURES.md).

## Quick Start

### Download a Release

Get the latest build from [GitHub Releases](https://github.com/adamdost-0/Telescope/releases).

| Platform | Recommended | Alternatives |
|---|---|---|
| **Windows** | `.msi` installer | `.exe` setup, portable `.exe` |
| **macOS** | `.dmg` installer | portable binary |

Every release includes SHA256 checksums for verification.

### Build from Source

```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
pnpm -C apps/desktop dev     # Development mode
pnpm -C apps/desktop bundle  # Release build
```

Prerequisites: Rust, Node.js 22+, pnpm. See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for platform-specific setup.

### Connect to a Cluster

1. Launch Telescope
2. Your kubeconfig contexts appear automatically
3. Select a context — resources load in real time
4. Navigate using the sidebar: Workloads, Network, Config, Storage, RBAC, Nodes, Events, Helm, CRDs, Azure

## Documentation

| Document | Description |
|---|---|
| [Features](docs/FEATURES.md) | Full feature matrix |
| [Architecture](docs/ARCHITECTURE.md) | System design, crate layering, data flow |
| [AKS Quick Start](docs/AKS_QUICKSTART.md) | Connect to AKS in under 5 minutes |
| [Development](docs/DEVELOPMENT.md) | Prerequisites, commands, project structure |
| [Deployment](docs/DEPLOYMENT.md) | Build, bundle, and distribute |
| [Security](docs/SECURITY.md) | Threat model, secret handling, audit logging |
| [Testing](docs/TESTING.md) | Test pyramid and CI policy |
| [Roadmap](docs/ROADMAP.md) | Post-v1.0.0 priorities |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Changelog](CHANGELOG.md) | Release history |

## License

[MIT](LICENSE)

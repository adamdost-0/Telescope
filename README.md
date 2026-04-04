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

### Build and Test from Source

```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope

# Recommended: run the CI-like Rust + web + Playwright checks in the dev container
./scripts/dev-test.sh
```

The container workflow is the recommended contributor path for build/test work. Open the repo in the included `.devcontainer/devcontainer.json` (for example, VS Code/Cursor: `Dev Containers: Reopen in Container`) or use the Docker commands below. The image provides Rust, Node.js, build tools, and Playwright browsers out of the box.

Need an interactive shell instead of the one-shot script?

```bash
./scripts/dev-test.sh shell
# or: pnpm run dev:container
```

If you need to run the native Tauri desktop shell or build installers on your host OS, use the local bootstrap fallback:

```bash
./scripts/bootstrap-dev.sh
./scripts/pnpm.sh -C apps/desktop dev
./scripts/pnpm.sh -C apps/desktop bundle
```

See [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) for the full container-first workflow and [docs/TESTING.md](docs/TESTING.md) for the exact validation commands.

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
| [Development](docs/DEVELOPMENT.md) | Container-first setup, fallback host bootstrap, project structure |
| [Deployment](docs/DEPLOYMENT.md) | Build, bundle, and distribute |
| [Security](docs/SECURITY.md) | Threat model, secret handling, audit logging |
| [Testing](docs/TESTING.md) | Containerized validation flow, test pyramid, CI policy |
| [Roadmap](docs/ROADMAP.md) | Post-v1.0.0 priorities |
| [Contributing](CONTRIBUTING.md) | How to contribute |
| [Changelog](CHANGELOG.md) | Release history |

## License

[MIT](LICENSE)

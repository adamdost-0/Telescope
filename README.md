# Telescope

Desktop Kubernetes IDE with native Azure AKS integration.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/adamdost-0/Telescope)](https://github.com/adamdost-0/Telescope/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/adamdost-0/Telescope/ci.yml?branch=main&label=CI)](https://github.com/adamdost-0/Telescope/actions/workflows/ci.yml)

<!-- Screenshot: overview dashboard -->

Telescope is a desktop Kubernetes IDE built with Rust and Tauri v2 that gives
platform engineers and cluster operators a single pane of glass for Kubernetes
resources and Azure AKS management. It is a native desktop application -- not a
web service or an Electron wrapper -- designed for low memory overhead and fast
startup on Windows and macOS.

## Key Features

### Kubernetes

| Capability | Details |
|---|---|
| Resource browsing | 28+ watched resource types with real-time updates |
| Pod operations | Streaming logs, interactive exec, port-forward |
| Helm management | Release listing, history, and lifecycle actions |
| CRD browser | Discover and inspect custom resource definitions |
| Metrics | Node and pod resource utilization |
| Resource actions | Scale, restart, cordon/uncordon, drain |
| Namespace scoping | Multi-namespace and all-namespace views |

### Azure AKS

| Capability | Details |
|---|---|
| ARM management | Node pool CRUD, cluster lifecycle, upgrade orchestration |
| Maintenance windows | Visibility into planned maintenance configuration |
| Multi-cloud | Commercial, Government, Secret, and Top Secret environments |
| Identity resolution | Entra ID principal display for RBAC bindings |

### AI Insights

| Capability | Details |
|---|---|
| Cluster analysis | BYOK Azure OpenAI integration for resource diagnostics |

### Security

| Capability | Details |
|---|---|
| Binary pinning | Trusted binary verification for bundled tools |
| Cache redaction | Sensitive fields stripped from local resource cache |
| Scoped identity | Least-privilege credential handling per connection |
| Exec audit | Redacted audit logging for exec and port-forward sessions |

### Developer Experience

| Capability | Details |
|---|---|
| Container-first validation | Dev container with Rust, Node.js, and Playwright pre-installed |
| E2E coverage | 39 Playwright end-to-end tests across core workflows |
| CI pipeline | Automated Rust, frontend, and desktop build checks |

## Quick Start

### Download a Release

Prebuilt installers are available from
[GitHub Releases](https://github.com/adamdost-0/Telescope/releases).
Each release includes SHA256 checksums for verification.

| Platform | Recommended | Alternatives |
|---|---|---|
| **Windows** | `.msi` installer | `.exe` setup, portable `.exe` |
| **macOS** | `.dmg` installer | Portable binary |

### Build and Test from Source

```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
./scripts/dev-test.sh
```

The container workflow is the recommended path. It runs CI-equivalent Rust,
frontend, and Playwright checks inside a pre-configured dev container. For an
interactive shell instead:

```bash
./scripts/dev-test.sh shell
```

To build the native desktop shell or produce installers on your host OS:

```bash
./scripts/bootstrap-dev.sh
./scripts/pnpm.sh -C apps/desktop dev
./scripts/pnpm.sh -C apps/desktop bundle
```

See [Development](docs/DEVELOPMENT.md) for the full container-first workflow
and [Testing](docs/TESTING.md) for validation commands.

### Connect to a Cluster

1. Launch Telescope.
2. Kubeconfig contexts appear automatically.
3. Select a context -- resources load in real time.
4. Navigate the sidebar: Workloads, Network, Config, Storage, RBAC, Nodes, Events, Helm, CRDs, Azure.

<!-- Screenshot: cluster context selection -->

## Architecture

Telescope is a Cargo + pnpm monorepo with a strict layering model.

```
apps/desktop (Tauri v2 shell)
  |-- apps/web (SvelteKit frontend, packaged into desktop)
  |-- crates/engine (Kubernetes client, watchers, actions, Helm, logs, exec)
  |-- crates/azure (Azure ARM client, AKS management-plane operations)
  |-- crates/core (shared domain types, state machine, SQLite resource store)
```

- **crates/core** -- Shared domain types, connection state machine, and
  SQLite-backed resource persistence.
- **crates/engine** -- Kubernetes client with 28+ resource watchers, pod
  log/exec/port-forward support, Helm integration, metrics, CRDs, and audit.
- **crates/azure** -- Azure ARM client for AKS cluster and node pool
  management across Commercial, Government, Secret, and Top Secret clouds.
- **apps/web** -- SvelteKit 5 frontend consumed by the desktop shell via
  Tauri IPC. Not a standalone web application.
- **apps/desktop** -- Tauri v2 packaging layer with 66+ registered IPC
  commands.

Dependency direction: `desktop -> engine -> core`, `desktop -> azure -> core`.

For the full system design, see [Architecture](docs/ARCHITECTURE.md).

## Documentation

| Document | Description |
|---|---|
| [Features](docs/FEATURES.md) | Full feature matrix |
| [Architecture](docs/ARCHITECTURE.md) | System design, crate layering, data flow |
| [AKS Quick Start](docs/AKS_QUICKSTART.md) | Connect to AKS in under 5 minutes |
| [Development](docs/DEVELOPMENT.md) | Container-first setup, host bootstrap, project structure |
| [Testing](docs/TESTING.md) | Containerized validation, test pyramid, CI policy |
| [Security](docs/SECURITY.md) | Threat model, secret handling, audit logging |
| [Deployment](docs/DEPLOYMENT.md) | Build, bundle, and distribute |
| [Roadmap](docs/ROADMAP.md) | Post-v1.0.0 priorities |
| [Changelog](CHANGELOG.md) | Release history |

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for
guidelines on code style, commit conventions, and the review process.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for
details.

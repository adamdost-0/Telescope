---
title: Development
nav_order: 9
description: "Prerequisites, build commands, and local development workflow for Telescope"
---

# Telescope — Development Guide

This guide covers everything needed to build, run, and test Telescope from source. For contribution guidelines, see [CONTRIBUTING.md](https://github.com/adamdost-0/Telescope/blob/main/CONTRIBUTING.md).

## Prerequisites

- **Rust** (stable toolchain)
- **Node.js** 22+
- **pnpm** 9.15+
- **Git**

### Platform SDK

| Platform | Requirements |
|----------|-------------|
| **macOS** | Xcode Command Line Tools |
| **Windows** | Microsoft Edge WebView2 |
| **Linux** | WebKitGTK 4.1, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev` |

Linux builds do not require system OpenSSL development packages; the remaining OpenSSL dependency is vendored during the build.

**Debian/Ubuntu:**
```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## Clone and Install

```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
```

## Project Structure

### Cargo workspace
- `crates/core` — shared domain, state, and storage types
- `crates/engine` — Kubernetes engine: client, watchers, logs, exec, port-forward, actions, Helm, metrics, CRDs
- `crates/azure` — Azure ARM management client for AKS lifecycle, node pools, upgrades, and diagnostics
- `apps/desktop/src-tauri` — Tauri desktop shell and IPC command surface

### pnpm workspace
- `apps/web` — SvelteKit frontend source packaged into the desktop app
- `apps/desktop` — Tauri build/package wrapper for the native desktop distribution
- `packages/*` — shared workspace packages

## Command Reference

### Desktop

```bash
pnpm -C apps/desktop dev      # Development mode with hot reload
pnpm -C apps/desktop build    # Debug desktop build
pnpm -C apps/desktop bundle   # Release bundle (platform-specific installers)
```

### Rust

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

### Frontend

```bash
pnpm -C apps/web test         # Vitest unit tests
pnpm -C apps/web build        # Production build for desktop packaging
pnpm -C apps/web e2e          # Playwright E2E tests
```

### k3d Integration Testing

```bash
./scripts/k3d-setup.sh                                             # Create local cluster with fixtures
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d   # Run integration tests
./scripts/k3d-teardown.sh                                          # Clean up
```

## How the Desktop Build Works

1. `apps/desktop/scripts/prepare-frontend.mjs` builds `apps/web` in production mode.
2. The built frontend output is copied into `apps/desktop/dist`.
3. Tauri packages that output as the shipped desktop UI.

All user-facing UI work happens in `apps/web` even though it ships inside the Tauri desktop shell.

## Azure ARM Features (Optional)

If you want to develop or test AKS management-plane features:

- Install Azure CLI (`az`) and sign in with `az login`
- Ensure your identity has appropriate RBAC roles on AKS resources (see [Deployment]({{ site.baseurl }}/DEPLOYMENT#azure-arm-features))

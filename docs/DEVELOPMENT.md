---
title: Development
nav_order: 9
description: "Prerequisites, build commands, and local development workflow for Telescope"
---

# Telescope — Development Guide

This guide covers everything needed to build, test, and run Telescope from source. The containerized build/test workflow is the required validation path -- all changes must pass `./scripts/dev-test.sh` before being pushed to any branch or merged to `main`. Use host bootstrap only when you need native Tauri windows or platform-specific packaging. For contribution guidelines, see [CONTRIBUTING.md](https://github.com/adamdost-0/Telescope/blob/main/CONTRIBUTING.md).

## Required workflow: containerized build/test environment

Use the provided dev container for day-to-day Rust work, frontend work, and CI-like validation. **This is the mandatory validation gate** -- all changes must pass the containerized suite before being pushed to any branch.

### Requirements

- **Docker** (or a compatible `docker` CLI/runtime)
- **Git**

### What the container includes

`tools/devtest/Dockerfile` builds a repeatable Linux environment with:

- Rust stable toolchain (`cargo`, `rustfmt`, `clippy`)
- Node.js + npm from the Playwright base image
- `build-essential` and `pkg-config` for Rust crate builds
- Playwright browsers and Linux browser dependencies preinstalled at `/ms-playwright`

### Open the included devcontainer

If you use VS Code, Cursor, or another Dev Containers-compatible editor:

1. Clone the repo and open the `Telescope` folder.
2. Run **Dev Containers: Reopen in Container**.
3. The included `.devcontainer/devcontainer.json` builds from `tools/devtest/Dockerfile`.
4. Its post-create setup prepares the repo-pinned pnpm version and installs workspace dependencies. The image already includes the Playwright browsers and Linux desktop libraries.

Once the container finishes opening, use the normal repo commands from the integrated terminal:

```bash
cargo test --workspace --exclude telescope-desktop --all-features
./scripts/pnpm.sh -C apps/web test
./scripts/pnpm.sh -C apps/web e2e
```

### Quick start

```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope

# Full Rust + web validation in the dev container (required before push)
./scripts/dev-test.sh
```

**Gate rule:** Do not push a branch, open a PR, or merge to `main` until `./scripts/dev-test.sh` passes. This single command validates the same checks CI enforces.

`./scripts/dev-test.sh` builds the local `telescope-devtest:local` image, mounts your checkout at `/repo`, and runs the standard validation stack:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings`
- `cargo test --workspace --exclude telescope-desktop --all-features`
- `./scripts/pnpm.sh install --frozen-lockfile`
- `./scripts/pnpm.sh -C apps/web test`
- `./scripts/pnpm.sh -C apps/web e2e`

### Interactive container shell

Use this when you want to rerun individual commands, investigate failures, or stay inside the same environment while editing on the host.

If you are already using the included devcontainer above, you can skip the manual shell startup below. Otherwise, use the container helper:

```bash
./scripts/dev-test.sh shell
# or: pnpm run dev:container
```

Inside the container:

```bash
# Usual build/test commands
./scripts/pnpm.sh install --frozen-lockfile
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
./scripts/pnpm.sh -C apps/web build
./scripts/pnpm.sh -C apps/web test
./scripts/pnpm.sh -C apps/web e2e
```

The repo is mounted from your host, so edits made in your normal editor are immediately visible inside the container shell.

## Host fallback: native desktop/Tauri development

Use a local host install **only** when you need one of these:

- `apps/desktop` hot reload with a native Tauri window
- platform-specific desktop build/bundle output
- optional cloud CLIs such as `az`, `kubectl`, and `helm`

**Important:** Even when using the host fallback for desktop work, run `./scripts/dev-test.sh` before pushing to validate the Rust and frontend changes in the container.

### Host prerequisites

- **Rust** (stable toolchain)
- **Node.js** 22+
- **pnpm** 9.15+ (or the repo-managed wrapper prepared by Corepack)
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

### One-command bootstrap (host fallback)

If your machine is missing the Rust toolchain, a C compiler, Node 22, repo-local pnpm, or the Linux/macOS desktop prerequisites, run:

```bash
./scripts/bootstrap-dev.sh
```

To also install the optional AKS/Kubernetes CLIs used for management-plane work:

```bash
./scripts/bootstrap-dev.sh --with-cloud-tools
```

The bootstrap script installs or refreshes the local prerequisites for native desktop development, including Rust, Node.js, the repo-pinned pnpm toolchain, Tauri system packages, workspace dependencies, and Playwright Chromium.

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

### Run these in the dev container or after host bootstrap

#### Rust

```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

#### Frontend

```bash
./scripts/pnpm.sh -C apps/web test    # Vitest unit tests
./scripts/pnpm.sh -C apps/web build   # Production build for desktop packaging
./scripts/pnpm.sh -C apps/web e2e     # Playwright E2E tests
```

### Host-only desktop commands

```bash
./scripts/pnpm.sh -C apps/desktop dev      # Development mode with hot reload
./scripts/pnpm.sh -C apps/desktop build    # Debug desktop build
./scripts/pnpm.sh -C apps/desktop bundle   # Release bundle (platform-specific installers)
```

### k3d Integration Testing

Run these on a Docker-enabled host (or a privileged container you manage yourself); they are not part of the default `dev-test.sh` loop.

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

- Install Azure CLI (`az`) and sign in with `az login` on your host
- Ensure your identity has appropriate RBAC roles on AKS resources (see [Deployment]({{ site.baseurl }}/DEPLOYMENT#azure-arm-features))

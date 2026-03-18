# Contributing

Thanks for contributing to Telescope.

## Getting Started

### Prerequisites
- Rust stable toolchain
- Node.js 22+
- pnpm 9.15+
- Git
- Tauri platform dependencies:
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft Edge WebView2
  - Linux: WebKitGTK 4.1, libgtk-3-dev, libayatana-appindicator3-dev, librsvg2-dev

#### Linux Build Notes

On Debian/Ubuntu systems:

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

System OpenSSL development packages are not required for the Azure dependency path; the remaining OpenSSL dependency is vendored during the build.

### Clone and install
```bash
git clone https://github.com/adamdost-0/Telescope.git
cd Telescope
pnpm install
```

## Development Workflow

### Rust workspace
```bash
cargo fmt --all -- --check
cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings
cargo test --workspace --exclude telescope-desktop --all-features
```

### Frontend app
```bash
pnpm -C apps/web dev
pnpm -C apps/web test
pnpm -C apps/web e2e
```

### Desktop app
```bash
pnpm -C apps/desktop dev
pnpm -C apps/desktop build
pnpm -C apps/desktop bundle
```

### Practical notes
- Telescope ships as a desktop-only Tauri app; all user-facing UI work still happens in `apps/web`.
- `apps/desktop` packages the built `apps/web` output for the native app.
- Prefer repo-defined commands and existing CI workflows over ad hoc scripts.
- Keep changes desktop-focused; do not reintroduce separate browser or hub flows in docs or code.

## Project Structure

### Cargo workspace
- `crates/core` - shared domain, state, and storage types
- `crates/engine` - Kubernetes engine code: clients, watchers, logs, exec, port-forward, Helm, metrics, CRDs
- `crates/azure` - Azure ARM management client for AKS lifecycle, node pools, upgrades, and diagnostics
- `apps/desktop/src-tauri` - Tauri desktop backend and IPC commands

### pnpm workspace
- `apps/web` - SvelteKit frontend source packaged into the desktop app
- `apps/desktop` - Tauri desktop shell that packages the frontend

## k3d Integration Testing

Telescope includes a k3d-based integration test workflow for the Kubernetes engine (see `.github/workflows/integration.yml`). These tests run against a real local Kubernetes cluster.

### Running locally

```bash
# Install k3d
curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash

# Create test cluster
k3d cluster create telescope-test

# Deploy test fixtures
kubectl apply -f tools/k3d-fixtures/

# Run integration tests
K3D_TEST=1 cargo test -p telescope-engine --test integration_k3d -- --ignored

# Cleanup
k3d cluster delete telescope-test
```

### CI behavior

The `integration.yml` workflow triggers on pushes to `main` that change `crates/engine/`, `crates/core/`, `tools/k3d-fixtures/`, or the workflow file itself. It can also be triggered manually via `workflow_dispatch`. CI creates a k3d cluster, deploys the fixture manifests, runs the integration tests, and tears down the cluster automatically.

## Pull Request Process
- Open PRs with the repository PR template in `.github/pull_request_template.md`.
- Describe the change clearly, including scope, user-visible behavior, and any risks.
- Include a short test plan and note whether unit, E2E, desktop, or CI changes were needed.
- Ensure the relevant CI checks pass before requesting review.
- Keep PRs focused; split unrelated changes into separate submissions.

## Code Style
- Rust: run `cargo fmt` and fix all `clippy` warnings required by CI.
- TypeScript/Svelte: follow existing patterns in the app, prefer Svelte 5 runes, and keep code straightforward.
- Avoid excessive comments; add them only when they clarify non-obvious behavior.
- When documentation and code differ, treat the current code and CI behavior as the source of truth.

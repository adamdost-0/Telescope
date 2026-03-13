# Deployment

> **Current status:** Telescope is a desktop-only Tauri application. Deployment guidance focuses on building, bundling, and distributing the native app.

## Desktop Deployment

### Prerequisites

- **Supported platforms:** macOS, Windows, and Linux.
- **Toolchains:**
  - Rust (stable)
  - Node.js 22+
  - pnpm 9.15+
- **Platform SDK/runtime dependencies:**
  - **macOS:** Xcode command-line tools
  - **Windows:** Windows SDK and WebView2-capable environment
  - **Linux:** GTK 3, WebKit2GTK, OpenSSL development libraries, and other Tauri system dependencies

### Build from source

The desktop package builds the `apps/web` frontend first, then bundles that output into the Tauri shell.

```bash
pnpm install && pnpm -C apps/desktop build
```

Notes:

- `pnpm -C apps/desktop build` runs `prepare:frontend`, which builds `apps/web` and copies its output into `apps/desktop/dist` before invoking `tauri build --debug`.
- Use this for a local debug-style native build.

### Bundle for distribution

To produce native release artifacts for the host platform:

```bash
pnpm -C apps/desktop bundle
```

This also rebuilds the frontend and then runs a release Tauri bundle build.

### Platform-specific notes

#### macOS

- Distribution builds typically need Apple code signing and notarization outside of Telescope itself.
- Build on macOS hosts with Xcode command-line tools installed.
- Expect macOS-native artifacts such as `.app` and DMG/PKG-style outputs depending on your signing and packaging setup.

#### Windows

- Build on Windows with the Windows SDK available.
- Tauri bundle builds produce native installer artifacts for Windows distributions.
- Unsigned builds may trigger Windows Defender / SmartScreen warnings.

#### Linux

- Linux desktop builds are possible, but this repository does not currently run desktop Linux bundles in CI.
- Install the required GTK/WebKit system libraries before building.

## Configuration

### Kubeconfig setup

- Telescope relies on the local machine's Kubernetes configuration.
- Ensure the user running Telescope has a valid kubeconfig with the contexts they expect to use.
- Desktop usage is local-first: the app discovers contexts from the local kubeconfig and talks to the cluster from the user's workstation.

### Network / firewall requirements

- Outbound access from the user workstation to the Kubernetes API servers referenced in kubeconfig.
- If exec auth plugins are used, local tooling such as `kubelogin` or the Azure CLI must also be available to the user session.

### TLS considerations

- The desktop Tauri configuration ships with a restrictive CSP for packaged frontend assets.
- Cluster transport security depends on the Kubernetes endpoints referenced in kubeconfig and any local enterprise proxy configuration.

## Production checklist

- [ ] Verify kubeconfig access for every context the user needs.
- [ ] Package builds on the target operating system.
- [ ] Complete platform signing/notarization requirements for distributed binaries.
- [ ] Confirm required desktop runtime dependencies are installed on managed endpoints.
- [ ] Validate connectivity to target Kubernetes APIs and any required exec-auth helpers.

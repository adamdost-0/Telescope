# Deployment

> **Current status:** Telescope v1.0.0 is a desktop-only Tauri application. There is no hub/web server mode. Deployment guidance focuses on building, bundling, and distributing the native app.

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
  - **Linux:** GTK 3, WebKit2GTK, and other Tauri system dependencies (system OpenSSL development packages are not required; the remaining OpenSSL dependency is vendored during the build)
- **Azure ARM features (optional):**
  - Azure CLI (`az`) installed and signed in — used for AKS identity resolution
  - Azure RBAC permissions on AKS resources (see Azure ARM Features below)

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
- Outbound access to Azure Resource Manager endpoints if using ARM features (`management.azure.com` for Commercial, `management.usgovcloudapi.net` for Government).
- If exec auth plugins are used, local tooling such as `kubelogin` or the Azure CLI must also be available to the user session.

### TLS considerations

- The desktop Tauri configuration ships with a restrictive CSP for packaged frontend assets.
- Cluster transport security depends on the Kubernetes endpoints referenced in kubeconfig and any local enterprise proxy configuration.

## Azure ARM Features

Telescope includes native Azure ARM integration for AKS cluster management. These features require Azure credentials and RBAC permissions — they do not use the Kubernetes API.

### Authentication

ARM operations use `DefaultAzureCredential`, which automatically chains:
1. Environment variables (`AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET`)
2. Azure CLI (`az login`)
3. Managed identity (when running on Azure)
4. Workload identity

The simplest setup for desktop use is to sign in with `az login` before launching Telescope.

### Required Azure RBAC permissions

| Operation | Minimum RBAC role |
|-----------|------------------|
| View cluster details, node pools, upgrades | `Reader` on AKS resource |
| Start/stop cluster | `Azure Kubernetes Service Contributor` |
| Upgrade cluster version | `Azure Kubernetes Service Contributor` |
| Scale/create/delete node pools | `Azure Kubernetes Service Contributor` |
| Update autoscaler configuration | `Azure Kubernetes Service Contributor` |
| Upgrade node pool version/image | `Azure Kubernetes Service Contributor` |
| View maintenance configurations | `Reader` on AKS resource |

### Azure Government support

Telescope supports multiple Azure cloud environments via the `AzureCloud` setting:
- **Commercial** — `management.azure.com`
- **US Government** — `management.usgovcloudapi.net`
- **US Gov Secret** — air-gapped secret cloud
- **US Gov Top Secret** — air-gapped top-secret cloud

Set the cloud via the `set_azure_cloud` command or the settings page.

### AKS identity resolution

Telescope resolves the AKS resource identity (subscription, resource group, cluster name) needed for ARM calls using this resolution order:
1. Saved preferences (from a previous session)
2. Azure CLI (`az aks list` matching by FQDN)
3. URL-based hints from the kubeconfig server address

## Production checklist

- [ ] Verify kubeconfig access for every context the user needs.
- [ ] Package builds on the target operating system.
- [ ] Complete platform signing/notarization requirements for distributed binaries.
- [ ] Confirm required desktop runtime dependencies are installed on managed endpoints.
- [ ] Validate connectivity to target Kubernetes APIs and any required exec-auth helpers.
- [ ] For Azure ARM features: verify `az login` or equivalent credential source is configured.
- [ ] For Azure ARM features: verify RBAC roles are assigned on target AKS resources.

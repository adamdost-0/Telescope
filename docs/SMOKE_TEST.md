# Telescope — Local k3d Smoke Test

> **Scope:** This checklist validates core Kubernetes features using a local k3d cluster. Azure ARM features (node pool management, cluster start/stop, upgrades) require a real AKS cluster and are not testable with k3d.

## Prerequisites
- [ ] Docker installed and running
- [ ] k3d installed (`curl -s https://raw.githubusercontent.com/k3d-io/k3d/main/install.sh | bash`)
- [ ] Rust toolchain installed
- [ ] pnpm installed
- [ ] Desktop platform SDK (WebView2 on Windows, WebKit on Linux)

## Automated Smoke Test
```bash
./scripts/smoke-test.sh
```

## Manual Desktop Smoke Test

### Setup
```bash
./scripts/k3d-setup.sh
pnpm install
pnpm -C apps/desktop dev
```

### Test Cases

| # | Test | Steps | Expected | Pass? |
|---|------|-------|----------|-------|
| 1 | App launches | Start desktop app | Window opens with Telescope header, "Disconnected" status | ☐ |
| 2 | Context list | Check context dropdown | Shows `k3d-telescope-dev` (and any other kubeconfig contexts) | ☐ |
| 3 | Connect | Select `k3d-telescope-dev` | Status changes: Connecting → Syncing → Ready (green) | ☐ |
| 4 | Pod list | Navigate to Pods page | Shows ~21 pods (20 nginx + 1 crashloop) | ☐ |
| 5 | Pod status | Check pod statuses | nginx pods: Running (green), crashloop-test: CrashLoopBackOff (red) | ☐ |
| 6 | Namespace switch | Select `telescope-test` namespace | Pod list updates to show 3 echo-server pods | ☐ |
| 7 | Namespace switch back | Select `default` namespace | Pod list returns to ~21 pods | ☐ |
| 8 | Disconnect | Stop k3d: `k3d cluster stop telescope-dev` | Status changes to Backoff/Error, UI shows last known data | ☐ |
| 9 | Reconnect | Start k3d: `k3d cluster start telescope-dev` | Status returns to Ready, pod data refreshes | ☐ |
| 10 | Memory | Check Task Manager / Activity Monitor | < 200 MB with 50 pods connected | ☐ |

### Azure ARM Features (requires real AKS cluster)

> ⚠️ These tests **cannot** be run with k3d. They require:
> - A real AKS cluster in an Azure subscription
> - Azure CLI signed in (`az login`)
> - Appropriate RBAC roles on the AKS resource

| # | Test | Steps | Expected | Pass? |
|---|------|-------|----------|-------|
| A1 | AKS identity resolution | Connect to an AKS context | AKS identity auto-resolved (subscription, resource group, cluster) | ☐ |
| A2 | Node pool list | Navigate to Azure Node Pools | Shows node pools with VM size, count, version | ☐ |
| A3 | Node pool scale | Scale a user pool | Node count changes, ARM polling completes | ☐ |
| A4 | Cluster detail | Check overview page | Shows AKS-specific detail (SKU, network profile, identity) | ☐ |
| A5 | Upgrade profile | Check upgrade availability | Shows available Kubernetes versions | ☐ |

### Teardown
```bash
./scripts/k3d-teardown.sh
```

## Reporting Results
Create a file `docs/retrospectives/SMOKE-TEST-<date>.md` with results.

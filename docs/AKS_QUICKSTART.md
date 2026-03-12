# Telescope — AKS Quick Start

## Prerequisites
- Azure CLI installed (`az`)
- kubelogin installed (`az aks install-cli`)
- AKS cluster with Azure AD enabled

## Step 1: Get AKS Credentials
```bash
az aks get-credentials --resource-group <rg> --name <cluster> --overwrite-existing
kubelogin convert-kubeconfig -l devicecode
```

## Step 2: Launch Telescope
Download from [Releases](https://github.com/adamdost-0/Telescope/releases) or build:
```bash
pnpm -C apps/desktop dev
```

## Step 3: Connect
1. Your AKS context appears automatically (shows 🔑 Exec auth badge)
2. Complete the device code auth when prompted
3. Status changes to ● Connected (green)

## Step 4: Explore
- **Overview**: Cluster health, resource counts, pod phases
- **Pods**: Real-time pod list with CPU/memory metrics
- **Workloads**: Deployments, StatefulSets, DaemonSets, Jobs, and CronJobs each have dedicated blades with detail views
- **Helm**: View and manage Helm releases (aks-managed-* releases visible)
- **Nodes**: AKS node pools grouped by pool name + VM size
- **Events**: Cluster-wide events including kube-system

## AKS-Specific Features
- **Node Pool Grouping**: Nodes organized by agentpool with VM size and OS
- **Add-on Status**: See Container Insights, Azure Policy, Key Vault CSI health
- **Portal Links**: "Open in Azure Portal" button for AKS clusters
- **Workload Identity**: Azure identity bindings on pod detail pages
- **Production Guardrails**: Red banner + type-to-confirm for production clusters

## Troubleshooting
- **"kubelogin not found"**: Run `az aks install-cli` to install
- **"Unauthorized"**: Run `kubelogin convert-kubeconfig -l devicecode` again
- **No pods showing**: Check namespace selector — switch from 'default' to 'kube-system'
- **No metrics**: Ensure metrics-server is enabled on your AKS cluster

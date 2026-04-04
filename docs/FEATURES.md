---
title: Features
nav_order: 2
description: "Full feature matrix for Telescope — Kubernetes, Azure ARM, Helm, and desktop capabilities"
---

# Telescope — Feature Matrix

> **Status: v1.2.0+** — This page reflects the shipped feature surface. For planned work, see [Roadmap]({{ site.baseurl }}/ROADMAP).

## Cluster Connection and Navigation

| Capability | Details |
|---|---|
| Full cluster connection and context management | Discover kubeconfig contexts, connect/disconnect, track connection state, and switch namespaces |
| Search and settings | Search cached resources quickly and manage user-facing preferences from the UI |
| Audit logging | Record key local actions for traceability in the desktop app |

## Kubernetes Operations

| Capability | Details |
|---|---|
| Broad resource coverage | Browse and manage workloads, networking, configuration, storage, policy, RBAC, and admission resources with 28+ watched Kubernetes resource types |
| CRD browsing | Explore installed CustomResourceDefinitions and dynamic resources with schema/details support |
| Pod operations | View logs, exec into containers, and start port-forwards |
| Resource actions | Scale workloads, delete resources, create namespaces, apply YAML, and trigger rollout operations with safety checks |
| Helm release management | List releases, inspect history/values, and support Helm rollback and uninstall workflows |
| Node management and metrics | Inspect nodes plus pod and node metrics for cluster health |

## Azure ARM Management Plane

| Capability | Details |
|---|---|
| Native ARM client | Manage AKS from the desktop app without leaving Telescope |
| Node pool CRUD | List, create, scale, autoscale, upgrade, and delete AKS node pools |
| Cluster lifecycle control | Start and stop AKS clusters, inspect upgrade profiles, and manage cluster upgrades |
| ARM-sourced diagnostics | View maintenance configs, upgrade readiness, and platform diagnostics sourced from Azure management APIs |
| Multi-cloud support | Works across Azure Commercial, Government, Secret, and Top Secret cloud environments |

## Desktop Experience

| Capability | Details |
|---|---|
| Native Tauri app | Rust backend commands over IPC with 60+ desktop commands exposed to the UI |
| SvelteKit frontend | `apps/web` contains the frontend source that is packaged into the desktop application by Tauri |
| Ephemeral local cache | Telescope clears the SQLite resource cache on startup, disconnect, and app exit; Secrets stay on-demand only, and cached Pod/workload env values, commands, annotations, ConfigMap data, and webhook client payloads are redacted before they hit disk |

## AKS-First Experience

| Capability | Details |
|---|---|
| Auth detection | Identifies exec/token/certificate auth, blocks untrusted kubeconfig exec helpers, and provides kubelogin guidance |
| Node pool grouping | Nodes organized by AKS agent pool with VM size, OS, and mode |
| Add-on status | Container Insights, Azure Policy, Key Vault CSI, KEDA, and Flux health |
| Azure Portal links | One-click navigation for AKS clusters |
| Workload identity visibility | Azure identity bindings shown on pod detail views |
| Production guardrails | Red banner and type-to-confirm flows for destructive ops in production |

## AI Insights (v1.2.0)

| Capability | Details |
|---|---|
| BYOK Azure OpenAI integration | Bring-your-own-key model with user-configured Azure OpenAI endpoint and deployment |
| Dual auth modes | Azure Entra ID via `DefaultAzureCredential` or session-only API Key (never persisted) |
| Multi-cloud support | Azure Commercial, Government, Secret, and Top Secret cloud profiles |
| Allowlist-only context builder | Curated cluster context with redaction of secrets, tokens, and credential material; deterministic per-category size caps |
| Structured JSON responses | Summary, risks, observations, recommendations, and resource references rendered deterministically in the UI |
| Per-cluster history | Generated insights stored per cluster scope with clear-all control |
| Dedicated Insights route | `/insights` page with test connection, generate, history review, and clear actions |
| Settings page configuration | Endpoint, deployment/model name, cloud profile, and auth mode managed from the Settings page |

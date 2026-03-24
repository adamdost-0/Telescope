---
title: Home
layout: home
nav_order: 1
description: "Telescope — Desktop-first Kubernetes IDE with native Azure AKS integration"
permalink: /
---

# Telescope Documentation
{: .fs-9 }

A desktop-first Kubernetes IDE built with Tauri, SvelteKit, and Rust — designed for operators who manage AKS clusters and need deep, native tooling without the Electron tax.
{: .fs-6 .fw-300 }

[Get Started with AKS]({{ site.baseurl }}/AKS_QUICKSTART){: .btn .btn-primary .fs-5 .mb-4 .mb-md-0 .mr-2 }
[View on GitHub](https://github.com/adamdost-0/Telescope){: .btn .fs-5 .mb-4 .mb-md-0 }

---

## What is Telescope?

Telescope is a **shipped v1.0.0 desktop application** that gives Kubernetes operators a native IDE experience:

- **28+ watched Kubernetes resource types** with real-time cache updates
- **66 Tauri IPC commands** — the full desktop command surface
- **Native Azure ARM integration** for AKS cluster lifecycle, node pools, upgrades, and maintenance
- **Helm release management** including list, detail, history, values, rollback, and uninstall
- **Pod workflows** — logs, exec, port-forward, events, YAML editing
- **CRD discovery and browsing** with dynamic resource support
- **Audit logging** for all destructive operations
- **Secret redaction** by default across the entire UI

{: .note }
> Telescope is desktop-only — no Electron, no browser mode. Built with Tauri v2 for a lightweight native footprint.

---

## Documentation

### Getting Started
{: .text-delta }

| Document | Description |
|:---------|:------------|
| [Features]({{ site.baseurl }}/FEATURES) | Full feature matrix — Kubernetes, Azure ARM, Helm, and desktop capabilities |
| [AKS Quick Start]({{ site.baseurl }}/AKS_QUICKSTART) | Connect Telescope to an AKS cluster in under 5 minutes |
| [Deployment]({{ site.baseurl }}/DEPLOYMENT) | Download, build, bundle, and distribute the desktop application |
| [Development]({{ site.baseurl }}/DEVELOPMENT) | Prerequisites, build commands, and local development workflow |

### Architecture and Design
{: .text-delta }

| Document | Description |
|:---------|:------------|
| [Architecture]({{ site.baseurl }}/ARCHITECTURE) | System design, crate layering, watch model, IPC surface, and data flow |
| [UX Reference]({{ site.baseurl }}/UX_NOTES) | Route inventory, navigation patterns, component catalog |

### Operations
{: .text-delta }

| Document | Description |
|:---------|:------------|
| [Security]({{ site.baseurl }}/SECURITY) | Threat model, secret handling, audit logging, credential management |
| [Testing]({{ site.baseurl }}/TESTING) | Test pyramid — Rust unit tests, Vitest, Playwright E2E |
| [Smoke Test]({{ site.baseurl }}/SMOKE_TEST) | Local k3d validation checklist |

### Planning
{: .text-delta }

| Document | Description |
|:---------|:------------|
| [Roadmap]({{ site.baseurl }}/ROADMAP) | Post-v1.0.0 milestones and priorities |
| [Changelog](https://github.com/adamdost-0/Telescope/blob/main/CHANGELOG.md) | Release history and version notes |
| [Contributing](https://github.com/adamdost-0/Telescope/blob/main/CONTRIBUTING.md) | How to contribute to Telescope |

---

## Tech Stack

```
┌─────────────────────────────────────────┐
│           Tauri v2 Desktop Shell        │
│  ┌───────────────────────────────────┐  │
│  │     SvelteKit Frontend (39 routes)│  │
│  │     apps/web · Svelte 5 runes     │  │
│  └───────────────┬───────────────────┘  │
│                  │ Tauri IPC             │
│  ┌───────────────┴───────────────────┐  │
│  │         Rust Backend              │  │
│  │  ┌─────────┐ ┌────────┐ ┌──────┐ │  │
│  │  │ engine  │ │ azure  │ │ core │ │  │
│  │  └────┬────┘ └───┬────┘ └──┬───┘ │  │
│  │       └──────────┴─────────┘     │  │
│  └───────────────────────────────────┘  │
└─────────────────────────────────────────┘
        │                    │
   Kubernetes API      Azure ARM API
```

---

## Quick Links

- [GitHub Releases](https://github.com/adamdost-0/Telescope/releases) — Download the latest build
- [Contributing Guide](https://github.com/adamdost-0/Telescope/blob/main/CONTRIBUTING.md) — How to contribute
- [License](https://github.com/adamdost-0/Telescope/blob/main/LICENSE) — Project license

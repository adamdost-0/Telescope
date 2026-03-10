# Telescope

A memory-efficient Kubernetes IDE (AKS-first) designed to compete with Lens without the Electron bloat.

## Principles
- **No Electron**: desktop uses **Tauri** (native WebView) + a native backend.
- **Memory-first**: watch-driven, on-demand informers; virtualized lists; bounded logs.
- **AKS-first**: Azure AD auth flows, cluster/nodepool awareness, optional Azure API actions behind explicit permissions.
- **Dual client**: Desktop app + Web client over the same backend API.
- **Safe by default**: read-only first, diff + server-side dry-run before apply, secrets redaction.

## Status
Very early. See:
- `docs/PRD.md`
- `docs/ARCHITECTURE.md`
- `docs/ROADMAP.md`
- `docs/SECURITY.md`

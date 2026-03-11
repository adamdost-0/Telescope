# Telescope — Roadmap

## M0 — Foundations
- ✅ Repo governance + CI pipelines
- ✅ Rust workspace skeleton (core/engine/api crates)
- ✅ Desktop shell boots (Tauri v2)
- ✅ Web scaffold (SvelteKit routes)
- 🔲 Rust engine API contract *(moved to M1)*
- 🔲 Desktop connects to engine *(moved to M1)*
- 🔲 Kubeconfig import + context switch *(moved to M1)*

## M1 — MVP “Debug Loop”
- Resource explorer (core kinds) + dynamic discovery
- Workload detail: logs + exec + events
- Port-forward with profiles + auto-reconnect
- AKS connect flow (Azure auth + kubeconfig merge)
- Memory-first caching + watch lifecycle management

## M2 — Helm + Metrics
- Helm releases list/detail, upgrade/rollback, values, diff
- Metrics-server: top + basic charts

## M3 — Web client + Hub mode
- Hub service container image
- Web UI connects to hub via OIDC
- Per-user access + audit log

## M4 — Plugins v1
- WASM plugin host + permissions
- First-party plugins: Helm, AKS tools, Prometheus metrics

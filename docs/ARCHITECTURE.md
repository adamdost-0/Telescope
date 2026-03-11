# Telescope — Architecture (Draft)

> ⚠️ **Status: Aspirational Design** — This document describes the *target* architecture. The current codebase is scaffold-only (v0.0.1). See the [Roadmap](ROADMAP.md) for implementation status.

## Goals
- Avoid Electron; keep resident memory low.
- One shared Kubernetes engine powering desktop and web.
- Watch-driven, on-demand data flow; no "watch the whole cluster" by default.
- Secure handling of kubeconfig/tokens/secrets.

## High-level design
### Components
1) **Engine (Rust)**
- Kubernetes client, watchers/informers, cache/index, query layer.
- Streams deltas to clients (resource add/update/delete).
- Provides:
  - **gRPC** API for structured calls + streaming
  - optional **WS/SSE gateway** for browser streaming

2) **Desktop app (Tauri)**
- Thin shell hosting the UI.
- Runs Engine embedded or as a local daemon.

3) **Web client**
- Same UI codebase.
- Connects to an Engine running as:
  - local daemon (single-user), or
  - **Hub** service (team/self-host) with OIDC.

## Data flow (memory-first)
- LIST → populate cache → WATCH from resourceVersion → send deltas.
- Start watchers only for visible scopes (cluster/ns/kind) and stop when not needed.
- Coalesce bursts into frames; apply backpressure.
- Hard caps on caches (Events/Pods/log lines), with LRU/TTL eviction.

## Storage
- Persist only what is necessary:
  - UI preferences, pinned contexts, recent clusters
  - optional warm cache metadata
- Suggested: **SQLite** (portable) + optional compression.

## Plugin system
- Default: **WASM plugins** (wasmtime) with a strict host API.
- Permissions manifest per plugin (capability-based).
- Optional advanced mode: external plugins via gRPC.

## Security model
- Kubeconfig references stored; avoid copying unless requested.
- Tokens stored only if necessary, encrypted using OS keychain envelope.
- Secrets masked; do not persist Secret values by default.
- Read-only default; destructive operations require diff + server-side dry-run.

## AKS-specific
- Support kubeconfig `exec` auth (kubelogin/az flows).
- Token refresh UX.
- Optional Azure API integration (nodepool scale/upgrade) behind explicit permissions.

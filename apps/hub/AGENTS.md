# Agent Guidance — Telescope Hub Server

## Overview

`apps/hub` is an Axum 0.8 HTTP server that mirrors the desktop's Tauri IPC commands as REST endpoints. It is used in browser/web mode when no Tauri runtime is present.

Key capabilities:
- REST API under `/api/v1/` for all read operations (contexts, connect, resources, pods, events, secrets, Helm, metrics, CRDs, search, audit)
- WebSocket streaming at `/ws`
- OIDC authentication scaffolding (disabled by default, enabled by `OIDC_ENABLED=true`)
- SQLite `ResourceStore` + `ResourceWatcher` for cached resource data (same architecture as desktop)
- Audit log (append-only JSONL file at `AUDIT_PATH`)

## Current State

**What works:**
- All listed `/api/v1/` read endpoints are implemented
- `connect` and `disconnect` endpoints start/stop the kube-rs ResourceWatcher
- SQLite-backed resource store (shared between watcher and request handlers)
- OIDC middleware (auth disabled by default; when enabled, JWT claims are decoded but NOT signature-verified)
- Audit log writes for destructive actions
- `/healthz`, `/auth/me`, `/auth/logout` endpoints

**NOT production-ready:**
- **OIDC: JWT signatures are NOT validated** — `auth.rs` decodes payload only (no `iss`/`aud`/`exp` checks)
- **CORS is permissive** — `CorsLayer::permissive()` allows all origins
- **No rate limiting, circuit breakers, or DDoS protection**
- **Write operations not exposed** — scale, delete, apply, exec, port-forward are desktop-only for now

## Build and Run Commands

```bash
# Run locally (connects to ~/.kube/config, port 3001)
cargo run -p telescope-hub

# Run with debug logging
RUST_LOG=debug cargo run -p telescope-hub

# Run tests
cargo test -p telescope-hub

# Build release binary
cargo build -p telescope-hub --release

# Docker build (basic, not production-hardened)
docker build -f apps/hub/Dockerfile -t telescope-hub .
```

## Configuration (Environment Variables)

| Variable | Default | Description |
|---|---|---|
| `PORT` | `3001` | TCP port to bind |
| `DB_PATH` | `/tmp/telescope-hub/resources.db` | SQLite database path |
| `AUDIT_PATH` | `/tmp/telescope-hub/audit.log` | Audit log file path (JSONL) |
| `RUST_LOG` | `info,telescope_engine=debug` | Log level filter |
| `OIDC_ENABLED` | (unset) | Set to `true` to require JWT Bearer tokens |
| `OIDC_ISSUER_URL` | (unset) | OIDC provider URL (required when OIDC_ENABLED=true) |
| `OIDC_CLIENT_ID` | (unset) | OIDC client ID (required when OIDC_ENABLED=true) |
| `OIDC_REDIRECT_URI` | `http://localhost:3001/auth/callback` | OIDC redirect URI |

On startup, hub ensures `DB_PATH` and `AUDIT_PATH` parent directories exist, creates the audit file with `0o600` permissions (Unix), and clears stale resource data from the previous run.

## API Endpoints

All data endpoints are nested under `/api/v1/` and protected by `auth_middleware`.

### Unauthenticated Routes

```
GET  /healthz                    # Health check — returns "ok"
GET  /auth/login                 # OIDC login redirect (returns 501 until OIDC provider is configured)
GET  /auth/callback              # OIDC callback (returns 501)
POST /auth/logout                # Logout (returns 200)
GET  /auth/me                    # Returns current AuthUser identity (requires auth middleware)
WS   /ws                         # WebSocket resource stream
```

### Authenticated REST Routes (`/api/v1/`)

```
GET  /api/v1/contexts                              # List kubeconfig contexts
POST /api/v1/connect                               # Connect to a context; starts ResourceWatcher
POST /api/v1/disconnect                            # Disconnect; stops ResourceWatcher, clears store
GET  /api/v1/connection-state                      # Current ConnectionState (Disconnected/Syncing/Ready/…)
GET  /api/v1/resources?gvk=&namespace=             # Get resources from store by GVK + optional namespace
GET  /api/v1/pods?namespace=                       # Get pod resources
GET  /api/v1/events?namespace=&involved_object=    # Get event resources
GET  /api/v1/namespaces                            # List namespace names from store
GET  /api/v1/secrets?namespace=                    # Fetch secrets on-demand from Kubernetes API (not from store)
GET  /api/v1/secrets/{namespace}/{name}            # Fetch one secret on-demand
GET  /api/v1/pods/{namespace}/{name}/logs          # Fetch pod logs (?container=&tail=&previous=)
GET  /api/v1/cluster-info                          # Server version, auth type, AKS detection
GET  /api/v1/search?q=                             # Search resource names across all cached GVKs
GET  /api/v1/helm/releases?namespace=              # List Helm releases
GET  /api/v1/metrics/pods?namespace=               # Pod CPU/memory metrics from metrics-server
GET  /api/v1/crds                                  # List Custom Resource Definitions
GET  /api/v1/audit?limit=                          # Read recent audit log entries
```

### `AuthUser` Identity

When `OIDC_ENABLED=true`, the JWT Bearer token is decoded and the `AuthUser` struct (email, name, groups) is injected via Axum `Extension`. When disabled, all requests are anonymous (`anonymous@local`). The `actor` field in audit log entries comes from `user.email`.

## Source Layout

```
apps/hub/src/
├── main.rs      # Axum app setup, route registration, CORS, tracing
├── auth.rs      # OidcConfig, AuthUser, auth_middleware, JWT decode, auth routes
├── routes.rs    # All /api/v1/* request handlers
├── state.rs     # HubState: ResourceStore (Mutex), ConnectionState (RwLock), active context/namespace, audit path
└── ws.rs        # WebSocket handler
```

## Architecture: ResourceStore + ResourceWatcher

Hub uses the same storage architecture as the desktop:

1. `HubState` owns an `Arc<Mutex<ResourceStore>>` (SQLite, `crates/core`)
2. On `POST /api/v1/connect`, a `ResourceWatcher` (kube-rs, `crates/engine`) is spawned
3. The watcher streams resources into `ResourceStore` via the shared `Arc<Mutex<_>>`
4. Request handlers query `ResourceStore` directly — no separate cache
5. On `POST /api/v1/disconnect`, the watch task is aborted and all resource data is cleared

## Security Status

**CRITICAL — DO NOT deploy to public internet:**

1. **No JWT signature verification** — `auth.rs:decode_jwt_claims` decodes payload without verifying signature
2. **Permissive CORS** — `CorsLayer::permissive()` in `main.rs` allows any origin
3. **OIDC flow incomplete** — login/callback routes return 501
4. **No authorization model** — authenticated users can access all clusters/namespaces
5. **No rate limiting**

## Testing

- Unit tests in `src/**/*.rs` (auth decoding, state management)
- CI runs `cargo test -p telescope-hub`
- No integration tests yet — API contract is exercised by the `apps/web` E2E suite (against stub server)

## Development Workflow

```bash
# Start hub (uses ~/.kube/config current context)
cargo run -p telescope-hub

# Verify it works
curl http://localhost:3001/healthz
curl http://localhost:3001/api/v1/contexts

# Connect to a cluster
curl -X POST http://localhost:3001/api/v1/connect \
  -H 'Content-Type: application/json' \
  -d '{"contextName":"my-context"}'
```

## Code Conventions

- Axum handlers use `State<Arc<HubState>>` for shared state
- Auth user is accessed via `Extension(user): Extension<AuthUser>`
- Error responses use `ApiResult<T>` = `Result<Json<T>, (StatusCode, Json<ErrorResponse>)>`
- Use `tracing` spans/events for request observability; include `actor` in destructive ops
- Engine calls are async — use `.await` and propagate errors via `api_err()`

## CI Integration

CI runs (`ci.yml` Rust job):
- `cargo clippy -p telescope-hub` (warnings are errors)
- `cargo test -p telescope-hub`

CI does NOT build a Docker image or deploy hub.

## What's Missing (High Priority)

- JWT signature validation (OIDC integration with Azure Entra ID)
- Secure CORS allowlist
- Authorization model (per-user/group cluster access)
- Rate limiting
- Write operation endpoints (scale, delete, apply, exec, port-forward)
- Production Dockerfile (non-root user, health probes, multi-stage)
- Helm chart / Kubernetes manifests
- Integration tests for API endpoints
- Metrics/observability hooks

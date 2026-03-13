# Agent Guidance — Telescope Hub Server

## Overview

`apps/hub` is an Axum-based HTTP server providing:

1. REST API for cluster/namespace/resource queries
2. WebSocket streaming for real-time resource updates
3. OIDC authentication scaffolding (dev-only, not production-ready)

Built with Rust, using `telescope-engine` and `telescope-core` crates for Kubernetes logic.

## Current State (v0.0.1)

**What works:**
- Axum HTTP server with CORS middleware
- `/api/v1/*` REST endpoints for cluster queries, mutations, Helm, metrics, and preferences
- kube-rs integration for real cluster queries and imperative actions
- Basic WebSocket setup (partial implementation)
- Tracing and audit logging

**What's NOT production-ready:**
- **OIDC authentication is scaffolding only:**
  - `/auth/login` and `/auth/callback` return `501 Not Implemented`
  - JWT decoding exists but **does NOT validate signatures**
  - No session management or token refresh
- **CORS is permissive:** `CorsLayer::permissive()` allows all origins
- **No rate limiting, circuit breakers, or retry logic**
- **No production deployment config** (Dockerfile exists but basic)

**Treat this as a dev/demo server, not a deployable backend.**

## Build and Run Commands

```bash
# Run locally (connects to current kubectl context)
cargo run -p telescope-hub

# Run tests
cargo test -p telescope-hub

# Build release binary
cargo build -p telescope-hub --release

# Run with custom env vars
RUST_LOG=debug cargo run -p telescope-hub

# Docker build (basic, not production-hardened)
docker build -f apps/hub/Dockerfile -t telescope-hub .
```

## Configuration

Environment variables:

- `RUST_LOG` — Log level (e.g., `info`, `debug`, `telescope_hub=trace`)
- `TELESCOPE_BIND` — Server bind address (default: `127.0.0.1:8080`)
- (Future) `TELESCOPE_OIDC_*` — OIDC provider config (not implemented yet)

## API Endpoints

### REST Routes

```
GET  /api/v1/contexts
POST /api/v1/connect
POST /api/v1/disconnect
GET  /api/v1/connection-state
GET  /api/v1/active-context
GET  /api/v1/namespace
POST /api/v1/namespace
GET  /api/v1/resources
POST /api/v1/resources/delete
POST /api/v1/resources/apply
POST /api/v1/resources/scale
POST /api/v1/rollout/restart
GET  /api/v1/rollout/status
POST /api/v1/exec
POST /api/v1/port-forward
GET  /api/v1/containers/{namespace}/{pod}
GET  /api/v1/pods/{namespace}/{name}/logs
GET  /api/v1/secrets
GET  /api/v1/secrets/{namespace}/{name}
GET  /api/v1/events
GET  /api/v1/namespaces
GET  /api/v1/cluster-info
GET  /api/v1/search
GET  /api/v1/helm/releases
GET  /api/v1/helm/releases/{namespace}/{name}/history
GET  /api/v1/helm/releases/{namespace}/{name}/values
POST /api/v1/helm/releases/{namespace}/{name}/rollback
GET  /api/v1/metrics/pods
GET  /api/v1/metrics/nodes
GET  /api/v1/metrics/available
GET  /api/v1/resource-counts
GET  /api/v1/crds
GET  /api/v1/preferences/{key}
PUT  /api/v1/preferences/{key}
GET  /api/v1/audit
GET  /healthz
```

### WebSocket (partial)

```
WS   /ws                                # Resource updates stream (scaffolded)
```

### Auth Routes (scaffolding only)

```
GET  /auth/login                        # Returns 501 Not Implemented
GET  /auth/callback                     # Returns 501 Not Implemented
```

## Security Status

**CRITICAL:** This server is NOT production-ready. Known issues:

1. **No signature verification on JWT tokens** — `auth.rs` decodes JWTs without validating the signature
2. **Permissive CORS** — `CorsLayer::permissive()` in `main.rs` allows any origin
3. **OIDC flow is incomplete** — login/callback routes are stubs
4. **No authorization checks** — all API routes are open (after auth scaffolding is bypassed)
5. **No rate limiting or DDoS protection**

See `docs/SECURITY.md` for planned security model (aspirational).

## Architecture

```
apps/hub/src/
├── main.rs          # Axum app setup, CORS, tracing
├── auth.rs          # OIDC scaffolding (NOT production-ready)
├── routes.rs        # All API handlers (clusters, namespaces, resources, etc.)
├── state.rs         # Shared app state
└── ws.rs            # WebSocket scaffolding (partial)
```

## Dependencies

Key crates:

- `axum` (0.8) — HTTP framework
- `tower-http` — CORS and tracing middleware
- `kube` (3.0+) — Kubernetes client
- `tokio` — Async runtime
- `serde` / `serde_json` — Serialization
- `tracing` / `tracing-subscriber` — Observability

## Testing

- Unit tests in `src/**/*.rs` (`#[cfg(test)]` modules)
- CI runs `cargo test -p telescope-hub`
- **No integration tests yet** — API testing happens in `apps/web` E2E suite (against stubs)

## Deployment

**Docker:**

A basic `Dockerfile` exists but is NOT production-hardened:
- No multi-stage build optimization
- No non-root user
- No health checks or readiness probes
- No secret management

**Kubernetes:**

No Helm chart or K8s manifests exist yet. See `docs/AKS_QUICKSTART.md` for aspirational deployment guide.

## Development Workflow

1. **Start hub locally:** `cargo run -p telescope-hub` (uses your local `~/.kube/config`)
2. **Test with curl:**
   ```bash
   curl http://localhost:8080/api/clusters
   curl http://localhost:8080/health
   ```
3. **Run tests:** `cargo test -p telescope-hub`
4. **Check logs:** Set `RUST_LOG=debug` for verbose output

## Code Conventions

- Async handlers using `axum::extract::State` for shared state
- Error handling via `anyhow` (return `Result<Json<T>, StatusCode>` from handlers)
- kube-rs `Client` initialized once and shared via Axum state
- Tracing spans for request logging

## CI Integration

CI runs:
- `cargo clippy -p telescope-hub` (warnings are errors)
- `cargo test -p telescope-hub`

CI does NOT deploy or validate runtime behavior beyond compile/test.

## What's Missing (High Priority)

- Real OIDC implementation with signature validation
- Secure CORS configuration (allowlist specific origins)
- Authorization middleware (check user permissions)
- Rate limiting and circuit breakers
- Production Dockerfile and Helm chart
- Integration tests for API endpoints
- Metrics and observability hooks
- WebSocket implementation completion
- Multi-cluster connection management
- Secret rotation and secure credential handling

## Guidance for Changes

- **Adding an endpoint:** Create handler in `src/routes/`, register in `main.rs`
- **Fixing auth:** Edit `src/auth.rs` — add signature verification, real OIDC flow
- **Hardening CORS:** Replace `CorsLayer::permissive()` in `main.rs` with specific origins
- **Deploying:** Wait for production hardening — do NOT deploy current version to public internet

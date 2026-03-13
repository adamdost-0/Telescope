# Telescope Hub

`apps/hub` is the Axum-based HTTP server for Telescope's browser/web mode. It exposes REST endpoints under `/api/v1`, provides auth-related routes, and hosts a WebSocket endpoint used for real-time integration work.

It is built on top of:

- `telescope-engine` for Kubernetes operations
- `telescope-core` for shared domain/state types
- `axum`, `tokio`, `tower-http`, and `tracing` for the HTTP runtime

## What it does

Telescope Hub is the backend that the shared SvelteKit frontend uses when it is running in the browser instead of inside Tauri. In practice, `apps/web/src/lib/api.ts` maps frontend commands to Hub endpoints such as contexts, connect/disconnect, resources, pods, events, namespaces, logs, Helm, metrics, CRDs, search, and audit.

## Run locally

```bash
cargo run -p telescope-hub
```

By default the server listens on `0.0.0.0:3001`.

## Environment variables

These are the environment variables currently read from `main.rs` and `auth.rs`:

- `PORT` — HTTP port for the server (default: `3001`)
- `DB_PATH` — path to the local SQLite-backed resource store (default: `/tmp/telescope-hub/resources.db`)
- `AUDIT_PATH` — path to the audit log file (default: `/tmp/telescope-hub/audit.log`)
- `RUST_LOG` — tracing filter for log verbosity; when unset, Hub defaults to `info,telescope_engine=debug`
- `OIDC_ENABLED` — when `true`, enables auth middleware that expects a bearer token
- `OIDC_ISSUER_URL` — issuer URL for OIDC configuration
- `OIDC_CLIENT_ID` — OIDC client/application ID
- `OIDC_REDIRECT_URI` — callback URL for OIDC login flow (default: `http://localhost:3001/auth/callback`)

Notes:

- `auth.rs` currently hard-codes the requested scopes to `openid profile email`.
- If `OIDC_ENABLED` is not set to `true`, requests run as an anonymous local user.
- The login/callback flow is scaffolded; `/auth/login` and `/auth/callback` are placeholders today.

## API overview

Main endpoint groups exposed by the server:

- **Health**
  - `GET /healthz`
- **Authentication**
  - `GET /auth/login`
  - `GET /auth/callback`
  - `POST /auth/logout`
  - `GET /auth/me`
- **Contexts and connection state**
  - `GET /api/v1/contexts`
  - `POST /api/v1/connect`
  - `POST /api/v1/disconnect`
  - `GET /api/v1/connection-state`
- **Resources and workloads**
  - `GET /api/v1/resources`
  - `GET /api/v1/pods`
  - `GET /api/v1/namespaces`
  - `GET /api/v1/secrets`
  - `GET /api/v1/secrets/{namespace}/{name}`
- **Observability and troubleshooting**
  - `GET /api/v1/events`
  - `GET /api/v1/pods/{namespace}/{name}/logs`
  - `GET /api/v1/cluster-info`
  - `GET /api/v1/search`
  - `GET /api/v1/audit`
- **Platform features**
  - `GET /api/v1/helm/releases`
  - `GET /api/v1/metrics/pods`
  - `GET /api/v1/crds`
- **WebSocket**
  - `GET /ws`

## Architecture

- `src/main.rs` — builds shared state, reads environment variables, sets up tracing/CORS, wires routes, and starts the Axum server
- `src/routes.rs` — request handlers for contexts, connection lifecycle, resources, pods, events, namespaces, secrets, logs, cluster info, search, Helm, metrics, CRDs, and audit
- `src/auth.rs` — auth middleware, user extraction, OIDC environment parsing, and auth route handlers
- `src/ws.rs` — WebSocket handler; currently provides a basic welcome/echo loop scaffold
- `src/state.rs` — shared application state including the resource store, active context/namespace, audit path, and watch task coordination

## Testing

```bash
cargo test -p telescope-hub
```

This runs the crate's Rust tests, including auth-related unit tests.

## Current implementation notes

- Hub protects `/api/v1/*` with auth middleware, but when OIDC is disabled all requests are treated as anonymous.
- When OIDC is enabled, bearer tokens are decoded for claims extraction; full signature validation is not implemented yet.
- The WebSocket endpoint exists, but it is currently a lightweight scaffold rather than a full streaming layer.

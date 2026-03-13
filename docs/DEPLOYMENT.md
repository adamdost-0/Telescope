# Deployment

> **Current status:** The desktop app is the most complete deployment target today. `apps/hub` is a working Axum service, but its OIDC flow is still scaffold-only and should be treated as development/demo infrastructure unless you harden it further.

## Desktop Deployment

### Prerequisites

- **Supported platforms:** macOS, Windows, and Linux.
- **Toolchains:**
  - Rust (stable)
  - Node.js 22+
  - pnpm 9.15+
- **Platform SDK/runtime dependencies:**
  - **macOS:** Xcode command-line tools
  - **Windows:** Windows SDK and WebView2-capable environment
  - **Linux:** GTK 3, WebKit2GTK, OpenSSL development libraries, and other Tauri system dependencies

### Build from source

The desktop package builds the shared `apps/web` frontend first, then bundles that output into the Tauri shell.

```bash
pnpm install && pnpm -C apps/desktop build
```

Notes:

- `pnpm -C apps/desktop build` runs `prepare:frontend`, which builds `apps/web` and copies its output into `apps/desktop/dist` before invoking `tauri build --debug`.
- Use this for a local debug-style native build.

### Bundle for distribution

To produce native release artifacts for the host platform:

```bash
pnpm -C apps/desktop bundle
```

This also rebuilds the shared frontend and then runs a release Tauri bundle build.

### Platform-specific notes

#### macOS

- Distribution builds typically need Apple code signing and notarization outside of Telescope itself.
- Build on macOS hosts with Xcode command-line tools installed.
- Expect macOS-native artifacts such as `.app` and DMG/PKG-style outputs depending on your signing and packaging setup.

#### Windows

- Build on Windows with the Windows SDK available.
- Tauri bundle builds produce native installer artifacts for Windows distributions.
- Unsigned builds may trigger Windows Defender / SmartScreen warnings.

#### Linux

- Linux desktop builds are possible, but this repository does not currently run desktop Linux bundles in CI.
- Install the required GTK/WebKit system libraries before building.

## Hub / Web Deployment

### Prerequisites

- Rust (stable)
- A reachable Kubernetes API using a kubeconfig the Hub process can read
- Filesystem paths writable for the Hub SQLite cache and audit log
- Optional OIDC provider metadata for authenticated deployments

### Environment variables

The Hub reads the following environment variables from `apps/hub/src/main.rs` and `apps/hub/src/auth.rs`:

| Variable | Default | Required | Purpose |
|---|---|---:|---|
| `PORT` | `3001` | No | TCP port the Hub binds to on `0.0.0.0`. |
| `DB_PATH` | `/tmp/telescope-hub/resources.db` | No | SQLite cache file for watched cluster resources. |
| `AUDIT_PATH` | `/tmp/telescope-hub/audit.log` | No | Audit log file written as JSON lines. |
| `OIDC_ENABLED` | `false` | No | Enables bearer-token auth middleware when set to `true`. |
| `OIDC_ISSUER_URL` | none | Yes, if `OIDC_ENABLED=true` | OIDC issuer URL. |
| `OIDC_CLIENT_ID` | none | Yes, if `OIDC_ENABLED=true` | OIDC client/application ID. |
| `OIDC_REDIRECT_URI` | `http://localhost:3001/auth/callback` | No | Redirect URI recorded in the runtime OIDC config. |

Additional OIDC details:

- The current implementation hard-codes scopes to `openid profile email`.
- `/auth/login` and `/auth/callback` are placeholders and currently return `501 Not Implemented`.
- When OIDC is enabled, the middleware only decodes JWT payload claims; it does **not** validate token signatures yet.

### Run locally

```bash
cargo run -p telescope-hub
```

By default the Hub listens on `http://0.0.0.0:3001` and exposes:

- `GET /healthz`
- REST endpoints under `/api/v1`
- auth endpoints under `/auth/*`
- WebSocket upgrades on `/ws`

Local health check:

```bash
curl http://localhost:3001/healthz
```

### Docker deployment

A basic Dockerfile already exists at `apps/hub/Dockerfile`:

```bash
docker build -f apps/hub/Dockerfile -t telescope-hub .
```

Current Dockerfile characteristics:

- multi-stage Rust build
- Debian runtime image with CA certificates
- exposes port `3001`
- starts `telescope-hub`

Recommended operational guidance:

- Mount a kubeconfig into the container for the runtime user so context-based endpoints can read cluster contexts.
- Persist `DB_PATH` and `AUDIT_PATH` to a writable volume if you want cache and audit data to survive restarts.
- Put TLS termination in front of the container; the Hub itself does not configure HTTPS listeners.
- For air-gapped environments, mirror the Dockerfile base images into your internal registry before building.

### Kubernetes deployment

There is no checked-in Helm chart or production manifest set yet, so Kubernetes deployment is currently manual. At minimum, your manifest should provide:

- a `Deployment` (or equivalent) running the Hub container
- a `Service` exposing the Hub port
- an `Ingress` or internal reverse proxy for HTTPS termination if browser clients connect remotely
- a mounted kubeconfig readable by the container user for context-based endpoints like `/api/v1/contexts` and `/api/v1/connect`
- writable storage for `DB_PATH` and `AUDIT_PATH` if you need persistence

Basic guidance:

- Set `PORT=3001` unless you have a reason to change it.
- Keep `/healthz` as the liveness/readiness probe target.
- If you enable OIDC, route `/auth/callback` through the same externally reachable host name you register with your IdP.
- Restrict inbound access to the Hub service; `main.rs` binds on all interfaces.

### OIDC configuration

Set these environment variables to turn on auth middleware:

```bash
export OIDC_ENABLED=true
export OIDC_ISSUER_URL="https://login.microsoftonline.us/<tenant>/v2.0"
export OIDC_CLIENT_ID="<client-id>"
export OIDC_REDIRECT_URI="https://<your-host>/auth/callback"
```

Behavior notes:

- All `/api/v1/*` routes require a `Bearer` token when `OIDC_ENABLED=true`.
- `GET /auth/me` also requires auth when OIDC is enabled.
- When OIDC is disabled, requests run as the anonymous user `anonymous@local`.
- The current login/callback routes are placeholders, so you must already have a bearer token source if you enable OIDC.

### Health check

The Hub exposes a simple unauthenticated health endpoint:

- **Method:** `GET`
- **Path:** `/healthz`
- **Response:** plain text `ok`

## Configuration

### Kubeconfig setup

#### Desktop mode

- The desktop app relies on the local machine's Kubernetes configuration.
- Ensure the user running Telescope has a valid kubeconfig with the contexts you expect to use.
- Desktop usage is local-first: the app discovers contexts from the local kubeconfig and talks to the cluster from the user's workstation.

#### Hub/web mode

- The Hub also expects kubeconfig-backed cluster access.
- `POST /api/v1/connect` and `GET /api/v1/contexts` explicitly depend on reading kubeconfig contexts.
- In containerized/server deployments, mount a kubeconfig into the runtime user's home directory (for example, `~/.kube/config`) and verify the target contexts exist inside that file.
- If the Hub is expected to impersonate authenticated users, the backing Kubernetes identity also needs the RBAC required for impersonation.

### Network / firewall requirements

- **Desktop:** outbound access from the user workstation to the Kubernetes API servers referenced in kubeconfig.
- **Hub:**
  - inbound access to the configured `PORT` (default `3001`) from web clients, reverse proxies, or local operators
  - outbound access to Kubernetes API servers referenced in kubeconfig
  - outbound access to the OIDC issuer when `OIDC_ENABLED=true`
- Allow WebSocket traffic on the same host/port if browser clients use `/ws`.
- If you reverse-proxy the Hub, forward both HTTP and WebSocket traffic.

### TLS considerations

- The Hub does not configure TLS listeners directly; terminate TLS at an ingress controller, reverse proxy, or load balancer.
- Use HTTPS for any remote browser access and for OIDC redirect URIs.
- Protect bearer tokens in transit; when OIDC is enabled, API auth depends on the `Authorization: Bearer ...` header.
- The desktop Tauri configuration ships with a restrictive CSP for packaged frontend assets, but cluster/API transport security still depends on your Kubernetes endpoints and any reverse proxy in front of the Hub.

## Production checklist

- [ ] Enable OIDC auth if the Hub will be shared by multiple users.
- [ ] Treat the current OIDC implementation as scaffold-only until token validation and a real login flow are added.
- [ ] Grant only the Kubernetes RBAC needed by the Hub and any impersonated users/groups.
- [ ] Configure TLS termination in front of the Hub.
- [ ] Expose and monitor `GET /healthz`.
- [ ] Persist and protect `DB_PATH` and `AUDIT_PATH`.
- [ ] Restrict network access to the Hub service and Kubernetes API.
- [ ] Verify the mounted kubeconfig contains every context you expect the Hub to present.

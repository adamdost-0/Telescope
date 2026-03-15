# Entra ID Authentication Feasibility

> **Status:** Feasibility investigation for Telescope desktop and hub/web modes.
> This document reflects the current codebase, not the aspirational state described elsewhere in `docs/`.

## 1. Executive Summary

Telescope does **not** currently implement native Entra ID authentication.

- **Hub/web mode** only has OIDC scaffolding in `apps/hub/src/auth.rs`: `/auth/login` and `/auth/callback` return `501 Not Implemented`, JWT payloads are decoded **without** signature validation, there is no JWKS discovery, and there is no session or refresh-token lifecycle.
- **Desktop mode** relies on kubeconfig auth only. `crates/engine/src/client.rs` and `crates/engine/src/kubeconfig.rs` detect exec/token/certificate auth and surface AKS/kubelogin hints, but `apps/desktop/src-tauri/src/main.rs` has no Entra login commands, no token storage, and no refresh path.
- **Web mode** cannot participate in an authenticated hub session today because `apps/web/src/lib/api.ts` issues plain `fetch()` calls with no `Authorization` header and no `credentials: 'include'` handling.

### Recommendation

Adopt **Option C: Hybrid**.

1. Use **Authorization Code + PKCE** as the primary flow for both runtimes.
   - **Desktop:** launch the system browser from Tauri, receive the callback on a **localhost loopback listener**.
   - **Hub/web:** perform a standard OIDC redirect to hub routes and keep browser auth **hub-mediated** with secure session cookies.
2. Add **Device Code Flow** as the fallback for headless, SSH, locked-down, or loopback-blocked environments.
3. **Do not replace kubelogin in phase 1.** Preserve the existing kubeconfig exec path for Kubernetes API access, and add native Entra auth first for Telescope-managed identity, token-expiry UX, and future ARM-backed features.
4. Use **separate Entra app registrations**:
   - **Hub:** confidential web app
   - **Desktop:** public/native client
5. Prefer **no browser-side MSAL dependency in phase 1**. The web UI should stay thin and let the hub own tokens, cookies, validation, and refresh.

This is feasible, but it is not a small change. A production-ready implementation spans hub middleware, web fetch/session handling, Tauri auth commands, secure token storage, Entra app registration, and explicit failure handling for expiry, group-claim overage, and cross-origin cookie policy.

---

## 2. Current State / Gap Analysis

### 2.1 What exists today

| Area | Current implementation | Evidence | Gap |
|---|---|---|---|
| Hub auth | OIDC config loader, bearer header parsing, unverified JWT payload decode, placeholder `/auth/logout` and `/auth/me` | `apps/hub/src/auth.rs`, `apps/hub/src/main.rs` | No real login/callback flow, no JWKS, no issuer/audience/expiry validation, no refresh, no session management |
| Hub route protection | `/api/v1/*` is wrapped in `auth_middleware`; `/auth/me` is also wrapped | `apps/hub/src/main.rs` | Middleware trusts decoded claims rather than verified identity |
| Hub impersonation | Route handlers can already create kube clients with impersonation headers | `apps/hub/src/routes.rs`, `crates/engine/src/client.rs` | Useful foundation, but only safe once identity is verified and impersonation RBAC is validated |
| Hub WebSocket | Placeholder echo socket | `apps/hub/src/ws.rs` | No auth on upgrade, no token/session expiry handling |
| Desktop auth | Tauri commands call engine/core directly | `apps/desktop/src-tauri/src/main.rs` | No Entra login, no token cache, no identity status, no refresh logic |
| Desktop capabilities | Only core window/event permissions | `apps/desktop/src-tauri/capabilities/default.json` | No opener/deep-link style capability configuration yet |
| Engine auth awareness | Detects `exec`, `token`, `certificate`; detects AKS URLs and kubelogin exec plugin; exposes `auth_hint` | `crates/engine/src/client.rs`, `crates/engine/src/kubeconfig.rs` | Detection only; no native Entra login or refresh orchestration |
| Web API facade | Tauri IPC for desktop, HTTP fallback for browser | `apps/web/src/lib/api.ts` | No auth-aware request wrapper, no bearer/header injection, no cookie/session mode, no 401 refresh behavior |
| Web auth UX | Generic auth-expiry and kubelogin hints | `apps/web/src/lib/error-suggestions.ts`, `apps/web/src/lib/components/ContextSwitcher.svelte` | No identity state, no login/logout UI, no Entra-specific remediation |
| Azure-specific UI | AKS URL parsing and identity/add-on display | `apps/web/src/lib/azure-utils.ts`, `apps/web/src/lib/components/AzureIdentitySection.svelte` | Helpful context only; no Azure sign-in or ARM token model |
| Dependencies | No `oauth2`, `openidconnect`, `azure-identity`, `keyring`, `@azure/msal-browser` | `apps/hub/Cargo.toml`, `apps/desktop/src-tauri/Cargo.toml`, `crates/engine/Cargo.toml`, `apps/web/package.json` | The auth stack must be added from scratch |

### 2.2 Why Entra token expiry causes failures today

The current code explains why Entra expiry turns into weak or silent failure modes:

1. **Desktop has no Telescope-managed token lifecycle.**
   - The engine builds Kubernetes clients from kubeconfig and exec plugins.
   - If the active AKS context depends on `kubelogin`, the renewal path is entirely external to Telescope.
   - Telescope can detect that the cluster is using exec auth (`auth_type == "exec"`) and can show a kubelogin hint, but it cannot proactively refresh, count down expiry, or distinguish Kubernetes token failure from future ARM token failure.

2. **Hub can only decode, not verify.**
   - `apps/hub/src/auth.rs` currently treats any bearer token with a decodable JWT payload as identity.
   - There is no `.well-known/openid-configuration` discovery, no JWKS fetch, and no `exp`, `aud`, `iss`, `nbf`, `nonce`, or `azp` enforcement.

3. **Web mode has no authenticated transport.**
   - `apps/web/src/lib/api.ts` makes `fetch()` calls directly to `${HUB_URL}/api/v1/*` without `Authorization` or `credentials`.
   - Even if the hub had real OIDC, the browser-side API layer could not attach a session or token consistently.

4. **Future ARM-backed features need a different token audience.**
   - The roadmap in `docs/ROADMAP.md` calls out AKS add-on status and Entra awareness.
   - ARM calls require Azure Resource Manager delegated permissions and refresh handling.
   - Those tokens are **not** the same as the kubeconfig exec flow that powers today’s Kubernetes calls.

### 2.3 Code excerpts that illustrate the gap

```rust
// apps/hub/src/auth.rs
pub async fn login() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        "OIDC login not yet configured. Set OIDC_ENABLED=true with issuer/client config.",
    )
}
```

```rust
// apps/hub/src/auth.rs
// TODO: Validate JWT signature, expiry, and audience once a real
// OIDC provider is configured. For now we only decode the payload claims.
```

```ts
// apps/web/src/lib/api.ts
const res = await fetch(`${base}/contexts`);
```

The current implementation is enough to prove feasibility, but not enough to safely authenticate users.

---

## 3. Authentication Flow Options

### 3.1 Option Summary

| Option | User experience | Implementation effort | Operational risk | Fit for Telescope |
|---|---|---:|---:|---|
| **A. Device Code Flow** | Functional but clunky; user copies a code into a browser | Low | Low | Good fallback, weak primary UX for hub/web |
| **B. Authorization Code + PKCE** | Best interactive UX; familiar “Sign in with Entra” | Medium | Medium | Best primary flow for both runtimes |
| **C. Hybrid** | Best overall coverage | Highest | Medium | **Recommended** |

### 3.2 Option A: Device Code Flow

**Protocol:** OAuth 2.0 Device Authorization Grant ([RFC 8628](https://www.rfc-editor.org/rfc/rfc8628))

#### How it works

1. Telescope requests a `device_code` from Entra.
2. Entra returns:
   - `device_code`
   - `user_code`
   - `verification_uri`
   - polling interval
3. Telescope displays the code and URL.
4. The user completes authentication in a browser.
5. Telescope polls the token endpoint until it receives tokens or an error.

#### User experience

- **Desktop:** good fallback when loopback listeners or deep links are blocked.
- **Hub/web:** less natural because the user is already in a browser.
- **Remote/headless:** strongest option.

#### Pros

- No redirect URI required.
- No localhost listener or custom protocol needed.
- Easy to implement server-side and desktop-side.
- Matches existing AKS/kubelogin mental model (`kubelogin convert-kubeconfig -l devicecode` in `docs/AKS_QUICKSTART.md`).

#### Cons

- Context switch: users must leave the app flow and manually enter a code.
- Polling introduces delay and more status handling.
- Poor primary UX for browser/hub mode.
- Still requires secure token storage and refresh once tokens are issued.

#### Implementation path

- **Rust:** use `oauth2` device authorization support.
- **Desktop:** add a Tauri command that returns `{ verificationUri, userCode, expiresIn, interval }`, then poll via Rust and push status into the UI.
- **Hub/web:** expose `/auth/device-code` and a status endpoint or server-sent event stream; render the code in the browser page.
- **Token storage:** desktop keyring; hub server-side cookie/session.

```rust
use oauth2::{DeviceAuthorizationUrl, Scope};

// sketch only
let details = client
    .exchange_device_code()
    .add_scope(Scope::new("openid".into()))
    .add_scope(Scope::new("profile".into()))
    .add_scope(Scope::new("email".into()))
    .request_async(&http_client)
    .await?;
```

#### Entra app registration requirements

- Register a **public client**.
- Enable **device code flow**.
- Request delegated scopes such as `openid`, `profile`, `email`, `offline_access`, and any ARM or Graph scopes actually needed.

#### Feasibility verdict

**Feasible and recommended as fallback**, but not recommended as Telescope’s primary sign-in experience.

---

### 3.3 Option B: Interactive Browser Login (Authorization Code + PKCE)

**Protocol:** OAuth 2.0 Authorization Code Grant with PKCE ([RFC 7636](https://www.rfc-editor.org/rfc/rfc7636))

#### How it works

1. Telescope generates `state`, `nonce`, `code_verifier`, and `code_challenge`.
2. Telescope redirects or opens the system browser to Entra’s authorize endpoint.
3. The user signs in and completes MFA/conditional access.
4. Entra redirects back with an authorization code.
5. Telescope exchanges the code + verifier for tokens.
6. Telescope validates the returned ID token and stores refresh/session state securely.

#### User experience

- **Hub/web:** best experience; standard browser redirect.
- **Desktop:** good experience if the app can launch the system browser and receive the callback reliably.

#### Pros

- Familiar sign-in pattern.
- Supports silent refresh or proactive refresh after the initial login.
- Best fit for web sessions and a branded sign-in button.
- Aligns with hub-mediated cookies and minimal browser-side state.

#### Cons

- Redirect URI management is more involved.
- Desktop needs a callback strategy.
- More moving parts: PKCE, state, nonce, cookie policy, callback errors.

#### Desktop callback choice

| Choice | Pros | Cons | Recommendation |
|---|---|---|---|
| **Loopback localhost** (`http://127.0.0.1:18000/auth/callback`) | Cross-platform, no OS URI registration, works well with Tauri Rust backend | Fixed ports must be pre-registered; some endpoint security tools block listeners | **Recommended for phase 1** |
| **Custom protocol** (`telescope://auth/callback`) | Clean UX, no local port | Requires OS registration, Tauri deep-link setup, and more packaging complexity | Good future enhancement, not the easiest first step |

#### Implementation path

- **Rust (hub):** prefer `openidconnect` for discovery, issuer metadata, nonce, ID token validation, and PKCE.
- **Desktop:** use `tauri-plugin-opener` to open the system browser; receive the callback on a localhost listener bound to a **fixed, pre-registered port range**; validate state before exchange.
- **Hub/web:** standard `/auth/login` -> `/auth/callback` flow, then set secure cookies.

```rust
use openidconnect::{
    AuthorizationCode, CsrfToken, Nonce, PkceCodeChallenge, Scope,
};

// sketch only
let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
let (auth_url, csrf_state, nonce) = client
    .authorize_url(CsrfToken::new_random, Nonce::new_random)
    .add_scope(Scope::new("openid".into()))
    .add_scope(Scope::new("profile".into()))
    .add_scope(Scope::new("email".into()))
    .set_pkce_challenge(pkce_challenge)
    .url();
```

#### Entra app registration requirements

- **Hub:** confidential web app registration, redirect URI such as `https://hub.example.com/auth/callback` and dev URI such as `http://localhost:3001/auth/callback`.
- **Desktop:** public/native client registration with loopback redirect URIs such as `http://127.0.0.1:18000/auth/callback` through a small fixed port range.
- PKCE required for public/native flow.

#### Feasibility verdict

**Feasible and recommended as the primary interactive flow.**

---

### 3.4 Option C: Hybrid (Recommended)

Use **interactive browser login as the default** and **device code as the fallback**.

#### Why this is the best fit

- Preserves a first-class UX for both desktop and web users.
- Covers locked-down and remote environments without redesign.
- Lets Telescope keep the web client thin and secure while still serving desktop users well.
- Aligns with the repo’s current split runtime model:
  - desktop = native Rust/Tauri runtime
  - web = browser UI backed by Axum hub

#### Recommended design decisions

- **Separate Entra app registrations** for hub and desktop.
- **Hub-managed sessions** for browser/web mode.
- **OS keyring** for desktop refresh-token custody.
- **Explicit kubelogin coexistence** in early phases.
- **Shared validation logic** (JWKS, claims normalization, expiry checks) implemented in Rust and reused where possible.

---

## 4. Implementation Requirements

### 4.1 New Rust dependencies

| Dependency | Where | Why |
|---|---|---|
| `oauth2` | `apps/hub`, `apps/desktop/src-tauri` | Device code flow, token exchange primitives |
| `openidconnect` | `apps/hub` and likely desktop auth module as well | Discovery, nonce, PKCE, ID token validation, issuer metadata |
| `jsonwebtoken` | `apps/hub` or shared auth module | Useful if Telescope validates access/session JWTs directly or emits its own signed tokens; otherwise `openidconnect` may cover ID token validation |
| `keyring` | `apps/desktop/src-tauri` | Secure OS-native token storage |
| `tauri-plugin-opener` | `apps/desktop` | Launch system browser for interactive login |
| `cookie` / `tower-sessions` / equivalent | `apps/hub` | Secure session cookie handling and refresh/session middleware |
| `reqwest` | direct dependency where auth code uses HTTP | Already present transitively today, but should be declared directly if auth code uses it |
| `secrecy` | direct dependency where token structs are added | Already available transitively via kube-client, but should be declared directly if refresh/access tokens are wrapped and redacted |

> **Recommendation:** prefer direct dependency declarations for auth code even if a crate is already available transitively. Auth/security code should not rely on incidental transitive APIs.

### 4.2 New JavaScript dependencies

#### Recommended phase 1 choice: **no new browser-side auth library**

Because `apps/web` should remain **hub-mediated**, phase 1 does **not** need `@azure/msal-browser`.

- The hub performs the OIDC dance.
- The browser only needs to know whether the user is signed in and whether it should include cookies.
- This keeps tokens out of browser-managed storage and avoids split token logic between Rust and TypeScript.

#### When `@azure/msal-browser` would make sense

Only add it if Telescope later supports a **pure SPA mode** that talks directly to Azure without the hub. That is a different security model and is not required for the recommended design.

### 4.3 Hub changes

#### Required endpoints

| Endpoint | Purpose |
|---|---|
| `GET /auth/login` | Start Authorization Code + PKCE flow and redirect to Entra |
| `GET /auth/callback` | Validate `state`, exchange code, validate token, establish session |
| `POST /auth/refresh` | Refresh access token or renew Telescope session before expiry |
| `POST /auth/device-code` | Start device code flow for fallback scenarios |
| `GET /auth/me` | Return normalized identity and expiry metadata |
| `POST /auth/logout` | Revoke/clear local session and logout from Telescope |

#### Required auth services

- Entra metadata discovery via `/.well-known/openid-configuration`
- JWKS retrieval and cache invalidation
- ID token validation (`iss`, `aud`, `exp`, `nbf`, `nonce`, signature)
- Access/refresh token refresh orchestration
- Normalized claim extraction: email/UPN, display name, object ID, tenant ID, groups/app roles
- Group overage handling if `_claim_names` / `_claim_sources` appears
- Session middleware and cookie policy
- WebSocket upgrade auth (`/ws` must not stay anonymous)
- Explicit preflight validation that the hub’s cluster identity can use Kubernetes impersonation where required

#### Recommended session model

For v1, prefer **hub-managed secure cookies** or another restart-safe session mechanism over volatile in-memory state.

- **Preferred deployment:** same-origin web + hub deployment with `HttpOnly`, `Secure`, and appropriate `SameSite` policy.
- **If cross-origin is required:** use `SameSite=None; Secure` plus an explicit `CORS_ALLOWED_ORIGINS` allowlist and `credentials: true` behavior.
- Do **not** keep PKCE bootstrap state only in volatile in-memory storage.

#### Hub sketch

```rust
// sketch only
async fn auth_callback(/* ... */) -> Result<impl IntoResponse, AuthError> {
    // 1. Validate state from signed cookie or durable server-side store
    // 2. Exchange code + code_verifier with Entra token endpoint
    // 3. Validate ID token using issuer metadata + JWKS
    // 4. Normalize claims -> AuthUser { email, name, groups, oid, tid }
    // 5. Set session cookie and redirect back to app
}
```

### 4.4 Desktop changes

#### Required Tauri commands

| Command | Purpose |
|---|---|
| `auth_login_interactive` | Start browser-based login |
| `auth_login_device_code` | Start device-code fallback |
| `auth_status` | Return current identity, expiry, and auth source |
| `auth_refresh` | Refresh tokens proactively or reactively |
| `auth_logout` | Clear keyring/session state |

#### Required desktop behavior

- Add `tauri-plugin-opener` to launch the browser.
- Bind a localhost callback listener on a **small fixed port range** registered in Entra.
- Store refresh tokens in the OS keyring.
- Keep the access token in memory where practical; avoid writing plaintext tokens to disk.
- Surface the signed-in Entra identity and expiry countdown in the shared web UI used by the desktop app.
- Preserve existing kubeconfig/kubelogin behavior as a fallback.

```rust
#[tauri::command]
async fn auth_login_interactive() -> Result<AuthStatus, String> {
    // 1. Start loopback listener on 127.0.0.1:18000..18009
    // 2. Generate PKCE + state + nonce
    // 3. Open system browser
    // 4. Receive callback and exchange code
    // 5. Persist refresh token in keyring
    // 6. Return normalized AuthStatus to the UI
    todo!()
}
```

#### Important nuance: kubelogin coexistence

For desktop, native Entra auth should **supplement**, not immediately replace, the current kubeconfig exec flow.

- **Phase 1:** keep `kubelogin` as the path that `kube-rs` already understands.
- **Phase 2+:** decide whether Telescope should mint and inject its own Kubernetes bearer token for AKS or continue delegating Kubernetes auth to kubeconfig while using native Entra auth for ARM calls and identity UX.

That distinction matters because the token audience for Kubernetes access is different from Graph or ARM access.

### 4.5 Web changes

#### Required UI changes

- Add a shared auth state store for identity, expiry, loading, and error state.
- Add login/logout controls.
- Add “session expired / re-auth required” messaging.
- Update error suggestions in `apps/web/src/lib/error-suggestions.ts` for Entra-specific failures (MFA required, tenant mismatch, expired refresh token, loopback blocked, Graph group overage).

#### Required API changes

`apps/web/src/lib/api.ts` should move to a centralized auth-aware wrapper.

- **Hub/web mode (recommended):** use `credentials: 'include'` for cookie-backed sessions.
- **Bearer-token mode (optional / future / remote API cases):** attach `Authorization: Bearer ...`.
- Add a single 401-handling path that attempts refresh once, then redirects to login or shows a re-auth prompt.

```ts
async function authFetch(input: string, init: RequestInit = {}) {
  const headers = new Headers(init.headers ?? {});
  const token = getAccessTokenIfBearerMode();
  if (token) headers.set('Authorization', `Bearer ${token}`);

  const res = await fetch(input, {
    ...init,
    headers,
    credentials: 'include',
  });

  if (res.status === 401) {
    await maybeRefreshSession();
  }

  return res;
}
```

### 4.6 Entra app registration

#### Recommended registration model

| Registration | Platform type | Redirects | Secret | Notes |
|---|---|---|---|---|
| **Telescope Hub** | Web / confidential client | `https://hub.example.com/auth/callback`, `http://localhost:3001/auth/callback` | **Yes** | Used for browser OIDC and hub-managed sessions |
| **Telescope Desktop** | Public/native client | `http://127.0.0.1:18000/auth/callback` ... fixed port range; device code enabled | **No** | Used by Tauri desktop with PKCE and device code |

#### Required delegated permissions

Minimum baseline:

- `openid`
- `profile`
- `email`
- `offline_access`
- `User.Read`

Potentially required depending on final design:

- ARM delegated permission(s) for Azure Resource Manager if Telescope makes ARM calls on behalf of the user
- Additional Graph permission(s) if group overage must be resolved via Graph rather than app roles

#### Tenant model

Open question:

- **Single-tenant:** simpler configuration and support story
- **Multi-tenant / organizations:** friendlier for OSS distribution, but more validation and tenant-boundary work

### 4.7 Security considerations

- Require **PKCE** for all public/native authorization code flows.
- Validate **state** and **nonce** every time.
- Validate JWT **signature**, **issuer**, **audience**, **expiry**, and **not-before**.
- Keep browser tokens out of `localStorage` and `sessionStorage`.
- Use **OS keyring** for desktop refresh tokens.
- Use **HTTP-only secure cookies** or an equivalent server-owned session model for browser mode.
- Tighten hub CORS; `CorsLayer::permissive()` is not compatible with a production cookie-based auth model.
- Authenticate WebSocket upgrades and disconnect stale sessions.
- Enrich audit/tracing with stable identity fields such as tenant ID (`tid`) and object ID (`oid`) without logging raw tokens.

---

## 5. Architecture Diagram (text-based)

### 5.1 Web / hub mode

```text
┌────────────┐
│ Web UI     │  apps/web/src/lib/api.ts
│ (browser)  │  credentials: include
└─────┬──────┘
      │ GET /auth/login
      v
┌────────────┐      discovery/JWKS      ┌──────────────────────────────┐
│ Axum Hub   │ ───────────────────────► │ Entra ID / Microsoft Entra   │
│ auth.rs    │ ◄─────────────────────── │ authorize + token + JWKS     │
│ main.rs    │                          └──────────────────────────────┘
│ routes.rs  │
└─────┬──────┘
      │ validated AuthUser + session cookie
      │
      v
┌────────────┐
│ Hub routes │ ──► create_client_for_context_as_user(...) ──► Kubernetes API
└────────────┘
```

### 5.2 Desktop mode

```text
┌────────────┐
│ Shared UI   │  apps/web packaged by Tauri
└─────┬──────┘
      │ invoke('auth_login_interactive')
      v
┌──────────────────────┐
│ Tauri backend        │ apps/desktop/src-tauri/src/main.rs + new auth module
│ - PKCE generation    │
│ - opener plugin      │
│ - localhost callback │
│ - keyring storage    │
└─────┬────────────────┘
      │ browser redirect / device code fallback
      v
┌──────────────────────────────┐
│ Entra ID / Microsoft Entra   │
└──────────────────────────────┘
      │ validated tokens / identity
      v
┌──────────────────────────────┐
│ Desktop auth state           │
│ - Entra identity             │
│ - expiry countdown           │
│ - ARM-capable token cache    │
└─────┬────────────────────────┘
      │
      ├─► Telescope UI status + re-auth
      └─► engine/kube path (preserve kubelogin fallback initially)
```

### 5.3 ARM vs Kubernetes token boundary

```text
Entra interactive login
        │
        ├─► Kubernetes token path (today: kubeconfig exec/kubelogin)
        │
        └─► ARM/Graph token path (future Telescope-managed tokens)
```

This separation is important. A successful Entra login for ARM operations does **not** automatically mean Telescope should replace the existing Kubernetes exec-plugin path on day one.

---

## 6. Scope and Complexity Assessment

### 6.1 Option sizing

| Scope | Main work | Estimated scope | Complexity |
|---|---|---:|---|
| **Device code only** | Hub/device code endpoints, desktop polling UI, keyring/session work | ~1–2 sprints | Lowest |
| **Interactive browser only** | Hub OIDC redirect, callback handling, cookies, desktop loopback + opener, keyring, UI state | ~2–3 sprints | Medium |
| **Full hybrid** | Everything above plus device-code fallback and more test coverage | ~3–4 sprints | Highest |

### 6.2 Recommended phased approach

#### Phase 0 — shared auth foundation

- Normalize claim mapping (`preferred_username`, `name`, `groups`, `oid`, `tid`).
- Define a shared auth status shape for UI and Rust.
- Decide app registration split and redirect URIs.

#### Phase 1 — desktop interactive auth (recommended first if prioritizing current users)

- Add `tauri-plugin-opener` and localhost callback handling.
- Add `keyring`-backed desktop token storage.
- Add auth status UI and expiry messaging.
- Preserve kubelogin as fallback.

#### Phase 2 — hub/web interactive auth

- Implement `/auth/login`, `/auth/callback`, `/auth/refresh`.
- Add JWKS validation, secure cookie/session handling, WebSocket auth, and tightened CORS.
- Update `api.ts` to use auth-aware fetch and cookie/session mode.

#### Phase 3 — device code fallback and hardening

- Add `/auth/device-code` and desktop fallback command.
- Handle group overage and Graph/app-role strategy.
- Add richer error handling, tracing, and retry/backoff.

#### Phase 4 — ARM integration and convergence

- Add the ARM-specific scope and refresh behavior required by AKS management features.
- Decide whether to keep Kubernetes auth delegated to kubeconfig or move more AKS token handling into Telescope.

> If multi-user hub mode becomes the immediate priority, phases 1 and 2 can be swapped. The recommended flow design still stands either way.

---

## 7. Risks and Open Questions

| Topic | Why it matters | Recommendation / open question |
|---|---|---|
| **Tenant model** | Affects issuer validation, support burden, and OSS onboarding | Decide whether v1 is single-tenant or multi-tenant (`organizations`) |
| **Separate app registrations** | Hub and desktop have different trust boundaries | Recommended: yes |
| **Group overage** | Large Entra tenants may not emit full `groups` in tokens | Decide between Graph lookup, app roles, or documented limitation |
| **Kubernetes vs ARM token scope** | Replacing kubelogin is not the same as acquiring ARM tokens | Keep kubelogin initially; treat ARM auth as a parallel concern |
| **Cookie origin policy** | Same-origin vs split-origin affects `SameSite` and CORS | Prefer same-origin deployment for v1; otherwise require `SameSite=None; Secure` |
| **Hub session durability** | Restart-safe auth bootstrap and cookie/session choices matter operationally | Do not depend on volatile-only state for callback correlation |
| **Linux keyring availability** | Some Linux environments do not provide Secret Service | Decide whether to require a keyring or implement an encrypted fallback store |
| **WebSocket auth** | `/ws` is currently anonymous | Add authenticated upgrade, idle/session expiry handling, and quota limits |
| **Impersonation RBAC** | Verified user identity is not enough if the hub cannot impersonate | Add explicit startup/runtime validation and clear operator guidance |
| **Conditional Access / MFA** | Entra policy may require browser UX, claims challenge handling, or re-auth | Include CA/MFA-specific error surfaces in the UX and troubleshooting docs |

---

## Closing Recommendation

Telescope should implement Entra ID auth as a **hybrid model** built around **Authorization Code + PKCE**, with **Device Code** as fallback.

The most important architecture decisions are:

1. **Hub/web stays hub-mediated** with server-owned sessions.
2. **Desktop uses native Rust auth commands** with browser launch and keyring storage.
3. **Hub and desktop use separate Entra app registrations.**
4. **Kubelogin remains supported** while native Entra auth is introduced.
5. **ARM token management is a first-class design concern**, because the current kubeconfig exec path does not solve future ARM-backed features.

That approach is technically feasible with the current repo shape and aligns with the code already present in:

- `apps/hub/src/auth.rs`
- `apps/hub/src/main.rs`
- `apps/hub/src/routes.rs`
- `crates/engine/src/client.rs`
- `crates/engine/src/kubeconfig.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/error-suggestions.ts`
- `apps/web/src/lib/azure-utils.ts`
- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src-tauri/capabilities/default.json`

It is a substantial cross-runtime authentication project, but it is a practical one.

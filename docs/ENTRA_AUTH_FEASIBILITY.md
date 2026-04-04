---
title: Entra ID Auth Feasibility
nav_exclude: true
description: "Desktop Entra ID authentication investigation — internal reference"
---

# Entra ID Authentication Feasibility

> **Status: Aspirational / Not Shipped** -- This is an internal feasibility study. Native Entra ID authentication is not yet implemented in Telescope. The current product relies on kubeconfig and kubelogin for AKS auth.
> Scope: the shared Svelte frontend in `apps/web` is packaged into the Tauri shell, and frontend/backend calls go through Tauri IPC `invoke()`.

## 1. Executive Summary

Telescope is now a desktop-only Tauri application. The current product can detect AKS and `kubelogin`-style kubeconfig auth, but it cannot start a native Entra ID sign-in, store Entra tokens securely, or refresh desktop-managed credentials when an AKS exec token has expired.

A desktop-native Entra auth subsystem is feasible and should live inside `apps/desktop/src-tauri`. The best long-term design is a **hybrid** approach:

1. **Primary:** Authorization Code + PKCE using the system browser and a localhost loopback callback.
2. **Fallback:** Device Code Flow for locked-down or headless environments.
3. **Storage:** OS credential storage via the `keyring` crate, not plain SQLite.
4. **Desktop contract:** the UI calls new Tauri commands such as `entra_login`, `entra_logout`, `entra_refresh`, and `entra_status`.

This work should **coexist with the current kubeconfig + kubelogin path at first**. The initial goal is not to rip out kubeconfig auth; it is to give Telescope a native Entra-aware sign-in and refresh experience inside the desktop app.

## 2. Current State / Gap Analysis

### 2.1 What exists today

| Area | Current implementation | Evidence | Gap |
|---|---|---|---|
| Frontend transport | `apps/web/src/lib/api.ts` is a Tauri-only wrapper around dynamic `@tauri-apps/api/core` `invoke()` calls | `apps/web/src/lib/api.ts` | No auth-specific IPC contract, no identity status store, no sign-in/out actions |
| AKS auth awareness | The engine reads kubeconfig auth metadata, detects `exec` / `token` / `certificate`, recognizes AKS endpoints, and surfaces a kubelogin hint | `crates/engine/src/client.rs` | Detection only; no native Entra token acquisition, validation, refresh, or persistence |
| Desktop command surface | `apps/desktop/src-tauri/src/main.rs` registers cluster, resource, Azure, and preference commands | `apps/desktop/src-tauri/src/main.rs` | No `entra_login`, `entra_logout`, `entra_refresh`, or `entra_status` commands |
| Desktop permissions | The default capability set only grants core window and event permissions | `apps/desktop/src-tauri/capabilities/default.json` | No browser-launch permission for opening the Entra authorize URL |
| Desktop dependencies | The Tauri crate depends on core runtime, Kubernetes, Azure, and serialization crates | `apps/desktop/src-tauri/Cargo.toml` | No `oauth2`, `openidconnect`, `jsonwebtoken`, or `keyring` dependencies |
| Secure token storage | No desktop token persistence mechanism is present | `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/src/main.rs` | No OS keyring integration and no refresh-token custody model |
| Auth UI state | The shared frontend has no Entra account store, sign-in UI, sign-out action, or token-expiry indicator | `apps/web/src/lib/api.ts`, `apps/web/src/routes`, `apps/web/src/lib/components` | The desktop UI cannot show identity or trigger re-auth flows |

### 2.2 Why expired AKS auth is still a desktop problem

Today the desktop app depends on kubeconfig-authenticated clients. When the selected AKS context uses `kubelogin`, the engine can tell the UI that the context is AKS-backed and exec-authenticated, but Telescope still has no native way to do any of the following inside the app:

- start an Entra sign-in flow,
- remember an authenticated desktop identity,
- refresh a desktop-managed token before it expires,
- show whether the user is signed in,
- recover gracefully when Entra-backed operations need user interaction.

That gap matters even if kubeconfig remains the source of truth for Kubernetes API access in early phases. Native desktop auth is still valuable for user guidance, future Azure management actions, and proactive re-auth instead of leaving the user to recover outside Telescope.

### 2.3 Concrete desktop-only conclusions from the current code

- `apps/web/src/lib/api.ts` already gives Telescope the right integration seam: add auth commands to the Tauri backend and call them through `invoke()`.
- `crates/engine/src/client.rs` already gives Telescope the right detection seam: when AKS + `kubelogin` are detected, the UI can surface Entra-specific guidance and prompt for native sign-in.
- `apps/desktop/src-tauri/src/main.rs` is missing the auth command surface entirely.
- `apps/desktop/src-tauri/capabilities/default.json` is too minimal for interactive sign-in because the app cannot yet open the system browser.
- `apps/desktop/src-tauri/Cargo.toml` confirms the auth stack has not been added yet.

## 3. Authentication Options

### 3.1 Option Summary

| Option | UX | Complexity | Strengths | Weaknesses | Fit |
|---|---|---:|---|---|---|
| **A. Device Code Flow** | Functional, explicit, works without callback plumbing | Low | Reliable in restricted environments | Slower and less polished | Good fallback |
| **B. Interactive Browser Login (Auth Code + PKCE)** | Best interactive experience | Medium | Familiar sign-in, better for day-to-day use | Requires loopback callback handling | Best default |
| **C. Hybrid** | Best overall coverage | Medium-high | Lets Telescope prefer PKCE and fall back to device code | More moving parts | **Recommended** |

### 3.2 Option A: Device Code Flow

**Desktop flow:** the Tauri backend requests a device code from Entra, returns the `user_code` and verification URL to the UI, the user completes sign-in in a separate browser session, and the backend polls until tokens are issued or the request expires.

#### Why it fits Telescope

- Works well when localhost callbacks are blocked.
- Maps cleanly onto a Tauri desktop command model.
- Avoids loopback server setup in the first implementation.
- Matches environments where users already expect CLI-style Azure sign-in recovery.

#### Implementation outline

1. Add `oauth2` to the desktop crate.
2. Implement a Rust auth module that starts the device-code grant.
3. Expose a Tauri command that returns:
   - `verification_uri`
   - `user_code`
   - `expires_in`
   - `interval`
4. Poll from Rust, not from the UI, so retry timing and cancellation remain centralized.
5. Persist the resulting refresh token in the OS keyring.
6. Return normalized identity and expiry metadata to the Svelte store.

#### Pros

- Simplest path to a working desktop sign-in.
- Strong fallback for headless, remote, or policy-constrained environments.
- No deep-link or loopback listener required.

#### Cons

- More friction for everyday sign-in.
- The user must switch to the browser and manually enter a code.
- Polling and timeout states need good UX copy.

### 3.3 Option B: Interactive Browser Login (Authorization Code + PKCE)

**Desktop flow:** a "Sign in with Entra" action in the UI calls `entra_login`; Rust generates `state`, `nonce`, `code_verifier`, and `code_challenge`; Rust launches the system browser with `tauri-plugin-opener`; Entra redirects to `http://localhost:{port}/callback`; the Rust backend exchanges the code for tokens, validates the ID token, stores secrets in the OS keyring, and updates desktop auth state.

#### Why it fits Telescope

- Best sign-in UX for a desktop app.
- Supports a visible account model and clear re-auth prompts.
- Gives Telescope a clean foundation for future proactive refresh.
- Keeps all token handling in Rust instead of the UI runtime.

#### Implementation outline

1. Add `openidconnect` for discovery, PKCE, nonce handling, and ID token validation.
2. Add `oauth2` for token exchange and refresh support.
3. Add `tauri-plugin-opener` so the app can launch the system browser.
4. Start a localhost loopback listener in Rust on an allowed callback port.
5. Validate `state`, `nonce`, issuer, audience, and expiry before storing anything.
6. Save refresh tokens to the OS keyring and keep short-lived access tokens in memory when practical.

#### Pros

- Best daily-user experience.
- Natural fit for a desktop sign-in button and account indicator.
- Easier to explain to users than device-code-only auth.

#### Cons

- Requires loopback callback plumbing and local port management.
- Needs careful handling of callback races, cancellation, and duplicate requests.
- Some endpoint security tooling may restrict localhost listeners.

### 3.4 Option C: Hybrid (Recommended)

Use **PKCE as the primary flow** and **device code as the fallback**.

This gives Telescope the best balance of usability and reliability:

- everyday use gets the smoothest sign-in path,
- restricted environments still have a viable fallback,
- the desktop app can ship incrementally without blocking on every edge case up front.

## 4. Implementation Requirements

### 4.1 New Rust dependencies

| Dependency | Purpose |
|---|---|
| `oauth2` | Device code grant, refresh-token exchange, common OAuth primitives |
| `openidconnect` | Discovery, PKCE flow, nonce handling, ID token validation |
| `jsonwebtoken` | Supplemental JWT parsing/validation where Telescope needs explicit claim handling outside the `openidconnect` helpers |
| `keyring` | OS-native storage for refresh tokens and related secret material |

### 4.2 New Tauri plugins and capability updates

| Item | Requirement |
|---|---|
| `tauri-plugin-opener` | Launch the system browser for interactive sign-in |
| `tauri-plugin-deep-link` | Optional future enhancement if Telescope later prefers app-link callbacks over loopback |
| `apps/desktop/src-tauri/capabilities/default.json` | Add the opener permissions needed to launch the Entra authorization URL |

### 4.3 New desktop auth commands

| Command | Purpose |
|---|---|
| `entra_login` | Start sign-in; choose PKCE by default and device code when needed |
| `entra_logout` | Clear in-memory auth state and remove stored credentials from the OS keyring |
| `entra_refresh` | Refresh tokens before expiry or in response to an auth challenge |
| `entra_status` | Return signed-in state, account metadata, auth method, and expiry information |

A practical first return shape for `entra_status` is:

```rust
struct EntraStatus {
    signed_in: bool,
    method: Option<String>,
    account_label: Option<String>,
    expires_at: Option<String>,
    tenant_id: Option<String>,
}
```

### 4.4 Desktop auth module responsibilities

The Rust backend needs a dedicated auth module, for example under `apps/desktop/src-tauri/src/auth/`, responsible for:

- Entra metadata discovery,
- PKCE generation and callback correlation,
- device-code grant startup and polling,
- ID token validation,
- refresh-token exchange,
- keyring read/write/delete operations,
- normalized auth state returned to the UI,
- translating auth failures into stable desktop command errors.

### 4.5 Frontend integration requirements

The shared Svelte frontend should add:

- an auth state store for current identity, auth method, loading state, and expiry,
- a login action that calls `invoke('entra_login')`,
- a logout action that calls `invoke('entra_logout')`,
- a token status indicator in the desktop UI,
- re-auth prompts when AKS/kubelogin detection suggests the current context needs Entra interaction.

The frontend should stay thin: it renders state and invokes desktop commands, while token acquisition and storage remain in Rust.

### 4.6 Entra app registration requirements

Telescope needs a **public/native client** registration configured for desktop use.

Required baseline:

- Redirect URI pattern: `http://localhost:{port}/callback`
- PKCE enabled for authorization code flow
- Device code flow enabled
- Delegated permissions for at least:
  - `openid`
  - `profile`
  - `email`
  - `offline_access`
  - any Azure management scope Telescope needs for user-driven operations

Open registration decisions:

- fixed callback port vs approved small port range,
- single-tenant vs multi-tenant distribution,
- which Azure scopes are needed in the first production milestone.

### 4.7 Token storage model

**Requirement:** store long-lived secrets in the OS credential store through `keyring`.

Recommended model:

- refresh token: OS keyring,
- ID token: optional, only if needed for cached profile display,
- access token: memory-first, refreshed when needed,
- plain SQLite storage for Entra secrets: **not allowed**.

### 4.8 Kubeconfig coexistence model

In the first rollout, Telescope should treat native Entra auth as a desktop capability that complements the existing kubeconfig path.

- `crates/engine/src/client.rs` continues building Kubernetes clients from kubeconfig.
- AKS and `kubelogin` detection remains useful for UX and remediation.
- Native Entra auth can power desktop identity, future Azure management operations, and token-expiry recovery UX.
- A later phase can decide whether Telescope should bridge native auth into Kubernetes credential renewal more directly.

## 5. Desktop-Only Architecture Diagram

```mermaid
flowchart LR
    UI[Svelte UI in apps/web\npackaged by Tauri] -->|invoke('entra_login')| IPC[Tauri IPC]
    IPC --> AUTH[Rust auth module\napps/desktop/src-tauri/src/auth]
    AUTH -->|open system browser\nor start device-code flow| ENTRA[Microsoft Entra ID]
    ENTRA -->|tokens + claims| AUTH
    AUTH --> KEYRING[OS Keyring]
    AUTH --> STATE[Desktop auth state]
    STATE --> UI
    AUTH -. AKS/kubelogin guidance .-> ENGINE[crates/engine/src/client.rs]
```

### Key architectural boundaries

- The UI never owns refresh tokens.
- The Rust backend owns OAuth/OpenID Connect logic.
- OS keyring storage is the long-term secret store.
- The existing kubeconfig path remains separate until Telescope intentionally changes it.

## 6. Phased Rollout

### Phase 1: Device Code

Ship the smallest complete desktop auth slice first.

Deliverables:

- `oauth2` integration for device code,
- `keyring` integration,
- `entra_login` using device code,
- `entra_status` and `entra_logout`,
- basic sign-in UI and token status indicator.

Exit criteria:

- a user can complete Entra sign-in from Telescope without leaving the desktop auth flow unmanaged,
- refresh tokens are stored in the OS keyring,
- the UI can show signed-in state and expiry.

### Phase 2: PKCE

Add the preferred interactive experience.

Deliverables:

- `openidconnect` + PKCE flow,
- `tauri-plugin-opener`,
- localhost callback listener,
- `entra_login` defaulting to PKCE with device code as fallback,
- better UI messaging for callback errors and cancellation.

Exit criteria:

- clicking "Sign in with Entra" launches the system browser,
- the desktop app completes the loopback callback and stores credentials securely,
- the fallback path still works when interactive login cannot complete.

### Phase 3: Silent Refresh

Make the desktop auth model durable for repeated use.

Deliverables:

- proactive refresh before expiry,
- `entra_refresh`,
- startup restoration from the OS keyring,
- expiry countdown and re-auth prompts,
- better handling for revoked refresh tokens or policy-driven reauthentication.

Exit criteria:

- users do not need to sign in on every app restart,
- desktop auth state survives normal restarts,
- expired or revoked sessions degrade gracefully.

## 7. Risks and Open Questions

| Topic | Why it matters | Notes |
|---|---|---|
| Loopback callback ports | Interactive login depends on a localhost callback that must be permitted and registered | Decide whether to use one fixed port or a small approved range |
| Linux keyring availability | Some desktop environments do not expose a working credential store by default | Decide whether lack of keyring support is blocking or needs an encrypted fallback |
| Tenant model | Affects issuer validation, support scope, and app registration defaults | Decide whether the first release is single-tenant or multi-tenant |
| Scope selection | Azure management features may need more than baseline OpenID scopes | Keep first release narrow and add scopes only when a desktop feature requires them |
| Kubeconfig integration depth | Native Entra auth and kubeconfig auth are related but not identical | Decide later whether native auth should stay advisory or participate in credential renewal |
| Refresh token revocation | Users may hit tenant policies, MFA step-up, or revoked grants | The UI needs clear re-auth messaging and safe logout behavior |
| Deep-link support | Loopback is a good default, but app links may become attractive later | Keep `tauri-plugin-deep-link` optional until loopback is proven insufficient |

## 8. Recommendation

Proceed with **Option C: Hybrid**.

- Build the desktop auth module in Rust.
- Store secrets in the OS keyring.
- Ship device code first if the team wants the fastest path to a complete flow.
- Make PKCE the default interactive experience once loopback callback handling is ready.
- Keep the current kubeconfig + `kubelogin` path in place until the desktop auth subsystem is proven and the Kubernetes credential handoff story is explicitly designed.

This plan is fully compatible with the current desktop-only repo shape and with the code that exists today in:

- `apps/web/src/lib/api.ts`
- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/capabilities/default.json`
- `crates/engine/src/client.rs`


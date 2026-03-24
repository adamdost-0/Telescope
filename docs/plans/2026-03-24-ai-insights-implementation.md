# AI Insights Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a dedicated desktop AI Insights workflow that generates advisory-only Kubernetes and AKS summaries through Azure OpenAI, supports either Azure login context or API key authentication, stores only the last 3 encrypted insight runs per cluster, and renders strictly schema-shaped output in a separate `/insights` route.

**Architecture:** Keep the split narrow and explicit. `crates/azure` owns Azure OpenAI endpoint normalization, `DefaultAzureCredential()` and API key auth, cloud-profile handling, and provider error classification. `crates/engine` owns the allowlist-only context builder, prompt/schema contract, and orchestration request/response models. `crates/core` owns encrypted history persistence in SQLite, but not OS-specific secret retrieval. `apps/desktop/src-tauri` owns OS credential-store integration plus the thin Tauri command surface. `apps/web` owns the dedicated route, Settings UI, deterministic rendering, and disconnected history access.

**Tech Stack:** Rust workspace crates (`telescope-core`, `telescope-engine`, `telescope-azure`), Tauri v2 IPC, Svelte 5 runes in `apps/web`, SQLite, OS credential store for secrets/key wrapping, Azure OpenAI chat completions over user-supplied endpoints.

---

I'm using the writing-plans skill to create the implementation plan.

## Locked Scope And Non-Negotiables

- Separate top-level Insights route: `/insights`.
- Azure OpenAI is the only provider in v1.
- Auth mode is user-selectable and explicit: Azure login context via `DefaultAzureCredential()` or API key.
- Cloud profile selection is visible in Settings and must not block future sovereign endpoints.
- Insight history is encrypted locally, capped at the last 3 entries per cluster, and only supports clear-all for the current cluster.
- Context builder is allowlist-only. Do not send raw resource bodies, secret payloads, kubeconfigs, tokens, or connection strings.
- Context shaping uses fixed deterministic caps per category and stable ordering.
- Dev diagnostics live on Settings only and only in dev mode. No prompt preview in v1.
- No chat UI, no auto-remediation, no browser fallback, no silent auth-mode fallback.

## Expected File Surface

**Likely create:**
- `crates/azure/src/openai.rs`
- `crates/engine/src/insights.rs`
- `crates/engine/src/insights_context.rs`
- `crates/core/src/insights_history.rs`
- `apps/desktop/src-tauri/src/ai_insights.rs`
- `apps/desktop/src-tauri/src/secure_storage.rs`
- `apps/web/src/routes/insights/+page.svelte`
- `apps/web/src/lib/insights.ts`
- `apps/web/src/lib/insights.test.ts`
- `apps/web/tests-e2e/insights.spec.ts`

**Likely modify:**
- `crates/azure/Cargo.toml`
- `crates/azure/src/error.rs`
- `crates/azure/src/lib.rs`
- `crates/azure/src/types.rs`
- `crates/engine/src/lib.rs`
- `crates/core/Cargo.toml`
- `crates/core/src/lib.rs`
- `crates/core/src/store.rs`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/src/main.rs`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/tauri-commands.ts`
- `apps/web/src/routes/settings/+page.svelte`
- `apps/web/src/lib/components/Sidebar.svelte`
- `apps/web/src/lib/components/SearchPalette.svelte`
- `apps/web/tests-e2e/settings.spec.ts`
- `apps/web/tests-e2e/helpers/mock-tauri.ts`
- `docs/TESTING.md`
- `docs/UX_NOTES.md`

## Implementation Tasks

### Task 1: Lock the shared AI Insights contract and settings keys

**Files:**
- Create: `crates/engine/src/insights.rs`
- Modify: `crates/engine/src/lib.rs`
- Modify: `apps/web/src/lib/tauri-commands.ts`
- Modify: `apps/web/src/lib/api.ts`

**Steps:**
1. Define the engine-owned request and response types for AI Insights so the JSON contract mirrors the PRD exactly: summary, risks, observations, recommendations, references.
2. Add explicit settings models for auth mode, cloud profile, endpoint, deployment/model name, and dev diagnostics metadata.
3. Centralize key names for persisted non-secret settings so Rust and TypeScript do not drift on string literals.
4. Keep the model contract schema-first and advisory-only; do not add freeform chat or action execution fields.

**Validation:**
- `cargo test -p telescope-engine insights`
- `pnpm -C apps/web build`

### Task 2: Add Azure OpenAI transport, endpoint normalization, and auth-mode handling

**Files:**
- Create: `crates/azure/src/openai.rs`
- Modify: `crates/azure/Cargo.toml`
- Modify: `crates/azure/src/error.rs`
- Modify: `crates/azure/src/lib.rs`
- Modify: `crates/azure/src/types.rs`

**Steps:**
1. Implement a small Azure OpenAI client in `crates/azure/src/openai.rs` using the existing `reqwest` stack and `AzureCloud` model instead of introducing a second provider abstraction.
2. Support exactly two auth paths: bearer token from `DefaultAzureCredential()` and API key from desktop-managed secret storage.
3. Normalize the user-supplied endpoint plus selected cloud profile into a validated chat-completions URL shape.
4. Add a dedicated test-connection method that performs a real chat-completions-capable request path without generating a full insight run.
5. Extend Azure error classification for AI-specific failures: invalid endpoint shape, credential acquisition failure, RBAC denied chat-completions access, timeout/network failure, and configuration errors.
6. Keep API key as an explicit mode only. Do not auto-retry from Azure login to API key.

**Validation:**
- `cargo test -p telescope-azure openai`
- `cargo test -p telescope-azure error`

### Task 3: Build the allowlist-only context builder with fixed category caps

**Files:**
- Create: `crates/engine/src/insights_context.rs`
- Modify: `crates/engine/src/insights.rs`
- Modify: `crates/engine/src/lib.rs`

**Steps:**
1. Implement a single context-builder entry point that reads only already-available app state: watcher cache, connection state, recent failures/events, Helm release state, node conditions, and AKS ARM details when available.
2. For each category, serialize curated summaries only. Do not pass raw Kubernetes objects, raw Helm values, or any Secret content.
3. Enforce deterministic per-category caps before serialization, with stable ordering so repeated runs stay comparable.
4. Add explicit redaction guards for token-like values, kubeconfig-looking text, connection strings, and service-account credentials even if they appear in otherwise safe fields.
5. Make namespace scope visible in the request contract so the model cannot make cluster-wide claims from namespace-limited data.

**Validation:**
- `cargo test -p telescope-engine insights_context`

### Task 4: Add the prompt contract and structured orchestration in engine

**Files:**
- Modify: `crates/engine/src/insights.rs`
- Modify: `crates/engine/src/lib.rs`

**Steps:**
1. Encode the PRD prompt contract as versioned engine-owned prompt text plus a strict response schema validator.
2. Keep prompt generation deterministic: stable section ordering, no user-specific hidden persona, no speculative instructions.
3. Add a thin orchestration function that receives a safe context payload plus a provider callback and returns validated structured output or a classified failure.
4. Surface prompt version and redaction-policy version for dev diagnostics without exposing prompt bodies.

**Validation:**
- `cargo test -p telescope-engine insights`

### Task 5: Add encrypted history persistence in core with per-cluster retention

**Files:**
- Create: `crates/core/src/insights_history.rs`
- Modify: `crates/core/Cargo.toml`
- Modify: `crates/core/src/lib.rs`
- Modify: `crates/core/src/store.rs`

**Steps:**
1. Add a new SQLite table for AI insight history keyed by cluster identity plus creation timestamp.
2. Store only ciphertext, nonce/IV, and the minimal non-sensitive metadata needed to list and trim history.
3. Implement `insert`, `list_recent`, and `clear_all_for_cluster` operations with retention trimming to the last 3 entries per cluster.
4. Keep encryption generic in core by accepting an already-resolved data-encryption key from the desktop layer rather than embedding OS-specific secret store logic in core.
5. Do not add per-entry deletion, export, or raw prompt persistence.

**Validation:**
- `cargo test -p telescope-core insights_history`
- `cargo test -p telescope-core store`

### Task 6: Add desktop secret management and a thin AI Insights Tauri surface

**Files:**
- Create: `apps/desktop/src-tauri/src/secure_storage.rs`
- Create: `apps/desktop/src-tauri/src/ai_insights.rs`
- Modify: `apps/desktop/src-tauri/Cargo.toml`
- Modify: `apps/desktop/src-tauri/src/main.rs`

**Steps:**
1. Add OS credential-store integration for two secret classes only: the wrapped data-encryption key for history and the optional Azure OpenAI API key.
2. Keep non-secret AI settings in normal preferences, but keep the API key out of `user_preferences` entirely.
3. Move AI Insights command logic into `ai_insights.rs` so `main.rs` stays registration-heavy rather than feature-heavy.
4. Expose typed Tauri commands for: load settings, save settings, test connection, generate insights, list recent history, and clear history for the current cluster.
5. Keep generation flow simple: gather current cluster identity, build safe context in engine, call Azure provider, validate response, encrypt and persist result, return the structured response.
6. Add dev-only diagnostics payload generation on Settings; never expose prompt/body contents in general UI state.

**Validation:**
- `cargo test -p telescope-core`
- `cargo test -p telescope-engine`
- `cargo test -p telescope-azure`

### Task 7: Extend Settings for AI Insights configuration and test-connect

**Files:**
- Modify: `apps/web/src/routes/settings/+page.svelte`
- Modify: `apps/web/src/lib/api.ts`
- Modify: `apps/web/src/lib/tauri-commands.ts`

**Steps:**
1. Add an AI Insights section to Settings instead of inventing a second configuration page.
2. Reuse the Azure cloud selector pattern already present on Settings; do not add a duplicate cloud control on `/insights`.
3. Add fields for endpoint, deployment/model name, auth mode, and API key entry, plus a real Test connection button.
4. Render RBAC-denied Azure login failures with explicit guidance to switch to API key mode if the endpoint is reachable but unauthorized.
5. Render dev diagnostics only when the frontend is in dev mode. Keep diagnostics on Settings only.
6. Do not add a prompt preview, token counter, or general debugging console.

**Validation:**
- `pnpm -C apps/web build`
- `pnpm -C apps/web test -- --run src/lib/insights.test.ts`

### Task 8: Build the dedicated `/insights` route with offline history review

**Files:**
- Create: `apps/web/src/routes/insights/+page.svelte`
- Create: `apps/web/src/lib/insights.ts`
- Create: `apps/web/src/lib/insights.test.ts`
- Modify: `apps/web/src/lib/api.ts`
- Modify: `apps/web/src/lib/tauri-commands.ts`

**Steps:**
1. Add a route that can load the current cluster’s last 3 stored insights on mount, even when the app is disconnected.
2. Render generated output in fixed section order only: summary, risks, observations, recommendations, references.
3. Validate the returned payload shape before rendering and fail closed with a user-visible error if validation fails.
4. Make “Generate insights” unavailable when required config or live cluster context is missing, but keep history browsing available.
5. Add a single clear-all action scoped to the current cluster. Do not add per-entry delete.
6. Keep the page advisory-only: link references back into Telescope routes where possible, but never run actions from AI output.

**Validation:**
- `pnpm -C apps/web test -- --run src/lib/insights.test.ts`
- `pnpm -C apps/web build`

### Task 9: Wire navigation, disconnected access, and search discovery

**Files:**
- Modify: `apps/web/src/lib/components/Sidebar.svelte`
- Modify: `apps/web/src/lib/components/SearchPalette.svelte`

**Steps:**
1. Add Insights to the top-level navigation and search palette as a first-class route, not an Overview sub-tab.
2. Adjust disconnected-route gating so `/insights` remains reachable for offline history review while generation controls remain disabled.
3. Keep the nav logic simple: do not create a second connectivity model just for AI Insights.

**Validation:**
- `pnpm -C apps/web build`
- `pnpm -C apps/web e2e --grep insights`

### Task 10: Add deterministic Rust tests for auth, caps, redaction, and ciphertext-at-rest

**Files:**
- Modify: `crates/azure/src/openai.rs`
- Modify: `crates/azure/src/error.rs`
- Modify: `crates/engine/src/insights.rs`
- Modify: `crates/engine/src/insights_context.rs`
- Modify: `crates/core/src/insights_history.rs`
- Modify: `crates/core/src/store.rs`

**Steps:**
1. Use fake credentials and fake provider transports for Azure login and RBAC-denied cases. Do not depend on live Azure OpenAI in CI.
2. Add context-builder tests that fail on unexpected fields, secret leakage, unstable ordering, and category-cap overruns.
3. Add persistence tests that verify ciphertext is stored at rest and that retention trims to 3 entries per cluster.
4. Add schema-validation tests for malformed model output and diagnostics metadata generation.

**Validation:**
- `cargo test -p telescope-azure`
- `cargo test -p telescope-engine`
- `cargo test -p telescope-core`

### Task 11: Add frontend unit coverage for rendering and settings state

**Files:**
- Modify: `apps/web/src/lib/insights.test.ts`
- Modify: `apps/web/src/routes/settings/+page.svelte`
- Modify: `apps/web/src/lib/api.ts`

**Steps:**
1. Unit-test schema validation and fixed-section rendering helpers in `apps/web/src/lib/insights.test.ts`.
2. Add coverage for auth-mode switching, Settings save/test-connect state transitions, and RBAC error messaging.
3. Keep frontend tests deterministic by mocking Tauri IPC only; do not call live Azure services.

**Validation:**
- `pnpm -C apps/web test`

### Task 12: Add Playwright coverage for the route, Settings, and offline history behavior

**Files:**
- Create: `apps/web/tests-e2e/insights.spec.ts`
- Modify: `apps/web/tests-e2e/settings.spec.ts`
- Modify: `apps/web/tests-e2e/helpers/mock-tauri.ts`

**Steps:**
1. Extend the mock Tauri layer with AI Insights settings, test-connect responses, generate responses, diagnostics payloads, and encrypted-history list/clear behavior.
2. Add happy-path coverage for saving config, testing connection, generating insights, rendering the structured output, and clearing history.
3. Add failure coverage for missing config, Azure RBAC denial, schema-validation failure, and disconnected history-only access.
4. Verify that `/insights` stays reachable while disconnected and that generation is disabled in that state.

**Validation:**
- `pnpm -C apps/web e2e --grep insights`
- `pnpm -C apps/web e2e --grep settings`

### Task 13: Finish with grounded docs and repo-wide validation

**Files:**
- Modify: `docs/TESTING.md`
- Modify: `docs/UX_NOTES.md`
- Modify: `docs/PRD.md`

**Steps:**
1. Update `docs/TESTING.md` with the new Rust, Settings, and Playwright AI Insights coverage.
2. Update `docs/UX_NOTES.md` with the new `/insights` route and the dev-only diagnostics placement on Settings.
3. Move the PRD addendum language from planned to shipped only after the implementation is complete.
4. Run full repo validation with existing repo commands rather than inventing new ones.

**Validation:**
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings`
- `cargo test --workspace --exclude telescope-desktop --all-features`
- `pnpm -r --if-present test`
- `pnpm -C apps/web build`

## Major Phases

1. Backend contract and provider seam: engine schema/context plus Azure OpenAI auth and error handling.
2. Persistence and desktop orchestration: encrypted history, OS-backed secret management, and thin Tauri commands.
3. Frontend delivery: Settings configuration, dedicated `/insights` route, offline history access, and navigation wiring.
4. Test and docs hardening: deterministic Rust coverage, mocked Playwright flows, and grounded docs updates.

## Top Risks

- Cross-platform OS credential-store behavior can slow delivery if the chosen crate does not behave consistently under Tauri packaging.
- Azure OpenAI RBAC failures are easy to misclassify; error guidance must distinguish endpoint, credential, and authorization failures cleanly.
- Context caps that are too tight will produce generic insights; caps that are too loose risk token bloat and unstable comparisons.
- Disconnected history access adds a small routing exception; keep that exception local to `/insights` so it does not erode the existing connected-route model.
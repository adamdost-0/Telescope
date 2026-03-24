# Squad Decisions

## Active Decisions

### 2026-03-19: K8s Capability Audit — Ship v1.0.0

**Authors:** Dallas (lead), Ripley, Lambert, Kane  
**Status:** Accepted  
**Type:** Architecture audit

**Context:** Full-stack audit of Telescope v1.0.0 K8s capabilities across engine, IPC, frontend, and test layers. Validated against live AKS cluster `dassadsawqew`.

**Findings:**
- 29 watched GVKs, 66 Tauri IPC commands, 65 frontend API functions, 39 routes — near-complete coverage
- All kubectl/helm live-cluster commands succeeded; zero failures
- All tests green: Rust 176/176, Web 36/36, E2E 16/16
- Frontend builds clean

**Gaps:**
| Gap | Severity | Notes |
|---|---|---|
| Helm install/upgrade/uninstall | Medium | Read-only + rollback today; no chart install or upgrade |
| Helm template/dry-run | Low | Pairs with install/upgrade when those ship |
| ReplicaSets list route | Low | Watched, accessible via generic detail |
| ClusterRoles list route | Low | Watched, accessible via generic detail |
| ClusterRoleBindings list route | Low | Watched, accessible via generic detail |

**Acceptable non-gaps:** VPA (CRD add-on, covered by CRD browser), legacy v1/Endpoints (superseded by EndpointSlices), Helm chart repos (CLI scope).

**Decision:** Ship v1.0.0 as-is. Only post-release priority is Helm write operations.

---

### 2026-03-19: SOTA Models Only

**Author:** Adam Dost (directive)  
**Status:** Active  
**Type:** Team policy

Only use latest SOTA models for all agent spawns: **Opus 4.6** and **GPT-5.3-Codex**. Haiku is forbidden.

---

### 2026-03-19: Post-Audit Work Priorities

**Author:** Dallas (Lead)  
**Status:** Accepted  
**Type:** Prioritization

**Context:** Prioritized backlog from the K8s capability audit. v1.0.0 shipped clean — these are post-release improvements.

**Items:**
| ID | Item | Priority | Owner | Depends on |
|---|---|---|---|---|
| P1-1 | Helm install | P1 | Ripley + Lambert | — |
| P1-2 | Helm upgrade | P1 | Ripley + Lambert | P1-1 |
| P1-3 | Helm uninstall | P1 [ok] | Ripley | — |
| P1-4 | Helm template/dry-run | P1 (lower) | Ripley + Lambert | P1-1, P1-2 |
| P2-1 | ReplicaSets list route | P2 [ok] | Lambert | — |
| P2-2 | ClusterRoles list route | P2 [ok] | Lambert | — |
| P2-3 | ClusterRoleBindings list route | P2 [ok] | Lambert | — |

**Decision:** Helm writes are P1, not P0. Recommended sequence: uninstall first (quick win), then install+upgrade together, then template/dry-run. Missing list routes are P2 — batch when convenient.

---

### 2026-03-19: ARM Node Pool Error Handling

**Authors:** Ripley (backend), Lambert (frontend), Kane (tests)  
**Status:** Accepted  
**Type:** Bug fix + UX improvement

**Context:** ARM node pool failures were surfaced as generic errors or silently swallowed. `listAksNodePools` returned `[]` on error, hiding failures. `delete_node_pool` polling treated any GET error as successful deletion.

**Changes:**
- Backend: Typed ARM error variants (TokenExpired, SubscriptionNotFound, ResourceGroupNotFound, ClusterNotFound, PermissionDenied, Timeout) with actionable messages
- Frontend: Dismissible error banner on node-pools page with guidance mapping; `listAksNodePools` now rethrows after notification
- Bug fix: `delete_node_pool` only treats `NotFound` as successful disappearance; other errors propagate
- Tests: Rust unit tests for error mapping, Playwright E2E for error display/dismiss/retry recovery, mock-tauri error injection support

**Decision:** Keep node pool inventory as authoritative ARM `agentPools` reads. Treat ARM failures as first-class user-visible errors with actionable remediation.

---

### 2026-03-19: Helm Uninstall Implementation

**Authors:** Ripley (engine + IPC), Kane (tests)  
**Status:** Accepted  
**Type:** Feature implementation

**Context:** Helm uninstall was prioritized as the first Helm write operation (P1-3) because it is the smallest safe slice — no chart payloads, only release identity and namespace.

**Changes:**
- Engine: `telescope_engine::helm::helm_uninstall(namespace, name)` with trusted binary resolution, input validation, CLI execution via `tokio::process::Command(kill_on_drop)`, and categorized error messaging (release-not-found, permission denied, timeout)
- Desktop IPC: `helm_uninstall` Tauri command with namespace/name validation, audit logging, registered in `generate_handler!`
- Tests: 3 new Rust unit tests for uninstall error categorization (engine tests 94→97); E2E specs for action/confirm/success/error flows

**Decision:** Follows established Helm rollback pattern. Reuses existing Kubernetes-name validator and trusted Helm binary resolution.

---

### 2026-03-19: P2 Resource List Routes (ReplicaSets, ClusterRoles, ClusterRoleBindings)

**Authors:** Lambert (frontend), Kane (tests)  
**Status:** Accepted  
**Type:** Feature implementation

**Context:** Per the post-audit P2 backlog, these three resource types were already watched by the backend but lacked dedicated frontend list pages.

**Changes:**
- Frontend: 3 new list routes under `apps/web/src/routes/resources/` (replicasets, clusterroles, clusterrolebindings) using standard pattern (`getResources`, `FilterBar`, `ResourceTable`, Svelte 5 runes)
- Routing: Updated `resource-routing.ts` with list/detail mappings for all three types
- Navigation: Sidebar links and search palette entries for all three types
- Tests: E2E specs for route rendering, columns, detail click-through, search palette discovery, loading/error states; `commandDelays` mock-tauri support

**Columns:**
- ReplicaSets: name, namespace, desired/current/ready replicas, age
- ClusterRoles: name, creation timestamp, rules count
- ClusterRoleBindings: name, role ref, subjects, creation timestamp

**Decision:** No new API wrappers needed — `getResources(gvk)` covers all three. Completes the P2 backlog from the post-audit prioritization.

---

### 2026-03-19: Helm Uninstall UI Wiring

**Authors:** Lambert (frontend)  
**Status:** Accepted  
**Type:** Feature implementation (UI layer)

**Context:** Helm uninstall engine + IPC were already delivered (P1-3). This completes the user-facing UI layer — confirmation dialog, success/error notifications, and post-action refresh.

**Changes:**
- Frontend: Wired `helmUninstall` into `apps/web/src/routes/helm/+page.svelte` with per-row Uninstall action button, `$state`-driven ConfirmDialog, success/error notification banners, and post-action list refresh
- UX: ConfirmDialog role set to `dialog` for Playwright E2E compatibility
- Validation: `pnpm build` [ok], `pnpm test` [ok] (36/36), helm uninstall E2E tests pass

**Decision:** Follows the established Helm rollback action pattern. P1-3 (Helm uninstall) is now fully delivered end-to-end: engine → IPC → UI → tests.

---

### 2026-03-19: GitHub Pages Documentation Site Structure

**Author:** Dallas (Lead)  
**Status:** Accepted  
**Type:** Documentation architecture

**Context:** Telescope needed a public-facing documentation site. The `docs/` directory had 11+ markdown files with no navigation, theming, or deployment pipeline.

**Changes:**
- Theme: `just-the-docs` v0.10.1 via `remote_theme` — searchable, sidebar navigation, native mermaid support
- Color scheme: Dark mode default (matches the IDE aesthetic)
- Deployment: GitHub Actions workflow (`docs.yml`) with `actions/deploy-pages@v4`
- Navigation: 8 public sidebar pages (nav_order 2–9), 3 internal pages (`nav_exclude: true`), AGENTS.md/retrospectives/diagrams excluded from Jekyll
- Landing page: `docs/index.md` with project overview, navigation table, tech stack links
- No content changes: Only front matter additions to existing files

**Decision:** `just-the-docs` is the standard for open-source technical docs. Internal planning docs stay accessible by URL but hidden from public navigation. CHANGELOG linked externally to avoid duplication drift.

---

### 2026-03-19: README Cleanup and Restructure

**Author:** Lambert (Frontend Dev)  
**Status:** Accepted  
**Type:** Documentation improvement

**Context:** README needed to be cleaner, more scannable, and compatible with both GitHub rendering and GitHub Pages/Jekyll publishing.

**Changes:**
- Hero section with badges and concise value proposition
- Feature highlight reel replacing long emoji bullet lists
- Scannable feature matrix tables for Kubernetes, Azure, and desktop capabilities
- Streamlined quick start and normalized development commands
- Clean footer with license, docs, and contributing links
- Pure markdown throughout (no raw HTML)

**Decision:** All existing information preserved; restructured for scannability. Markdown-only approach reduces rendering risk across GitHub and Jekyll contexts.

---

---

### 2026-03-20: Ripley Decision: cluster-metrics polling at 5s

**Authors:** Ripley (agent)  
**Requested by:** Adam Dost  
**Status:** Accepted  
**Scope:** `apps/web/src/lib/realMetrics.ts`, `apps/web/src/lib/realMetrics.test.ts`

## Context

Cluster vitals polling cadence was set to 10 seconds in `realMetrics.ts`. The task required moving cluster metrics polling to 5 seconds while keeping runtime behavior stable and tests consistent.

## Decision

Set `POLL_INTERVAL_MS` from `10_000` to `5_000` in `apps/web/src/lib/realMetrics.ts`, and updated timer-based unit test expectations in `apps/web/src/lib/realMetrics.test.ts` to advance fake timers by `5_000`.

## Why this is safe

- Polling deduplication remains intact via `inFlightPoll`, preventing overlapping fetches under slower API responses.
- Unavailable metrics behavior still resets history buffers without throwing.
- Existing ring buffer size and update logic are unchanged.

## Validation

- `pnpm -C apps/web test -- --run src/lib/realMetrics.test.ts` [ok]
- `pnpm -C apps/web build` [ok]

---

### 2026-03-20: Lambert Decision: Metrics Precision Tuning

**Authors:** Lambert  
**Status:** Accepted  
**Type:** UX Improvement

**Context:**

CPU and memory metrics displays were experiencing visual instability (jitter/boomeranging) when values crossed formatting thresholds. This created a poor UX where metrics would rapidly flip between display formats even when underlying values changed minimally.

**Decision:**

Implemented precision tuning in `apps/web/src/lib/metrics-format.ts` using hysteresis thresholds and consistent percent precision:

CPU Formatting:
- Small values (< 100m): display with 1 decimal place (e.g., "5.3m", "99.9m")
- Large values (≥ 100m): round to nearest integer (e.g., "100m", "251m", "1000m")

Memory Formatting:
- Use binary units (`B`, `KiB`, `MiB`, ...)
- Hysteresis threshold: only transition to the next unit at 95% of the unit boundary (e.g., 972.8 bytes for 1 KiB)
- Effect: values remain in the lower unit until passing the hysteresis point, reducing rapid unit switching

Percent Formatting:
- Always format percentages with 1 decimal place (e.g., "35.7%", "100.0%")

**Testing:**

- Added 12 unit tests covering CPU decimal threshold behavior, memory hysteresis across unit boundaries, percent formatting consistency, and edge cases (null, undefined, NaN, Infinity, negative values).
- Tests targeted at `apps/web/src/lib/metrics-format.test.ts` and all pass in CI.

**Files Changed:**

- `apps/web/src/lib/metrics-format.ts` — Added hysteresis constants and conditional decimal logic
- `apps/web/src/lib/metrics-format.test.ts` — Expanded test coverage

**Impact:**

- UX: metrics displays now stable and easier to read
- Truthfulness: raw values remain unchanged; only presentation formatting improved
- Consistency: all metrics surfaces use shared formatters
- Production-safe: no breaking changes to data flow or existing display contracts

**Validation:**

- `pnpm -C apps/web test -- --run src/lib/metrics-format.test.ts` [ok]
- `pnpm -C apps/web build` [ok]

## Governance

- All meaningful changes require team consensus
- Document architectural decisions here
- Keep history focused on work, decisions focused on direction

---

### 2026-03-20: Align Docs with Shipped Desktop Architecture

**Author:** Dallas  
**Status:** Accepted  
**Type:** Documentation architecture

**Decision:** Updated architecture documentation to reflect the current shipped, desktop-only system and the canonical workspace shape. Small, targeted doc edits are preferred over speculative additions. When docs disagree with code or CI, update docs and record a short decision.

**Files changed:**
- `docs/ARCHITECTURE.md` (clarified frontend description)
- `.squad/agents/dallas/history.md` (appended learning entry)

**Rationale:** The repository's source of truth is the code and CI. Keeping docs aligned reduces onboarding friction and prevents stale guidance from influencing design decisions.

---

### 2026-03-23: No-Emoji Policy for Docs, Prompts, Logs, and UI

**Author:** Dallas  
**Status:** Accepted  
**Type:** Communication and iconography policy

**Context:** Emoji usage had become inconsistent across prompts, docs, orchestration outputs, and UI text. Telescope already has guidance to prefer plain text labels and a standardized icon system.

**Decision:** Do not add emojis to docs, prompts, orchestration logs, or UI text. Use plain text labels or the icon registry when a visual indicator is needed, and replace status markers with markdown checkboxes or neutral headings.

**Implications:**
- New squad outputs should use plain role labels and neutral status text
- UI work should prefer the icon registry over inline emoji glyphs
- Documentation cleanup should remove emoji markers when touched

---

### 2026-03-24: AI Insights v1 Implementation Plan and Scope Locks

**Authors:** Adam Dost (directives), Dallas (lead), Ripley, Lambert, Kane  
**Status:** Accepted  
**Type:** Implementation planning and architecture

**Context:** AI Insights planning was finalized across product directives, backend/frontend/test specialist briefs, and the implementation plan artifact in `docs/plans/2026-03-24-ai-insights-implementation.md`.

**Decision:**
- Ship AI Insights as a dedicated `/insights` route rather than an Overview tab
- Support two explicit auth modes in v1: Azure login context via `DefaultAzureCredential()` and API key; do not auto-fallback between them
- Expose Azure OpenAI endpoint, deployment/model, auth mode, and cloud profile in Settings; keep diagnostics limited to dev-mode metadata on Settings
- Keep ownership aligned to the existing architecture:
	- `crates/azure` owns Azure OpenAI auth, endpoint/cloud handling, and provider error classification
	- `crates/engine` owns allowlist-only context shaping, prompt/schema contracts, and insight orchestration models
	- `crates/core` owns encrypted local history persistence
	- `apps/desktop/src-tauri` owns secure credential storage integration and thin Tauri commands
	- `apps/web` owns the `/insights` route and Settings UX
- Persist encrypted local insight history, retain only the last 3 entries per cluster, and expose clear-all only for deletion
- Build model input from curated summaries with deterministic per-category caps, stable ordering, and hard exclusion of secrets, kubeconfigs, token-like values, and raw unsafe payloads
- Render only schema-validated structured output in the UI
- Keep v1 intentionally narrow: no prompt preview, no chat surface, no browser fallback, and no per-entry delete

**Testing direction:**
- Test Azure login and RBAC-denied behavior with fake credential and transport seams, not live Azure dependencies
- Validate encrypted history at the storage boundary with ciphertext-at-rest and per-cluster retention assertions
- Add deterministic unit tests for allowlist-only context shaping, ordering, and cap enforcement before UI/E2E coverage
- Use existing mocked Tauri Playwright flows for route, settings, and error-path coverage

**Artifacts:**
- `docs/plans/2026-03-24-ai-insights-implementation.md`
- Merged inbox notes from Dallas, Ripley, Lambert, Kane, and the three AI Insights directive updates

---

### 2026-03-24: AI Insights Task 1 Contract Surface Accepted

**Authors:** Ripley (engine contract + frontend revision), Dallas (final persistence fix), Kane (QA acceptance)  
**Status:** Accepted  
**Type:** Contract implementation

**Context:** Task 1 locked the first shipped AI Insights contract across Rust and TypeScript. The engine contract landed first in `crates/engine/src/insights.rs`. The initial frontend contract implementation drifted from the Rust schema and reused the shared Azure cloud preference path, so Dallas rejected it. Ripley revised the frontend contract layer, and a second review found one remaining persistence bug where `modelName: null` could round-trip as `""`. Dallas fixed that final wrapper issue and Kane accepted the slice.

**Changes:**
- Added the engine-owned request, response, settings, diagnostics, and stable preference-key contract in `crates/engine/src/insights.rs`
- Exported the AI Insights contract surface through `apps/web/src/lib/tauri-commands.ts`
- Added strict frontend validators and helpers in `apps/web/src/lib/insights.ts`
- Added dedicated AI Insights settings wrappers in `apps/web/src/lib/api.ts` using `ai_insights_*` preference keys only
- Preserved `modelName` optionality by storing the empty-string sentinel and normalizing it back to `null` on load
- Added targeted frontend contract and persistence tests in `apps/web/src/lib/insights.test.ts` and `apps/web/src/lib/api.test.ts`

**Decision:**
- Treat the Rust engine contract as the source of truth for AI Insights serialization and enum values
- Keep AI Insights settings isolated from the shared Azure cloud preference/localStorage path
- Fail closed on unknown response and diagnostics fields in the frontend validators
- Preserve Rust `Option<String>` semantics for `modelName` across the persistence wrapper boundary

**Validation:**
- `cargo test -p telescope-engine --lib insights` [ok]
- `pnpm -C apps/web test -- --run src/lib/insights.test.ts src/lib/api.test.ts` [ok]
- `pnpm -C apps/web build` [ok]

---

### 2026-03-24: AI Insights Task 2 Azure OpenAI Transport Accepted

**Authors:** Ripley (initial transport), Dallas (transport corrections), Kane (QA acceptance)  
**Status:** Accepted  
**Type:** Provider transport implementation

**Context:** Task 2 added the first Azure OpenAI transport seam in `crates/azure`. Ripley delivered the initial transport, then Dallas rejected it because sovereign cloud authority selection was incomplete for Azure-login mode, endpoint validation silently accepted pasted `/openai/...` request paths, and provider-side 401s were conflated with local credential failures. Dallas implemented the correction slice in `crates/azure`, and Kane accepted the revised transport.

**Changes:**
- Added Azure OpenAI transport wiring in `crates/azure/src/openai.rs` and exported it from `crates/azure/src/lib.rs`
- Applied cloud-specific `TokenCredentialOptions` authority host selection for Azure-login mode
- Tightened endpoint validation to accept only the Azure OpenAI resource root and reject non-root path, query-string, and fragment inputs
- Split provider-side authentication and authorization failures into clearer AI-specific `AzureError` variants and messages
- Extended targeted transport and error coverage in `cargo test -p telescope-azure openai` and `cargo test -p telescope-azure error`

**Decision:**
- Keep the Azure OpenAI seam inside `crates/azure`
- Require the selected `AzureCloud` to drive both endpoint validation and `DefaultAzureCredential` authority construction
- Keep Azure login and API key as explicit user-selected auth modes with no fallback between them
- Surface provider-side auth failures distinctly enough for future Settings test-connect guidance and dev diagnostics

**Validation:**
- `cargo test -p telescope-azure openai` [ok]
- `cargo test -p telescope-azure error` [ok]

---

### 2026-03-24: AI Insights Task 3 Context Builder Accepted

**Authors:** Ripley (implementation), Kane (QA acceptance)  
**Status:** Accepted  
**Type:** Engine context shaping implementation

**Context:** Task 3 delivered the engine-owned AI Insights context builder in `crates/engine`. Ripley implemented an allowlist-only builder with deterministic category caps, stable ordering, namespace-aware scope handling, and explicit redaction safeguards. Kane reviewed the slice and accepted it with low-risk follow-up notes only.

**Changes:**
- Added the AI Insights context-builder entry point in `crates/engine/src/insights_context.rs`
- Extended the engine AI Insights contract in `crates/engine/src/insights.rs` for capped, curated context categories and redaction-policy metadata
- Exported the new engine modules from `crates/engine/src/lib.rs`
- Built allowlist-only summaries for workloads, pods, warning events, nodes, Helm releases, connection state, and narrow AKS posture input
- Enforced fixed deterministic caps, stable ordering, and explicit redaction for token-like values, kubeconfig-looking text, connection strings, and service-account credential material
- Made namespace scope explicit and dropped cluster-only sections such as node posture and AKS posture when the request is namespace-limited

**Decision:**
- Keep the builder pure and deterministic over `ResourceStore` cache data plus explicit inputs for `ConnectionState`, Helm release summaries, and a narrow AKS summary
- Keep the allowlist boundary in `crates/engine`; do not serialize raw Kubernetes objects, raw Helm values, or secret payloads into model context
- Treat namespace-limited visibility as a hard boundary and omit cluster-only sections rather than summarizing partial cluster posture

**Residual notes:**
- Kane accepted the slice with two low-risk follow-ups: add one focused cap-overrun test for the remaining pod, event, node, and Helm categories, and add one namespace-scope test that explicitly filters cross-namespace pods and events
- Existing workspace-level `cargo fmt --all -- --check` issues in `crates/azure` and existing `pnpm -C apps/web build` type errors remain outside this Task 3 slice

**Validation:**
- `cargo test -p telescope-engine insights_context` [ok]
- `cargo test -p telescope-engine insights` [ok]

---

### 2026-03-24: AI Insights Deficiency Fixes (Tasks 2-3)

**Authors:** Ripley (fixes), Dallas (review), Kane (QA acceptance)
**Status:** Accepted
**Type:** Bug fix + test gap closure

**Context:** Post-acceptance review of the AI Insights implementation identified three deficiency classes: (1) `response_format_json()` in `crates/azure/src/openai.rs` serialized `"description": null` when the field was `None`, which Azure OpenAI's structured output API rejects; (2) HTTP 408/504 timeouts and 429 rate-limit responses fell through to the generic `OpenAiApi` catch-all instead of mapping to their correct error variants; (3) cap enforcement tests for pod, event, node, and Helm release categories and cross-namespace filtering for pods and events were missing from `crates/engine/src/insights_context.rs`.

**Changes:**
- Fix 1 (schema serialization): `response_format_json()` now conditionally inserts the `"description"` key only when `Some`, preventing a `null` value on the wire. The struct-level `#[serde(skip_serializing_if)]` annotation is redundant but harmless since manual `Value` construction controls the wire path.
- Fix 2 (HTTP status classification): `classify_openai_response_error()` now explicitly branches on 408 and 504 to return `AzureError::OpenAiTimeout`, and on 429 to return `OpenAiApi` with code `"TooManyRequests"`. Ordering is safe -- these checks follow 401/403/404 branches.
- Fix 3 (test gap closure): Five new tests in `insights_context.rs` cover cross-namespace pod/event filtering, and individual cap enforcement for pod, event, node, and Helm release categories. All use contract constants (`AI_INSIGHTS_POD_CAP`, etc.) and assert both `total_count` and `items.len()`.

**Review outcomes:**
- Dallas approved: all three fixes are narrow, correctly targeted, contract-aligned, and have dual-path or boundary-condition coverage
- Kane approved: all 37 targeted tests pass with no false positives detected

**Validation:**
- `cargo test -p telescope-azure` 76 passed
- `cargo test -p telescope-engine` 130 passed (115 lib + integration)
- `cargo test -p telescope-core` 40 passed
- `pnpm -C apps/web test` 11 passed
- `pnpm -C apps/web build` [ok]

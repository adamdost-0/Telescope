# Telescope — Security Notes

> **Status: Partially implemented** — Telescope ships real desktop security controls, but the full security model is still incomplete.

## Threat model (baseline)
- Workstations may contain credentials to production clusters.
- UI must avoid accidental destructive actions.
- Secrets must not leak via local caches, logs, YAML views, or audit trails.
- Desktop storage and logs should remain scoped to the local user account.

## Current implementation snapshot
- **Desktop connection state is explicit:** the shared `ConnectionState` machine tracks `Disconnected`, `Connecting`, `Syncing`, `Ready`, `Degraded`, `Error`, and `Backoff`, which lets the UI surface authentication/connection failures clearly.
- **Secrets are redacted by default:** secret list/detail reads bypass the shared cache and redact `data`, `stringData`, `binaryData`, and the `kubectl.kubernetes.io/last-applied-configuration` annotation.
- **Audit logging exists today:** the engine writes JSONL audit entries for selected local actions such as secret access and pod-log fetches.
- **Sensitive local files are permissioned on Unix:** Telescope creates the audit log and SQLite DB with restrictive `0600` permissions when running on Unix-like systems.

## Kubeconfig & credentials
- Telescope reads kubeconfig contexts and builds Kubernetes clients from the existing kubeconfig rather than copying credentials into a separate credential store.
- The app relies on the local operator's Kubernetes identity and environment.
- OS keychain-backed token storage is still planned, not implemented.

## Secrets
- Secret list/detail APIs intentionally fetch secrets on demand instead of storing them in the shared watched-resource cache.
- The engine redacts secret payload fields before serializing them for the UI.
- The same redaction approach is used for known-sensitive Helm values keys (for example `password`, `token`, `connectionString`, and similar variants).
- The UI supports viewing masked secret content and prevents naïve re-apply of masked YAML.
- Per-key reveal flows, reveal timeouts, and secure local persistence are still future work.

## Actions safety
- The connection-state machine gives the UI explicit feedback for connect/auth/backoff/error flows instead of silently failing.
- Production-context detection is implemented in the UI and is used to show prominent warnings and stronger confirmation dialogs.
- Destructive action safeguards are partial today:
  - confirmation dialogs exist for destructive operations,
  - YAML dry-run/apply exists for supported resources,
  - rollout and scale actions are implemented for supported workload kinds.
- Still missing/planned:
  - broad RBAC capability pre-checks before every mutation,
  - diff preview everywhere,
  - universal server-side dry-run enforcement,
  - consistent delete coverage across every resource detail page.

## Audit logging
- `crates/engine/src/audit.rs` appends structured JSON lines with actor, context, namespace, action, resource type/name, result, and optional detail.
- Current local audit coverage includes selected actions such as:
  - cluster connect/disconnect,
  - listing secrets,
  - fetching a specific secret,
  - reading pod logs.
- Current limitation: audit coverage is not yet comprehensive for every write action or UI workflow.

## Plugins
- No plugin system is implemented yet.
- The earlier WASM/capability model remains a design target rather than an active security boundary.

## Telemetry
- There is no notable production telemetry/security analytics pipeline yet.
- The “opt-in only telemetry” principle remains the intended policy for future work.

## Still planned / not yet complete
- Stronger desktop-side credential handling.
- Real authorization-style safety checks for sensitive operations.
- Stronger mutation safety checks (RBAC preflight, diff preview, broader dry-run coverage).
- Secret reveal workflows, secret-at-rest protection beyond kubeconfig reuse, and richer compliance/audit features.

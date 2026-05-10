---
title: Security
nav_order: 6
description: "Threat model, secret handling, audit logging, and credential management"
---

# Telescope — Security Notes

> **Status: v1.0.0** — Telescope ships desktop security controls for Kubernetes access, secret handling, audit logging, and Azure ARM authentication.

## Threat model (baseline)
- Workstations may contain credentials to production clusters.
- UI must avoid accidental destructive actions.
- Secrets must not leak via local caches, logs, YAML views, or audit trails.
- Desktop storage and logs should remain scoped to the local user account.
- Azure ARM credentials must not be stored or embedded by Telescope.

## Current implementation snapshot
- **Desktop connection state is explicit:** the shared `ConnectionState` machine tracks `Disconnected`, `Connecting`, `Syncing`, `Ready`, `Degraded`, `Error`, and `Backoff`, which lets the UI surface authentication/connection failures clearly.
- **Secrets are redacted by default:** secret list/detail reads bypass the shared cache and redact `data`, `stringData`, `binaryData`, and the `kubectl.kubernetes.io/last-applied-configuration` annotation. The redaction placeholder is `●●●●●●●●`.
- **Helper execution is pinned to trusted binaries:** kubeconfig `exec` auth helpers, Azure CLI fallback calls, and Helm rollback/uninstall only run when Telescope can resolve them to a trusted installation path. Relative or unknown helper commands are blocked before execution.
- **Helm values are redacted and raw reveal is denied:** sensitive keys (`password`, `passwd`, `secret`, `token`, `apikey`, `api_key`, `apiKey`, `connectionstring`, `connection_string`, `connectionString`, `private_key`, `client_secret`, `access_key`, `secret_key`, `credentials`, `auth`) are recursively redacted in Helm release values. Requests to reveal raw values are denied before values are fetched, audited as denied attempts, and returned as sanitized errors.
- **SQLite cache redaction is broader than v1.0.0:** cached resources now redact Pod/workload `env`, `command`, and `args`; annotation values; ConfigMap payloads; webhook client URLs / CA bundles; and secret-shaped fields before they are written to `resources.db`.
- **Cached resource reads are allowlisted:** cached `gvk` arguments are validated against the watched GVK registry before the desktop command queries SQLite. Unsupported cached resource types return a generic sanitized error instead of echoing caller input.
- **User-facing command/API errors are sanitized:** Helm command failures and frontend IPC errors remove local paths, raw command output, and secret-shaped values before they reach notifications or console logs.
- **Audit logging covers destructive and sensitive operations:** the engine writes JSONL audit entries for connection lifecycle, AKS node pool operations, Helm rollbacks, Helm uninstall, denied Helm values reveal attempts, namespace create/delete, resource apply/delete/scale, rollout restarts, node cordon/uncordon/drain/taint, and exec commands.
- **Sensitive local files are permissioned on Unix:** Telescope creates `~/.telescope/audit.log` and `~/.telescope/resources.db` with restrictive `0600` permissions.

## Kubeconfig & credentials
- Telescope reads kubeconfig contexts and builds Kubernetes clients from the existing kubeconfig rather than copying credentials into a separate credential store.
- The app relies on the local operator's Kubernetes identity and environment.
- Kubeconfig `exec` auth helpers are allowed only from Telescope's trusted installation list; arbitrary or relative helper commands are blocked during connect.
- OS keychain-backed token storage is still planned, not implemented.

## Azure ARM security

### Authentication
- `ArmClient` (in `crates/azure/src/client.rs`) uses `azure_identity::DefaultAzureCredential` for all ARM API access.
- **No Azure credentials are stored by Telescope.** The credential chain delegates to the environment: Azure CLI session, environment variables, managed identity, or workload identity.
- `ArmClient::new(cloud)` constructs the credential at call time; tokens are acquired per-request and scoped to the ARM management endpoint.

### Token scope
- Tokens are scoped to the ARM endpoint for the configured cloud:
  - Commercial: `https://management.azure.com/.default`
  - US Government: `https://management.usgovcloudapi.net/.default`
  - US Gov Secret / Top Secret: cloud-specific ARM endpoints
- Telescope never requests broader scopes than required for ARM operations.

### RBAC requirements
- ARM operations require the operator's Azure identity to hold appropriate RBAC roles on the AKS resource. Telescope does not elevate privileges or cache Azure tokens.
- Recommended minimum: `Azure Kubernetes Service Contributor` for management operations, `Reader` for read-only views.

### ARM audit trail
- All AKS management operations performed through `crates/azure` are logged to the local audit log (`~/.telescope/audit.log`) with the same `AuditEntry` format used for Kubernetes operations.
- Audited ARM operations include: `scale_aks_node_pool`, `update_aks_autoscaler`, `create_aks_node_pool`, `delete_aks_node_pool`.
- ARM operations are also recorded server-side in the Azure Activity Log by Azure Resource Manager.

## Secrets
- Secret list/detail APIs intentionally fetch secrets on demand instead of storing them in the shared watched-resource cache.
- The engine redacts secret payload fields before serializing them for the UI.
- The UI treats redacted secret YAML as masked data and prevents naive re-apply of masked content.
- Per-key secret reveal flows, reveal timeouts, and secure local persistence are still future work.

## Helm release values
- Normal Helm values requests (`reveal: false` or omitted) fetch release values and recursively redact sensitive keys before returning YAML to the UI.
- Raw Helm values reveal requests (`reveal: true`) are denied before Telescope creates a Kubernetes client or fetches Helm values.
- Denied reveal attempts write an audit entry with action `helm_values_reveal` and result `denied`.
- The frontend wrapper no longer accepts or sends a reveal flag, and the Helm detail page no longer renders plaintext reveal controls.
- A nonce challenge or scoped plaintext reveal policy is future/proposed only. Any future plaintext reveal must require short-lived nonce verification before an approved audit entry is written and scoped plaintext values are returned. The current implementation has no plaintext Helm values return path.

## Actions safety
- The connection-state machine gives the UI explicit feedback for connect/auth/backoff/error flows instead of silently failing.
- Production-context detection is implemented in the UI and is used to show prominent warnings and stronger confirmation dialogs.
- Destructive action safeguards:
  - Confirmation dialogs exist for destructive operations.
  - YAML dry-run/apply exists for supported resources.
  - Rollout and scale actions are implemented for supported workload kinds.
  - Node operations (cordon, drain, taint) require confirmation.
- Still missing/planned:
  - Broad RBAC capability pre-checks before every mutation.
  - Diff preview everywhere.
  - Universal server-side dry-run enforcement.

## User-facing error sanitization
- Command handlers should return user-safe categories such as not found, permission denied, timeout, command unavailable, or command failed.
- Public errors must not include helper binary paths, kubeconfig paths, raw `stdout`/`stderr`, tokens, passwords, or secret-shaped key/value material.
- Frontend IPC wrappers sanitize thrown errors and listener messages before showing notifications or logging command-level failures.
- Detailed diagnostics belong in explicit debug or audit channels, not default UI text or console output.

## Audit logging
- `crates/engine/src/audit.rs` appends structured JSONL entries with: `timestamp`, `actor`, `context`, `namespace`, `action`, `resource_type`, `resource_name`, `result`, `detail`.
- **Audited operations:** cluster connect/disconnect, AKS node pool scale/create/delete/autoscaler, Helm rollback, Helm uninstall, denied Helm values reveal attempts, namespace create/delete, resource apply/delete/scale, rollout restart, node cordon/uncordon/drain/taint add/remove, exec commands.
- Audit log location: `~/.telescope/audit.log` (permissions `0600` on Unix).

## Plugins
- No plugin system is implemented yet.
- The WASM/capability model remains a design target rather than an active security boundary.

## Telemetry
- There is no production telemetry pipeline.
- The "opt-in only telemetry" principle remains the intended policy for future work.

## Still planned / not yet complete
- OS keychain envelope encryption for stored tokens.
- RBAC capability pre-checks before every mutation.
- Diff preview for all apply operations.
- Secret reveal workflows with timeouts and secure local persistence.
- Scoped Helm plaintext reveal policy, if product requirements ever justify one; current behavior denies plaintext reveal.

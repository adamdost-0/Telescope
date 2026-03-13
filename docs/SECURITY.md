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
- **Helm values are redacted:** sensitive keys (`password`, `passwd`, `secret`, `token`, `apikey`, `api_key`, `apiKey`, `connectionstring`, `connection_string`, `connectionString`, `private_key`, `client_secret`, `access_key`, `secret_key`, `credentials`, `auth`) are recursively redacted in Helm release values. Reveal requires an explicit user action with a UI warning.
- **Audit logging covers destructive operations:** the engine writes JSONL audit entries for connection lifecycle, AKS node pool operations, Helm rollbacks, namespace create/delete, resource apply/delete/scale, rollout restarts, node cordon/uncordon/drain/taint, and exec commands.
- **Sensitive local files are permissioned on Unix:** Telescope creates `~/.telescope/audit.log` and `~/.telescope/resources.db` with restrictive `0600` permissions.

## Kubeconfig & credentials
- Telescope reads kubeconfig contexts and builds Kubernetes clients from the existing kubeconfig rather than copying credentials into a separate credential store.
- The app relies on the local operator's Kubernetes identity and environment.
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
- The UI prevents naïve re-apply of masked YAML and warns when revealing unredacted values.
- Per-key reveal flows, reveal timeouts, and secure local persistence are still future work.

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

## Audit logging
- `crates/engine/src/audit.rs` appends structured JSONL entries with: `timestamp`, `actor`, `context`, `namespace`, `action`, `resource_type`, `resource_name`, `result`, `detail`.
- **Audited operations:** cluster connect/disconnect, AKS node pool scale/create/delete/autoscaler, Helm rollback, namespace create/delete, resource apply/delete/scale, rollout restart, node cordon/uncordon/drain/taint add/remove, exec commands.
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

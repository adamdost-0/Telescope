# Telescope — Security Notes (Draft)

## Threat model (baseline)
- Workstations may contain credentials to production clusters.
- UI must avoid accidental destructive actions.
- Secrets must not leak via logs, crash reports, or plugin misuse.

## Kubeconfig & credentials
- Prefer storing **references** to kubeconfig paths; avoid copying.
- If derived tokens must be stored, encrypt-at-rest using OS keychain:
  - Windows Credential Manager
  - macOS Keychain
  - Linux Secret Service

## Secrets
- Do not fetch Secret `.data` unless explicitly requested.
- Mask by default; reveal per-key with timeout.
- Never persist secret values to disk by default.

## Actions safety
- Default to read-only mode for new clusters.
- Destructive actions require:
  - RBAC capability check
  - diff preview
  - server-side dry-run (`dryRun=All`) where supported
  - explicit confirmation

## Plugins
- Default plugin format: WASM with capability-based permissions.
- No filesystem/network access without explicit grant.
- Resource mutations require explicit permission and show in local audit log.

## Telemetry
- Opt-in only; clearly documented payloads.

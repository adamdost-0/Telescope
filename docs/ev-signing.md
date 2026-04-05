# EV code signing plan (Windows + macOS)

Telescope already signs and optionally notarizes macOS builds in `.github/workflows/release.yml`. Windows bundles are currently unsigned. This document captures what is required to ship **EV-signed** Windows artifacts and keep macOS artifacts trusted.

## What “EV” changes
- **Windows:** Authenticode signing must use an EV certificate (hardware-backed HSM or Azure Key Vault with an EV-backed key). GH-hosted runners cannot access USB tokens, so use either a self-hosted Windows runner with the EV token attached or Azure Key Vault Code Signing (recommended).
- **macOS:** Developer ID Application certificates are already accepted; using a hardware token is supported on a self-hosted macOS runner. Notarization remains required for gatekeeper trust.

## Pipeline changes (Windows)
Add the following to `.github/workflows/release.yml`:
1. Detect Windows EV signing secrets and set `WINDOWS_EV_SIGNING_ENABLED=true`.
2. Install AzureSignTool (or SignTool if using a thumbprint on a hardware token).
3. Sign **all** Windows outputs after they are renamed:
   - Portable EXE (`target/release/${VERSIONED_PORTABLE}`)
   - MSI installer (`${MSI_ARTIFACT}` when present)
   - NSIS installer (`${NSIS_ARTIFACT}` when present)
4. Fail the job if signing is requested but any artifact is missing or signing fails.

Recommended secrets/vars (Azure Key Vault Code Signing):
- `WINDOWS_EV_AZURE_VAULT_URL` – Key Vault URL
- `WINDOWS_EV_AZURE_CERT_NAME` – Certificate name in Key Vault
- `WINDOWS_EV_AZURE_CLIENT_ID` / `WINDOWS_EV_AZURE_TENANT_ID` – AAD app registration
- `WINDOWS_EV_AZURE_CLIENT_SECRET` – client credential (or use OIDC + federated credential)
- Optional: `WINDOWS_EV_TIMESTAMP_URL` (default: `http://timestamp.digicert.com`)

Runner requirement: `windows-latest` works with Azure Key Vault Code Signing. Hardware-token EV certs require a self-hosted Windows runner with the token attached and `signtool` access via certificate thumbprint.

## Pipeline changes (macOS)
- Keep the existing Developer ID + notarization block in `release.yml`.
- For EV hardware tokens, run the macOS matrix leg on a self-hosted runner with the token attached; set `APPLE_SIGNING_IDENTITY` to the hardware identity name and skip `APPLE_CERTIFICATE/APPLE_CERTIFICATE_PASSWORD`.
- Ensure `APPLE_ID`, `APPLE_PASSWORD`, and `APPLE_TEAM_ID` remain populated for notarization.

## Validation
- The release job already verifies macOS signatures and DMG contents.
- Add Windows verification by checking `signtool verify /pa` on the signed EXE/MSI/NSIS when EV signing is enabled (optional follow-up).

## Definition of done
- Windows artifacts are signed with the EV certificate in `release.yml` (gated by secrets).
- macOS artifacts continue to be Developer ID–signed and notarized when credentials exist.
- Both platforms produce trusted installers without manual post-processing.

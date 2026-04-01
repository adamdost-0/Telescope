# Autopilot Prompt for Deep Repository Security Review

Use the prompt below with Copilot in autopilot mode when you want a rigorous, evidence-first security review of the Telescope repository without speculative or hallucinated findings.

## Recommended Prompt

```text
Perform a deep, read-only security review of the Telescope repository.

Repository context you must anchor on before analyzing:
- Telescope is a desktop-only Tauri v2 Kubernetes IDE, not a browser/SaaS web app.
- Cargo workspace members are `crates/core`, `crates/engine`, `crates/azure`, and `apps/desktop/src-tauri`.
- The frontend is `apps/web`, packaged into the desktop app.
- `apps/web/src/lib/api.ts` uses Tauri `invoke()` for desktop IPC; there is no HTTP fallback.
- The Tauri command surface is centered in `apps/desktop/src-tauri/src/main.rs`.
- CI validation commands are defined in `.github/workflows/ci.yml`.
- Treat code and CI behavior as the source of truth; docs may lag implementation.

Goal:
Produce a rigorous, non-hallucinatory security assessment optimized for completeness, exact evidence, and validated testing. Review the actual implemented code in this repo. Zero findings is an acceptable outcome if that is what the evidence supports.

Hard rules:
1. Read-only mode only. Do not edit, patch, commit, or create files.
2. Every claim must cite exact file paths and line ranges in this repo, e.g. `crates/engine/src/foo.rs:120-148`.
3. Distinguish clearly between:
   - Confirmed finding
   - Likely issue needing more validation
   - Hypothesis / potential concern
4. If you cannot verify something from the code or available runtime checks, say so explicitly.
5. Prefer "no issue found in this area" over inventing a vulnerability.
6. For each finding, include concrete reproduction or validation steps using existing repo commands where possible.
7. Do not invent web-server-only issues such as CSRF/CORS/session-cookie bugs unless the repo actually exposes such a surface.
8. Do not treat expected admin capabilities (Kubernetes actions, AKS operations, exec, port-forward) as vulnerabilities unless there is a real trust-boundary, validation, secret-handling, or privilege-exposure flaw.
9. If you cannot point to the code path, line range, and validation evidence, it is not a finding.

Repository-specific scope you must inspect:

A. Rust workspace — inspect every implemented crate
- `crates/core/**`
- `crates/engine/**`
- `crates/azure/**`
- `apps/desktop/src-tauri/**`

B. Desktop/API boundary — inspect privileged command exposure across all layers
1. Desktop command surface:
   - `apps/desktop/src-tauri/src/main.rs`
   - any delegated Tauri command modules it registers via `generate_handler![]`
2. Frontend transport contract:
   - `apps/web/src/lib/api.ts`
   - `apps/web/src/lib/tauri-commands.ts`
   - related frontend types/guards/contracts
3. UI invocation paths:
   - relevant files in `apps/web/src/routes/**`
   - relevant components in `apps/web/src/lib/**`

C. Dependency/build/config surface
- root `Cargo.toml`
- `apps/desktop/src-tauri/Cargo.toml`
- root `package.json`
- `apps/web/package.json`
- Tauri/web config files affecting permissions, CSP, build mode, or API exposure

Required review focus areas:

1. Tauri/IPC security
- Privileged command exposure
- Missing validation on command arguments
- Dangerous filesystem/process/network access from commands
- Error leakage from Rust to frontend
- Any invoke surfaces or wrappers that bypass typing/guards

2. Authn/authz and credential handling
- Kubeconfig handling
- Azure credential/token handling
- Secret storage, logging, caching, and persistence
- Whether sensitive values can cross IPC/UI boundaries unintentionally

3. Injection and unsafe execution
- Shell/process invocation
- Path traversal
- SQL/query construction
- YAML/JSON deserialization risks
- Templating or command-string construction
- Any `unsafe` Rust blocks or FFI boundaries

4. Frontend/webview risks
- XSS or UI injection from cluster/resource-controlled data
- Rendering of logs, YAML, annotations, names, labels, and errors
- Trust boundaries between the webview and desktop backend

5. Data exposure / privacy / local attack surface
- Secrets in logs
- Overbroad file access
- Insecure default storage
- Unnecessary persistence of credentials or cluster data

6. Network/API calls
- UI input -> Tauri IPC -> Rust handler -> filesystem/network/OS side effect
- UI input -> Kubernetes API calls
- UI input -> Azure ARM calls
- UI input -> Azure OpenAI / AI Insights calls
- TLS/certificate validation behavior
- SSRF or attacker-controlled outbound endpoint misuse

7. Dependency and configuration risk
- High-risk dependencies
- Security-relevant overrides/pinning
- Debug/dev settings that weaken production posture

Required method:
1. First, build a coverage checklist and do not finish until each required area above is inspected.
2. Enumerate security-relevant code patterns and inspect the matching files, including but not limited to:
   - `#[tauri::command]`
   - `generate_handler!`
   - `unsafe`
   - process execution
   - filesystem read/write
   - serde/YAML/JSON parsing
   - token/secret/credential handling
   - error propagation to UI
3. Inspect every implemented crate, not just the desktop app.
4. At each API boundary, verify both sides:
   - backend command implementation/registration
   - frontend wrapper/contract usage
   - reachable UI entry points that invoke it
5. Validate suspected findings using existing repo commands whenever feasible.

Use these existing repo commands for validation where appropriate:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --exclude telescope-desktop --all-targets --all-features -- -D warnings`
- `cargo test --workspace --exclude telescope-desktop --all-features`
- `pnpm -r --if-present lint`
- `pnpm -r --if-present test`
- `pnpm -C apps/web build`
- `pnpm -C apps/web test`
- if appropriate and feasible: `pnpm -C apps/web exec playwright install --with-deps chromium`
- if appropriate and feasible: `pnpm -C apps/web e2e`

If a command cannot be run in the environment, say exactly which command failed or was skipped and why. Do not pretend it passed.

Required output format:

## 1) Executive summary
- Overall security posture: Strong / Adequate / Needs Improvement / High Risk
- Top risks, if any
- What was actually validated vs only inspected statically

## 2) Coverage manifest
A checklist or table showing each required area inspected, with representative file citations.
This must explicitly mention:
- `crates/core`
- `crates/engine`
- `crates/azure`
- `apps/desktop/src-tauri`
- `apps/web/src/lib/api.ts`
- `apps/web/src/lib/tauri-commands.ts`
- relevant `apps/web/src/routes/**` callers

## 3) Confirmed findings
For each confirmed issue, use exactly this structure:

### [ID] Title
- Severity: Critical | High | Medium | Low | Info
- Confidence: Confirmed
- CWE: if applicable
- Evidence:
  - `path/to/file:line-line`
  - `path/to/other/file:line-line`
- Why this is a real issue:
  - concise explanation tied to the cited code
- Reproduction / validation:
  1. exact steps
  2. exact command(s) to run, if applicable
  3. expected observable result
- Impact:
  - realistic attacker capability and consequence
- Remediation:
  - concrete fix direction tied to the same files

## 4) Likely issues needing more validation
Use the same structure, but mark `Confidence: Likely` and state what is still needed to confirm.

## 5) Hypotheses / potential concerns
Use the same structure, but mark `Confidence: Hypothesis` and explain why the evidence is incomplete.

## 6) No-issue / acceptable areas
Document areas you checked and found no meaningful issue in, with citations. This is required so the review shows what was covered, not just what was flagged.

## 7) Validation log
List the commands you ran, whether they succeeded, and what they validated.

Important review behavior:
- Be skeptical and precise.
- Do not inflate severity.
- Do not report generic best-practice nits as vulnerabilities.
- Do not claim exploitability without showing the code path and the trust boundary.
- If a concern depends on runtime conditions, say that plainly.
- If no confirmed vulnerabilities are found in a category, say so with citations.

Your job is not to sound smart; your job is to be correct, evidenced, and useful.
```

## Why this prompt works

- It anchors the review to Telescope's real architecture, which reduces generic false positives.
- It forces exact file citations and separates confirmed issues from unvalidated suspicions.
- It requires use of existing repository commands for validation, which helps prevent AI hallucinations and unsupported claims.

# Security Test Plan Framework: Issues #200, #201, #202

## Verification vs. Validation Principles

**Verification:** Did we build the fix right?  
- Tests the implementation artifact (code correctness, logic, coverage)
- Unit and integration tests, static checks, code review

**Validation:** Did we solve the real problem?  
- Tests the system behavior against the actual security goal
- End-to-end scenarios, threat scenarios, user workflows, security posture assessment

---

## Issue #200: Exec Audit Log Stores Full Secret-Bearing Commands

### Summary
Full user-supplied exec commands (including secrets) are persisted to `~/.telescope/audit.log`.

### Success Criteria
- ✓ Audit log entries for `exec_command` no longer contain sensitive data in plaintext
- ✓ Basic troubleshooting capability is preserved (i.e., operator can identify *that* a command ran and when)
- ✓ Desktop app startup does not break; log file is correctly located and writable

### Verification: Did We Build the Fix Right?

**1. Unit Tests**
- [ ] Redaction function correctly identifies and masks secrets in command strings
- [ ] Redaction preserves program name and argument count (for forensics)
- [ ] Redaction works on common secret patterns (Bearer tokens, passwords in URLs, env vars)
- [ ] Redaction doesn't mask innocuous commands

**2. Code Coverage**
- [ ] `crates/engine/src/audit.rs` redaction logic has >80% branch coverage
- [ ] `apps/desktop/src-tauri/src/main.rs` exec_command handler passes redacted value to `write_audit_entry`

**3. Integration Tests**
- [ ] Desktop app calls Tauri command with secret; entry written to audit log; secret not present in log file

**4. Static Analysis**
- [ ] No hardcoded plaintext secrets in audit redaction tests
- [ ] `cargo clippy` and `cargo fmt` pass on affected crates

### Validation: Did We Solve the Real Problem?

**1. Threat Scenario Test**
- [ ] End-to-end: user types `curl -H 'Authorization: Bearer secret123' https://example.com` in exec terminal
- [ ] Audit log entry is persisted
- [ ] Log file examined; secret is not present in plaintext
- [ ] No partial leaks (e.g., token head is redacted but tail is visible)

**2. Forensic Utility Test**
- [ ] Audit log still identifies: command was run, timestamp, pod/ns, whether it succeeded
- [ ] Support bundle / log export does not expose secrets

**3. Compliance Scenario**
- [ ] If backup or support logs are shared, they do not contain exec command secrets
- [ ] Unencrypted disk access does not expose secrets via audit log

### Go/No-Go Evidence for Closing Issue #200
- [ ] All verification tests pass (unit, integration, static analysis)
- [ ] Threat scenario test passes; secret not in log
- [ ] Desktop app e2e smoke test (start app, exec command, verify log) passes
- [ ] Human spot-check: inspect audit.log for 3–5 test commands; no plaintext secrets visible
- [ ] CI passes: `pnpm -C apps/web test`, `cargo test --all-features`, desktop build succeeds

---

## Issue #201: Vulnerable Frontend Transitive Dependencies

### Summary
Transitive dependencies `picomatch@4.0.3` and `devalue@5.6.3` have known ReDoS and prototype pollution vulnerabilities.

### Success Criteria
- ✓ `pnpm audit --audit-level=moderate` reports 0 known vulnerabilities in `picomatch` and `devalue`
- ✓ Build system (vite, @sveltejs/kit, svelte) still functions correctly
- ✓ pnpm-lock.yaml is updated to reflect the new dependency state

### Verification: Did We Build the Fix Right?

**1. Dependency Audit Pass**
- [ ] `pnpm audit --audit-level=moderate` from repo root returns success (exit 0)
- [ ] Specifically: no advisories for `picomatch` or `devalue` CVEs (GHSA-c2c7-rcm5-vvqj, GHSA-3v7f-55p6-f55p, GHSA-cfw5-2vxh-hr84)

**2. Build Verification**
- [ ] `pnpm install` completes without errors
- [ ] `pnpm -C apps/web build` produces valid build artifacts
- [ ] `pnpm -C apps/desktop build` succeeds (desktop packaging includes updated frontend)

**3. pnpm-lock.yaml Consistency**
- [ ] pnpm-lock.yaml contains only patched versions: `picomatch@>=4.0.4`, `devalue@>=5.6.4`
- [ ] No fallback to vulnerable versions in the lock file

**4. Static Analysis**
- [ ] `pnpm -r lint` passes across all packages

### Validation: Did We Solve the Real Problem?

**1. Supply Chain Security**
- [ ] CI/CD pipeline runs without dependency audit failures
- [ ] Security scanning tools (if integrated) no longer flag these CVEs
- [ ] No functional regressions in build or dev toolchain

**2. Dependency Graph Inspection**
- [ ] Run `pnpm list picomatch devalue` and verify pinned versions are ≥ patched
- [ ] Confirm no other transitive paths reintroduce older versions

**3. Release-Readiness Audit**
- [ ] Dependency audit clean at moderate or higher severity threshold
- [ ] Desktop bundle does not include known-vulnerable frontend libraries

### Go/No-Go Evidence for Closing Issue #201
- [ ] `pnpm audit --audit-level=moderate` exit code is 0
- [ ] `pnpm list picomatch devalue` output shows only patched versions
- [ ] `pnpm -C apps/web build` and `pnpm -C apps/desktop build` succeed
- [ ] CI passes (lint, test, build)
- [ ] Git commit updates pnpm-lock.yaml only (no unnecessary changes)

---

## Issue #202: Helm Values Redaction Misses Nested Secrets

### Summary
Default Helm values redaction doesn't recurse into nested objects under sensitive keys (e.g., `auth`, `credentials`).

### Success Criteria
- ✓ Helm values endpoint returns redacted output when `reveal=false`
- ✓ Nested strings under sensitive keys are redacted (not just top-level sensitive keys)
- ✓ Operator can still use the `reveal=true` override when needed for trusted contexts
- ✓ No regression in UI display of Helm release values

### Verification: Did We Build the Fix Right?

**1. Unit Tests (Core Redaction Logic)**
- [ ] Redaction function recurses into nested objects under sensitive keys
- [ ] Redaction function recurses into arrays of objects under sensitive keys
- [ ] Non-sensitive keys and their nested content are left unchanged
- [ ] Mixed sensitive/non-sensitive structures are handled correctly
- [ ] Edge cases tested: null values, empty objects, deep nesting (3+ levels)

**2. Regression Tests**
- [ ] Existing test `redact_sensitive_values_skips_non_string_sensitive` is updated or replaced
- [ ] New test `redact_sensitive_values_redacts_nested_objects_under_sensitive_keys` passes
- [ ] Test cases cover: `auth: { username, password }`, `credentials: { token, user }`, `secret: { nested: { key } }`

**3. Integration Tests**
- [ ] Desktop Tauri command `get_helm_release_values(release, namespace, reveal=false)` returns redacted YAML
- [ ] Nested secrets are not visible in returned value
- [ ] `reveal=true` still returns full unredacted values (for authorized admin flow)

**4. Code Coverage**
- [ ] `crates/engine/src/helm.rs` redaction logic has >85% branch coverage
- [ ] All redaction paths are exercised by tests

**5. Static Analysis**
- [ ] `cargo clippy` and `cargo fmt` pass on `crates/engine`

### Validation: Did We Solve the Real Problem?

**1. Threat Scenario: Helm Chart with Nested Credentials**
- [ ] Create a test Helm release with values:
  ```yaml
  auth:
    username: admin-user
    password: super-secret-password
  credentials:
    apiKey: sk-12345secret
  config:
    port: 8080
  ```
- [ ] Call default values endpoint (reveal=false)
- [ ] Verify: `auth` and `credentials` subtrees are redacted; `config` is visible

**2. UI Display Verification**
- [ ] Desktop app Helm release view shows redacted values correctly
- [ ] Nested objects render without exposing secrets
- [ ] Redacted fields have visual indicator (e.g., `[REDACTED]`, masked placeholder)

**3. Compliance/Audit Context**
- [ ] Support bundle export of Helm values does not expose nested secrets
- [ ] Logs exported with reveal=false do not contain plaintext secrets
- [ ] Authorized admin override (reveal=true) is distinguishable and auditable

**4. Chart Compatibility**
- [ ] Test against real-world Helm charts with common credential structures:
  - PostgreSQL chart (auth.password)
  - Redis chart (auth.password, redis.password)
  - Elasticsearch chart (elasticsearch.security.enabled, auth.enabled)

### Go/No-Go Evidence for Closing Issue #202
- [ ] All unit tests pass; code coverage >85% on redaction logic
- [ ] Threat scenario test passes; nested secrets are redacted in returned YAML
- [ ] Desktop e2e test: fetch Helm release values with reveal=false; verify no nested secrets visible
- [ ] Human spot-check: inspect UI for Helm release with nested credentials; no plaintext secrets visible
- [ ] Existing Helm value displays still work (no regression)
- [ ] CI passes: `cargo test --all-features`, desktop e2e test, Playwright test suite

---

## Execution Order & Dependencies

### Recommended Sequence

**Phase 1: Dependency-Free Work (Parallel Execution)**

1. **Issue #201: Frontend Dependency Remediation** *(lowest risk, highest parallelism)*
   - No blocking dependencies within Telescope codebase
   - Execution: Run `pnpm audit` → update overrides in `package.json` → re-run audit
   - Validation: Pure build/toolchain checks; no logic changes needed
   - Duration: ~30 mins
   - Dependency: None

2. **Issue #202: Helm Redaction Logic** *(medium complexity, well-scoped)*
   - Execution: Update `crates/engine/src/helm.rs` redaction walker to recurse
   - Validation: Unit tests are comprehensive; can be tested in isolation
   - Does not require desktop packaging to validate
   - Duration: ~2 hours
   - Dependency: None (but should be done before desktop e2e if redaction is UI-critical)

**Phase 2: Desktop/Tauri Coupling**

3. **Issue #200: Exec Command Audit Redaction** *(higher complexity, desktop integration)*
   - Execution: Add redaction function to `crates/engine/src/audit.rs` → update `apps/desktop/src-tauri/src/main.rs` to call it
   - Validation: Unit tests in engine, integration test in desktop (exec terminal scenario)
   - Requires desktop app rebuild to validate end-to-end
   - Duration: ~2.5 hours
   - Dependency: Should wait until #201 completes (clean CI state)

**Phase 3: Integration Validation (Serial)**

4. **Cross-Issue E2E Test Suite**
   - Execute in order: #201 audit pass → #202 Helm UI display → #200 exec log capture
   - Verify no regressions across the three fixes
   - Duration: ~1 hour
   - Dependency: All three issues must have verification tests passing

### Dependency Graph

```
Issue #201 (deps)
  ↓ (no blocking deps, but want clean CI)
Issue #202 (Helm logic) ← can run in parallel with #201
  ↓
Issue #200 (exec redaction) ← waits for clean CI from #201/#202
  ↓
Integration E2E test
  ↓
Sign-off
```

### Why This Order?

| Reason | Implication |
|--------|-------------|
| **#201 first:** Pure dependency/build fix; no code logic changes; fast CI pass | Establishes clean baseline for subsequent tests |
| **#202 parallel with #201:** Isolated to engine crate; logic tests can pass independently | Maximizes parallelism without blocking |
| **#200 after #201/#202:** Requires desktop rebuild; benefits from clean dependency state | Ensures desktop integration test doesn't fail due to unrelated CI issues |
| **E2E last:** Cross-checks all three fixes together in real desktop workflow | Catches regressions and integration issues |

---

## Test Environment & Tooling

### Verification Testing
- **Unit tests:** `cargo test --workspace --all-features` (Rust) + `pnpm -r test` (Frontend)
- **Linting:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Dependency audit:** `pnpm audit --audit-level=moderate`
- **Build:** `pnpm -C apps/web build` + `pnpm -C apps/desktop build`

### Validation Testing
- **Desktop integration:** `pnpm -C apps/desktop dev` + manual smoke test
- **E2E scenarios:** Playwright tests in `apps/web` + custom E2E harness for exec/Helm workflows
- **Security scanning:** Inspect audit log, Helm values returned, build artifacts for exposed secrets

### CI Integration Points
- All tests must pass in `.github/workflows/ci.yml` (Rust + frontend checks + desktop build)
- Desktop build only runs on Windows/macOS; Linux CI validates engine/core
- Dependency audit runs as part of frontend lint pass

---

## Key Tradeoffs & Assumptions

| Tradeoff | Decision | Rationale |
|----------|----------|-----------|
| **Redaction completeness vs. logging utility** | Redact full command details by default; preserve program name + timestamp | Prevents secret exposure; still enables basic forensics |
| **Nested redaction granularity** | Redact entire subtree under sensitive key; no partial reveal | Simpler, safer; no risk of selective exposure |
| **Dependency override vs. upstream upgrade** | Prefer override if upstream lag; fall back to upgrade if override fails CI | Faster iteration; less risk of unintended side effects |
| **Desktop validation scope** | Include e2e; exclude fuzzing or adversarial input generation | Balances thoroughness with timeline; fuzzing deferred to future hardening |
| **Audit log backward compatibility** | Do not attempt to scrub existing logs; only future entries are redacted | Scope containment; existing logs remain on user's system (user responsibility) |

---

## Sign-Off Checklist

For each issue, lead must verify:

- [ ] **Issue #200:**
  - [ ] Verification: All unit + integration tests passing; CI green
  - [ ] Validation: Threat scenario test passed; audit log confirmed clean
  - [ ] No regressions in other exec/logging workflows

- [ ] **Issue #201:**
  - [ ] Verification: `pnpm audit --audit-level=moderate` exits 0; build succeeds
  - [ ] Validation: No functional regressions in dev toolchain
  - [ ] pnpm-lock.yaml reflects only patched versions

- [ ] **Issue #202:**
  - [ ] Verification: Redaction unit tests >85% coverage; all tests passing
  - [ ] Validation: Threat scenario test passed; UI shows redacted nested values
  - [ ] No regressions in existing Helm release views

- [ ] **Cross-Issue:**
  - [ ] E2E integration test suite passes
  - [ ] Desktop app starts cleanly; no startup errors
  - [ ] All CI workflows pass (Rust, frontend, desktop)
  - [ ] Coordinator ready to create release tag (if release is planned)


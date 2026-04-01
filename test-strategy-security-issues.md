# Test Strategy for Security Issues #200, #201, #202

**Created by:** Kane (Tester)  
**Date:** 2025-01-06  
**Purpose:** Comprehensive test strategy for validating fixes to exec audit logging, dependency vulnerabilities, and Helm values redaction

---

## Issue #200: Exec Audit Log Stores Full Secret-Bearing Container Commands

### Problem Summary
Container exec commands may contain secrets (passwords, tokens, API keys) in command arguments. Currently, `apps/desktop/src-tauri/src/main.rs:exec_command()` logs the full command verbatim via `command.join(" ")` into audit entries, exposing secrets in audit logs.

### 1. Verification Tests (Proving Fix Implementation)

**Unit Tests** (`crates/engine/src/audit.rs`)
- **Test:** `redact_exec_command_redacts_password_args()`
  - Input: `vec!["mysql", "-u", "root", "-psecret123", "mydb"]`
  - Expected: `["mysql", "-u", "root", "-p●●●●●●●●", "mydb"]`
  - Framework: `cargo test --package telescope-engine --lib audit`
  - Validates: Redaction logic correctly identifies and masks password flags

- **Test:** `redact_exec_command_redacts_env_assignments()`
  - Input: `vec!["sh", "-c", "export TOKEN=abc123 && curl api"]`
  - Expected: `["sh", "-c", "export TOKEN=●●●●●●●● && curl api"]`
  - Validates: Environment variable assignments with sensitive names are redacted

- **Test:** `redact_exec_command_handles_equals_syntax()`
  - Input: `vec!["python", "script.py", "--api-key=sk-1234567890"]`
  - Expected: `["python", "script.py", "--api-key=●●●●●●●●"]`
  - Validates: `--flag=value` syntax redaction

- **Test:** `redact_exec_command_preserves_safe_commands()`
  - Input: `vec!["ls", "-la", "/app"]`
  - Expected: `["ls", "-la", "/app"]` (unchanged)
  - Validates: Non-sensitive commands remain unmodified

- **Test:** `redact_exec_command_handles_edge_cases()`
  - Empty command vector
  - Commands with only whitespace
  - Commands with special characters in safe contexts
  - Validates: Robust error handling

**SWE Principles Illustrated:**
- **Input boundary testing** (empty, whitespace, special chars)
- **Positive and negative cases** (secrets vs. safe commands)
- **Pattern matching correctness** (multiple syntax variations)

### 2. Validation Tests (Proving Security Problem Solved)

**Integration Test** (`crates/engine/tests/exec_audit_redaction.rs` - NEW)
- **Test:** `exec_with_password_flag_redacts_in_audit_log()`
  - Setup: Create temp audit log file, initialize engine state
  - Execute: Call `exec_command()` with `mysql -pSECRET123`
  - Verify: Read audit log JSONL, parse entries, assert detail field contains `●●●●●●●●` not `SECRET123`
  - Teardown: Clean up temp files
  - Framework: `cargo test --package telescope-engine --test exec_audit_redaction`
  - Validates: End-to-end flow from command execution through audit log write

- **Test:** `exec_with_env_var_secrets_redacts_in_audit_log()`
  - Execute: `sh -c "export API_KEY=abc123 && app"`
  - Verify: Audit log contains redacted env var value
  - Validates: Shell command string parsing and redaction

**Desktop E2E Test** (`apps/web/tests-e2e/exec-audit-security.spec.ts` - NEW)
- **Test:** "exec command with password redacts in audit viewer"
  - Preconditions: Desktop app running, connected to test cluster
  - Steps:
    1. Navigate to a pod with exec capability
    2. Execute: `mysql -u root -pTOPSECRET mydb`
    3. Navigate to audit log viewer (if exists) or check audit file directly
  - Assertions:
    - Command appears in audit log
    - Password value is redacted (`●●●●●●●●`)
    - Timestamp, actor, resource info are correct
  - Framework: `pnpm -C apps/web e2e`
  - Validates: Complete user-facing flow with real UI interaction

**SWE Principles Illustrated:**
- **End-to-end validation** (desktop command → Rust handler → audit file → UI)
- **Real-world scenario testing** (actual MySQL password syntax)
- **User acceptance testing** (verifying from user perspective)

### 3. Negative Tests / Regression / Edge Cases

**Regression Tests** (`crates/engine/src/audit.rs`)
- **Test:** `redaction_does_not_break_control_char_sanitization()`
  - Input: Command with secrets AND control characters
  - Validates: Both sanitization layers work together correctly

- **Test:** `redaction_preserves_audit_log_format()`
  - Validates: JSONL structure remains valid after redaction
  - Parse redacted entry as JSON, verify all fields present

**Edge Case Tests** (`crates/engine/src/audit.rs`)
- **Test:** `redact_handles_very_long_commands()`
  - Input: Command with 1000+ arguments
  - Validates: Performance and correctness at scale

- **Test:** `redact_handles_unicode_in_secrets()`
  - Input: `--password=パスワード123`
  - Expected: Redacted properly regardless of character encoding

- **Test:** `redact_handles_multiline_shell_scripts()`
  - Input: `sh -c "line1\nAPI_KEY=secret\nline3"`
  - Validates: Newlines don't break pattern matching

- **Test:** `redact_does_not_match_safe_patterns()`
  - Input: `cat password.txt` (filename, not a password value)
  - Input: `echo "The password is..."` (literal string, not assignment)
  - Expected: NO redaction (avoid false positives)
  - Validates: Precision - only redact actual secret values

**Performance Test** (`crates/engine/benches/audit_redaction.rs` - NEW)
- Benchmark redaction overhead for typical exec commands
- Ensure < 1ms overhead per command
- Validates: Security doesn't significantly impact UX

**SWE Principles Illustrated:**
- **Regression prevention** (existing features still work)
- **False positive minimization** (precision in pattern matching)
- **Performance validation** (security doesn't degrade UX)
- **Internationalization** (Unicode handling)
- **Scale testing** (large inputs)

### 4. Recommended Framework/Commands

- **Primary:** `cargo test --package telescope-engine --lib audit` (unit tests)
- **Integration:** `cargo test --package telescope-engine --test exec_audit_redaction`
- **E2E:** `pnpm -C apps/web e2e -- exec-audit-security.spec.ts`
- **Benchmarks:** `cargo bench --package telescope-engine --bench audit_redaction`
- **Coverage:** `cargo tarpaulin --package telescope-engine --lib audit`

### 5. Core SWE Validation & Verification Principles

1. **Defense in Depth:** Multiple test layers (unit → integration → E2E)
2. **Verification vs. Validation:** 
   - Verification = "built it right" (unit tests prove algorithm correctness)
   - Validation = "built the right thing" (E2E proves user security problem solved)
3. **Boundary Value Analysis:** Test empty, single, many arguments
4. **Equivalence Partitioning:** Group inputs by redaction behavior (secrets vs. safe)
5. **Negative Testing:** Verify doesn't over-redact safe content
6. **Regression Testing:** Ensure control char sanitization still works
7. **Performance Testing:** Security overhead is acceptable
8. **Test Oracle:** Known secret patterns → expected redacted output

---

## Issue #201: Frontend Transitive Dependency Vulnerabilities Flagged by pnpm audit

### Problem Summary
`pnpm audit` reports vulnerabilities in transitive dependencies. These may include XSS, prototype pollution, or other client-side attack vectors. The fix typically involves dependency updates, patches, or overrides.

### 1. Verification Tests (Proving Fix Implementation)

**Dependency Audit Verification** (Shell script or CI check)
- **Test:** `dependency_audit_passes_without_high_severity()`
  - Command: `pnpm audit --audit-level=high --production`
  - Expected: Exit code 0 (no high/critical vulnerabilities)
  - Framework: Shell script in `.github/workflows/ci.yml` or `scripts/verify-dependencies.sh`
  - Validates: No high-severity vulnerabilities in production dependencies

- **Test:** `lockfile_is_consistent_after_update()`
  - Command: `pnpm install --frozen-lockfile`
  - Expected: Exit code 0 (no lockfile drift)
  - Validates: `pnpm-lock.yaml` correctly reflects resolved vulnerabilities

- **Test:** `overrides_apply_correctly()`
  - If using `pnpm.overrides` in `package.json`
  - Verify: `pnpm list <vulnerable-package>` shows overridden version
  - Validates: Dependency resolution uses patched versions

**SWE Principles Illustrated:**
- **Automated security scanning** (pnpm audit integration)
- **Declarative verification** (exit codes, lockfile consistency)
- **Supply chain validation** (dependency graph correctness)

### 2. Validation Tests (Proving Security Problem Solved)

**Vulnerability-Specific Tests** (`apps/web/src/lib/*.test.ts` - depending on vulnerability)

Example scenario: If vulnerability is in a UI component library (e.g., XSS in markdown renderer):

- **Test:** `sanitizes_user_input_preventing_xss()`
  - Setup: Component that renders potentially vulnerable content
  - Input: `<script>alert('XSS')</script>` in user-controlled field
  - Expected: Script tags escaped or removed in DOM
  - Framework: `pnpm -C apps/web test` (Vitest)
  - Validates: Actual XSS attack is prevented

Example scenario: Prototype pollution in object utilities:

- **Test:** `prevents_prototype_pollution_via_object_merge()`
  - Input: Malicious object with `__proto__` key
  - Expected: Prototype chain remains unmodified
  - Validates: Real attack vector is closed

**E2E Security Tests** (`apps/web/tests-e2e/security-vuln-validation.spec.ts` - NEW)
- **Test:** "vulnerable component does not execute injected script"
  - If vulnerability was in a chart/graph library
  - Steps: Navigate to page, inject malicious payload via UI
  - Assertions: No alert dialog, no console errors, payload rendered safely
  - Framework: `pnpm -C apps/web e2e`

**Manual Validation Checklist** (Document in PR/issue)
- For each CVE fixed, document:
  - CVE ID and CVSS score
  - Attack vector (network, local, user interaction required?)
  - Specific component/route affected in Telescope
  - Reproduction steps (if applicable)
  - Verification that attack no longer works

**SWE Principles Illustrated:**
- **Threat modeling** (understanding attack vector)
- **Attack simulation** (attempting exploit post-fix)
- **CVE-driven validation** (mapping specific vulnerabilities to tests)

### 3. Negative Tests / Regression / Edge Cases

**Regression Tests** (`apps/web/src/lib/*.test.ts`, `apps/web/tests-e2e/*.spec.ts`)
- **Test:** `dependency_update_does_not_break_existing_features()`
  - Run full existing test suite: `pnpm -C apps/web test`
  - Run full E2E suite: `pnpm -C apps/web e2e`
  - Expected: All existing tests pass
  - Validates: Dependency updates didn't introduce breaking changes

- **Test:** `ui_rendering_remains_correct_after_library_update()`
  - Specific tests for components using updated libraries
  - Visual regression testing (if Playwright screenshots exist)
  - Validates: UI appearance unchanged

**Edge Cases for Security Patches**
- **Test:** `patched_library_handles_edge_case_inputs()`
  - If patch modifies input validation, test boundary values
  - Empty strings, null, undefined, very long strings
  - Validates: Patch is robust, not just a quick fix

**Dependency Graph Tests**
- **Test:** `no_duplicate_vulnerable_versions_in_tree()`
  - Command: `pnpm why <package>` for each updated package
  - Validates: No shadowed/duplicate versions of fixed packages

**SWE Principles Illustrated:**
- **Non-regression testing** (existing features still work)
- **Whole-system impact analysis** (dependency updates affect entire app)
- **Dependency resolution verification** (no duplicates or conflicts)

### 4. Recommended Framework/Commands

- **Primary:** `pnpm audit --audit-level=high --production` (CI-enforced)
- **Unit Tests:** `pnpm -C apps/web test` (Vitest for specific vulnerability scenarios)
- **E2E:** `pnpm -C apps/web e2e` (Playwright for UI attack simulation)
- **Regression:** Full existing test suite (`pnpm -r --if-present test`)
- **Dependency Tree:** `pnpm list --depth=10 | grep <vulnerable-pkg>`
- **Lockfile Validation:** `pnpm install --frozen-lockfile` in CI

### 5. Core SWE Validation & Verification Principles

1. **Automated Security Gates:** CI fails on high-severity vulnerabilities
2. **CVE-to-Test Mapping:** Each fixed CVE has corresponding validation
3. **Attack Simulation:** Test actual exploit attempts post-fix
4. **Regression Coverage:** Full test suite guards against breakage
5. **Dependency Hygiene:** Lockfile consistency, no duplicates
6. **Defense Verification:** Test both exploit prevention AND feature preservation
7. **Layered Testing:** Audit tool (detection) + unit tests (specific vectors) + E2E (real UI)
8. **Continuous Monitoring:** Scheduled `pnpm audit` runs detect new vulnerabilities

---

## Issue #202: Helm Values Redaction Misses Nested Secrets Under auth/credentials Maps

### Problem Summary
`crates/engine/src/helm.rs::redact_sensitive_values()` only matches top-level and one-level-nested keys against `SENSITIVE_KEYS`. Deeply nested secrets like `spec.auth.credentials.password` or `database.auth.credentials.token` are not redacted because the current recursion only checks immediate key names, not full key paths.

### 1. Verification Tests (Proving Fix Implementation)

**Unit Tests** (`crates/engine/src/helm.rs`)
- **Test:** `redact_sensitive_values_handles_deeply_nested_auth()`
  - Input JSON:
    ```json
    {
      "spec": {
        "auth": {
          "credentials": {
            "password": "secret123",
            "username": "admin"
          }
        }
      }
    }
    ```
  - Expected: `password` value redacted to `●●●●●●●●`, `username` unchanged
  - Framework: `cargo test --package telescope-engine --lib helm`
  - Validates: Recursion depth correctly handles 3+ levels

- **Test:** `redact_sensitive_values_handles_nested_credentials_map()`
  - Input:
    ```json
    {
      "database": {
        "credentials": {
          "connectionString": "postgresql://user:pass@host/db",
          "apiKey": "key-12345"
        }
      }
    }
    ```
  - Expected: Both `connectionString` and `apiKey` redacted
  - Validates: All sensitive keys in `credentials` map are caught

- **Test:** `redact_sensitive_values_handles_arrays_of_credentials()`
  - Input:
    ```json
    {
      "services": [
        {"auth": {"token": "abc123"}},
        {"auth": {"password": "xyz789"}}
      ]
    }
    ```
  - Expected: Both secrets redacted
  - Validates: Array iteration + nested object redaction

- **Test:** `redact_sensitive_values_preserves_non_string_secrets()`
  - Input: `{"config": {"secret": {"enabled": true, "value": "actual-secret"}}}`
  - Expected: `enabled` (boolean) unchanged, `value` redacted
  - Validates: Type checking still works at all depths

- **Test:** `redact_sensitive_values_handles_empty_nested_maps()`
  - Input: `{"auth": {}, "credentials": null}`
  - Expected: No panic, graceful handling
  - Validates: Edge case robustness

**SWE Principles Illustrated:**
- **Recursive algorithm correctness** (depth handling)
- **Data structure traversal** (objects, arrays, primitives)
- **Type-aware processing** (string vs. non-string)
- **Edge case handling** (empty, null, missing fields)

### 2. Validation Tests (Proving Security Problem Solved)

**Integration Test** (`crates/engine/tests/helm_values_redaction.rs` - NEW)
- **Test:** `helm_release_values_redact_deeply_nested_secrets()`
  - Setup: Create mock Helm release secret with realistic nested values
  - YAML structure mimicking real-world charts (e.g., Bitnami PostgreSQL):
    ```yaml
    auth:
      postgresPassword: "SENSITIVE1"
      replicationPassword: "SENSITIVE2"
      credentials:
        primary:
          password: "SENSITIVE3"
        readReplica:
          password: "SENSITIVE4"
    ```
  - Execute: `get_release_values()` → `redact_sensitive_values()`
  - Verify: All four password values are `●●●●●●●●`
  - Framework: `cargo test --package telescope-engine --test helm_values_redaction`
  - Validates: Real Helm chart structure is correctly redacted

**Desktop E2E Test** (`apps/web/tests-e2e/helm-values-redaction.spec.ts` - NEW or extend existing `helm-uninstall.spec.ts`)
- **Test:** "Helm release values view redacts nested credentials"
  - Preconditions: Desktop app, cluster with Helm release containing nested secrets
  - Steps:
    1. Navigate to Helm releases view
    2. Select a release with nested auth/credentials
    3. View values (without "reveal" mode)
  - Assertions:
    - Deeply nested password fields show `●●●●●●●●`
    - Non-sensitive nested fields (e.g., `username`, `host`) are visible
    - "Reveal" button (if exists) can show actual values when clicked
  - Framework: `pnpm -C apps/web e2e`
  - Validates: User-facing security from UI perspective

**Manual Testing Checklist** (Document in test plan)
- Test against real Helm charts:
  - Bitnami PostgreSQL (`auth.postgresPassword`)
  - Bitnami Redis (`auth.password`)
  - OAuth2 Proxy (`config.clientSecret`)
  - Custom charts with `credentials` maps
- Verify: All expected secrets redacted, no false positives

**SWE Principles Illustrated:**
- **Real-world data testing** (actual Helm chart structures)
- **End-to-end flow validation** (K8s secret → Rust parser → UI display)
- **User acceptance testing** (manual verification against real charts)
- **Scenario-based testing** (common Helm chart patterns)

### 3. Negative Tests / Regression / Edge Cases

**Regression Tests** (`crates/engine/src/helm.rs`)
- **Test:** `redaction_still_works_for_top_level_secrets()`
  - Input: `{"password": "secret", "apiKey": "key"}`
  - Expected: Both redacted (existing behavior preserved)
  - Validates: Fix doesn't break current redaction

- **Test:** `redaction_preserves_reveal_flag_behavior()`
  - Verify: `reveal: true` in desktop command still bypasses redaction
  - Validates: Feature flag logic unaffected

**Edge Cases** (`crates/engine/src/helm.rs`)
- **Test:** `redact_handles_circular_references_gracefully()`
  - Note: Helm values are typically acyclic JSON/YAML, but test that parser doesn't infinitely loop on malformed input
  - Use `serde_json` limits or depth counter

- **Test:** `redact_handles_very_deeply_nested_structures()`
  - Input: 10+ levels of nesting with secret at bottom
  - Expected: Redacted correctly, no stack overflow
  - Validates: Recursion depth limit (if any) is adequate

- **Test:** `redact_does_not_match_non_secret_credential_fields()`
  - Input:
    ```json
    {
      "credentials": {
        "username": "admin",
        "host": "db.example.com",
        "port": 5432,
        "password": "secret"
      }
    }
    ```
  - Expected: Only `password` redacted, others visible
  - Validates: Precision - avoid false positives

- **Test:** `redact_handles_mixed_case_sensitive_keys()`
  - Input: `{"AUTH": {"PASSWORD": "secret"}}`
  - Expected: Case-insensitive matching still works
  - Validates: Case normalization at all depths

**Performance Test** (`crates/engine/benches/helm_redaction.rs` - NEW)
- Benchmark redaction on large Helm values (1000+ key-value pairs)
- Ensure < 50ms for typical release values
- Validates: Deeper recursion doesn't cause UX lag

**SWE Principles Illustrated:**
- **Regression prevention** (top-level redaction still works)
- **Recursion safety** (stack overflow prevention)
- **Performance bounds** (large values don't hang UI)
- **Precision testing** (false positive avoidance)
- **Case-insensitive matching** (robustness)

### 4. Recommended Framework/Commands

- **Primary:** `cargo test --package telescope-engine --lib helm` (unit tests)
- **Integration:** `cargo test --package telescope-engine --test helm_values_redaction`
- **E2E:** `pnpm -C apps/web e2e -- helm-values-redaction.spec.ts`
- **Benchmarks:** `cargo bench --package telescope-engine --bench helm_redaction`
- **Watcher Test:** Extend `crates/engine/tests/watchers_smoke.rs::watcher_redacts_pod_env_values_in_store` to validate Helm secret redaction in store

### 5. Core SWE Validation & Verification Principles

1. **Algorithm Correctness:** Unit tests prove recursive traversal works
2. **Real-World Data:** Integration tests use actual Helm chart structures
3. **User-Facing Validation:** E2E tests confirm UI displays redacted values
4. **Regression Safety:** Existing top-level redaction still works
5. **Performance Validation:** Deep recursion doesn't degrade UX
6. **Edge Case Robustness:** Handles empty, null, very deep, malformed inputs
7. **Precision:** Only redacts actual secrets, not safe fields
8. **Defense in Depth:** Multiple test layers catch different failure modes

---

## Cross-Cutting Test Infrastructure Recommendations

### CI Integration
- Add security-specific CI job: `.github/workflows/security.yml`
  - Run `pnpm audit --audit-level=high` (fail on high/critical)
  - Run `cargo test` with `--features security-tests` (if gated)
  - Run E2E security tests: `pnpm -C apps/web e2e -- security-*.spec.ts`

### Test Data Management
- Create `crates/engine/tests/fixtures/` for:
  - Sample Helm values with nested secrets (`helm-values-nested.yaml`)
  - Exec command samples with various secret patterns (`exec-commands.json`)
  - Mock audit log entries (`audit-samples.jsonl`)

### Coverage Tracking
- Set coverage targets:
  - `crates/engine/src/audit.rs`: 90%+ line coverage
  - `crates/engine/src/helm.rs::redact_sensitive_values()`: 95%+ branch coverage
  - Desktop commands related to security: 80%+ coverage

### Documentation
- Add to `docs/testing.md` (if exists):
  - Security testing strategy overview
  - How to run security-specific tests
  - How to add tests for new security features
  - Incident response: tests to write when vulnerabilities are discovered

### Continuous Security Testing
- Scheduled CI runs:
  - Daily `pnpm audit` runs, alert on new vulnerabilities
  - Weekly integration test runs against live test cluster
  - Monthly review of security test coverage

---

## Summary Matrix

| Issue | Primary Framework | Key Test Types | Success Criteria |
|-------|-------------------|----------------|------------------|
| #200 Exec Audit | `cargo test` (engine) | Unit, Integration, E2E | No secrets in audit logs; all exec patterns covered |
| #201 Dependencies | `pnpm audit`, Vitest, Playwright | Audit, Unit (vulnerability-specific), E2E, Regression | `pnpm audit` passes; no high-severity CVEs; all existing tests pass |
| #202 Helm Redaction | `cargo test` (engine), Playwright | Unit, Integration, E2E | Deeply nested secrets redacted; real Helm charts validated |

---

## Test Development Workflow

1. **Write failing test first** (TDD approach)
   - For #200: `redact_exec_command_redacts_password_args()` fails before fix
   - For #202: `redact_sensitive_values_handles_deeply_nested_auth()` fails before fix

2. **Implement fix**
   - Minimal code to make test pass

3. **Expand test coverage**
   - Add edge cases, negative tests, performance tests

4. **Run full regression suite**
   - Ensure no existing tests broken

5. **Add integration/E2E tests**
   - Validate end-to-end flow

6. **Document test rationale**
   - Link tests to specific CVEs, attack vectors, or security requirements

7. **Review and merge**
   - All tests passing, coverage targets met

---

## Risk Mitigation

- **Risk:** Tests pass but real vulnerability remains
  - **Mitigation:** Manual security review, penetration testing, external audit
  
- **Risk:** Tests become flaky due to async timing or environment dependencies
  - **Mitigation:** Use deterministic fixtures, avoid network dependencies in unit tests, retry logic in E2E

- **Risk:** Performance tests fail in CI due to resource constraints
  - **Mitigation:** Set realistic thresholds, run performance tests in dedicated environment

- **Risk:** Dependency updates introduce breaking changes
  - **Mitigation:** Comprehensive regression suite, version pinning, gradual rollout

---

**End of Test Strategy Document**

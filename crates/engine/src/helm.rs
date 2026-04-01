//! Helm release management via Kubernetes Secrets.
//!
//! Helm stores releases as Secrets with type `helm.sh/release.v1`.
//! The secret data is base64-encoded, gzipped JSON containing release metadata.
//! This module parses those secrets directly — no `helm` binary required.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use flate2::read::GzDecoder;
use k8s_openapi::api::core::v1::Secret;
use kube::api::ListParams;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmRelease {
    pub name: String,
    pub namespace: String,
    pub chart: String,
    pub app_version: String,
    pub revision: i32,
    pub status: String,
    pub updated: String,
}

const MAX_HELM_RELEASE_JSON_BYTES: usize = 8 * 1024 * 1024;
const HELM_BINARY_PATH_ENV: &str = "TELESCOPE_HELM_PATH";

#[cfg(target_os = "windows")]
const TRUSTED_HELM_BINARY_PATHS: &[&str] = &[
    r"C:\Program Files\Helm\helm.exe",
    r"C:\Program Files (x86)\Helm\helm.exe",
    r"C:\ProgramData\chocolatey\bin\helm.exe",
];

#[cfg(not(target_os = "windows"))]
const TRUSTED_HELM_BINARY_PATHS: &[&str] = &[
    "/usr/local/bin/helm",
    "/opt/homebrew/bin/helm",
    "/usr/bin/helm",
    "/snap/bin/helm",
];

fn decode_release_json(data: &[u8]) -> crate::Result<serde_json::Value> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)
        .map_err(|error| {
            crate::EngineError::Other(format!("Failed to decode Helm release payload: {error}"))
        })?;

    let mut limited_reader =
        GzDecoder::new(&decoded[..]).take((MAX_HELM_RELEASE_JSON_BYTES as u64) + 1);
    let mut json_bytes = Vec::new();
    limited_reader
        .read_to_end(&mut json_bytes)
        .map_err(|error| {
            crate::EngineError::Other(format!(
                "Failed to decompress Helm release payload: {error}"
            ))
        })?;

    if json_bytes.len() > MAX_HELM_RELEASE_JSON_BYTES {
        return Err(crate::EngineError::Other(format!(
            "Helm release payload exceeds {MAX_HELM_RELEASE_JSON_BYTES} bytes after decompression"
        )));
    }

    serde_json::from_slice(&json_bytes).map_err(|error| {
        crate::EngineError::Other(format!("Failed to parse Helm release payload: {error}"))
    })
}

fn validate_helm_binary_path(path: PathBuf, source: &str) -> crate::Result<PathBuf> {
    if !path.is_absolute() {
        return Err(crate::EngineError::Other(format!(
            "Helm binary from {source} must be configured with an absolute path"
        )));
    }

    let canonical = path.canonicalize().map_err(|error| {
        crate::EngineError::Other(format!(
            "Failed to resolve Helm binary from {source}: {error}"
        ))
    })?;
    let metadata = std::fs::metadata(&canonical).map_err(|error| {
        crate::EngineError::Other(format!(
            "Failed to inspect Helm binary at {}: {error}",
            canonical.display()
        ))
    })?;

    if !metadata.is_file() {
        return Err(crate::EngineError::Other(format!(
            "Configured Helm binary at {} is not a file",
            canonical.display()
        )));
    }

    Ok(canonical)
}

fn resolve_helm_binary_path_with<I>(
    configured_path: Option<PathBuf>,
    trusted_paths: I,
) -> crate::Result<PathBuf>
where
    I: IntoIterator<Item = PathBuf>,
{
    if let Some(path) = configured_path {
        return validate_helm_binary_path(
            path,
            &format!("environment variable {HELM_BINARY_PATH_ENV}"),
        );
    }

    for candidate in trusted_paths {
        if !candidate.exists() {
            continue;
        }

        if let Ok(resolved) = validate_helm_binary_path(candidate, "trusted installation location")
        {
            return Ok(resolved);
        }
    }

    Err(crate::EngineError::Other(format!(
        "Unable to locate Helm in trusted installation paths. Set {HELM_BINARY_PATH_ENV} to an absolute path to a trusted Helm binary."
    )))
}

fn resolve_helm_binary_path() -> crate::Result<PathBuf> {
    resolve_helm_binary_path_with(
        std::env::var_os(HELM_BINARY_PATH_ENV).map(PathBuf::from),
        TRUSTED_HELM_BINARY_PATHS.iter().map(PathBuf::from),
    )
}

/// List all Helm releases across all namespaces (or a specific namespace).
///
/// Only the latest revision of each release is returned.
pub async fn list_releases(
    client: &Client,
    namespace: Option<&str>,
) -> crate::Result<Vec<HelmRelease>> {
    let secrets_api: Api<Secret> = match namespace {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };

    let params = ListParams::default().labels("owner=helm");
    let secrets = secrets_api.list(&params).await?;

    let mut releases: HashMap<String, HelmRelease> = HashMap::new();

    for secret in &secrets.items {
        if secret.type_.as_deref() != Some("helm.sh/release.v1") {
            continue;
        }

        let ns = secret.metadata.namespace.as_deref().unwrap_or("default");

        if let Some(data) = &secret.data {
            if let Some(release_data) = data.get("release") {
                if let Ok(release) = parse_helm_release(&release_data.0) {
                    let key = format!("{}/{}", ns, release.name);
                    let is_newer = releases
                        .get(&key)
                        .is_none_or(|existing| existing.revision < release.revision);
                    if is_newer {
                        releases.insert(
                            key,
                            HelmRelease {
                                namespace: ns.to_string(),
                                ..release
                            },
                        );
                    }
                }
            }
        }
    }

    let mut result: Vec<HelmRelease> = releases.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

/// Get all revisions of a specific Helm release, sorted by revision number.
pub async fn get_release_history(
    client: &Client,
    namespace: &str,
    name: &str,
) -> crate::Result<Vec<HelmRelease>> {
    let secrets_api: Api<Secret> = Api::namespaced(client.clone(), namespace);

    let params = ListParams::default().labels("owner=helm");
    let secrets = secrets_api.list(&params).await?;

    let mut revisions: Vec<HelmRelease> = Vec::new();

    for secret in &secrets.items {
        if secret.type_.as_deref() != Some("helm.sh/release.v1") {
            continue;
        }

        // Check secret name matches sh.helm.release.v1.<name>.v*
        let secret_name = secret.metadata.name.as_deref().unwrap_or("");
        let prefix = format!("sh.helm.release.v1.{}.", name);
        if !secret_name.starts_with(&prefix) {
            continue;
        }

        if let Some(data) = &secret.data {
            if let Some(release_data) = data.get("release") {
                if let Ok(release) = parse_helm_release(&release_data.0) {
                    if release.name == name {
                        revisions.push(HelmRelease {
                            namespace: namespace.to_string(),
                            ..release
                        });
                    }
                }
            }
        }
    }

    revisions.sort_by_key(|r| r.revision);
    Ok(revisions)
}

/// Parse a Helm release from the secret's `release` field.
///
/// The data flow is: K8s Secret (base64-decoded by kube-rs into ByteString)
/// -> inner base64 decode -> gzip decompress -> JSON.
fn parse_helm_release(data: &[u8]) -> crate::Result<HelmRelease> {
    let release = decode_release_json(data)?;

    Ok(HelmRelease {
        name: release["name"].as_str().unwrap_or("").to_string(),
        namespace: String::new(), // filled by caller
        chart: format!(
            "{}-{}",
            release["chart"]["metadata"]["name"].as_str().unwrap_or(""),
            release["chart"]["metadata"]["version"]
                .as_str()
                .unwrap_or("")
        ),
        app_version: release["chart"]["metadata"]["appVersion"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        revision: release["version"].as_i64().unwrap_or(0) as i32,
        status: release["info"]["status"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
        updated: release["info"]["last_deployed"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    })
}

/// Extract the user-supplied values from a Helm release secret.
///
/// Returns the values as a YAML string. If no config values exist,
/// returns an empty YAML document.
pub fn extract_values_from_release(data: &[u8]) -> crate::Result<String> {
    let release = decode_release_json(data)?;
    let config = &release["config"];
    if config.is_null() || config.as_object().is_some_and(|o| o.is_empty()) {
        return Ok("# No custom values configured\n".to_string());
    }
    serde_yaml::to_string(config).map_err(|error| {
        crate::EngineError::Other(format!("Failed to serialize Helm values to YAML: {error}"))
    })
}

/// Get the user-supplied values for the latest revision of a Helm release.
pub async fn get_release_values(
    client: &Client,
    namespace: &str,
    name: &str,
) -> crate::Result<String> {
    let secrets_api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    let params = ListParams::default().labels("owner=helm");
    let secrets = secrets_api.list(&params).await?;

    let mut best: Option<(i32, Vec<u8>)> = None;

    for secret in &secrets.items {
        if secret.type_.as_deref() != Some("helm.sh/release.v1") {
            continue;
        }
        let secret_name = secret.metadata.name.as_deref().unwrap_or("");
        let prefix = format!("sh.helm.release.v1.{}.", name);
        if !secret_name.starts_with(&prefix) {
            continue;
        }
        if let Some(data) = &secret.data {
            if let Some(release_data) = data.get("release") {
                // Parse just to get revision number
                if let Ok(rel) = parse_helm_release(&release_data.0) {
                    if rel.name == name {
                        let is_newer = best.as_ref().is_none_or(|(rev, _)| *rev < rel.revision);
                        if is_newer {
                            best = Some((rel.revision, release_data.0.clone()));
                        }
                    }
                }
            }
        }
    }

    match best {
        Some((_, data)) => extract_values_from_release(&data),
        None => Err(crate::EngineError::Other(format!(
            "Release \"{name}\" not found in namespace \"{namespace}\""
        ))),
    }
}

/// Validate that a string is a valid Kubernetes resource name (RFC 1123 DNS subdomain).
fn validate_k8s_name(name: &str) -> crate::Result<()> {
    // safe: literal regex is infallible
    let re = regex::Regex::new(r"^[a-z0-9]([a-z0-9\-\.]*[a-z0-9])?$").unwrap();
    if name.is_empty() || name.len() > 253 || !re.is_match(name) {
        return Err(crate::EngineError::Other(format!("Invalid name: {}", name)));
    }
    Ok(())
}

fn map_helm_uninstall_error(stderr: &str, stdout: &str) -> String {
    let detail = if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        "Helm uninstall command failed with no output".to_string()
    };
    let detail_lower = detail.to_lowercase();

    if detail_lower.contains("release: not found") || detail_lower.contains("not found") {
        format!("Helm uninstall failed: release not found. {detail}")
    } else if detail_lower.contains("forbidden")
        || detail_lower.contains("unauthorized")
        || detail_lower.contains("permission denied")
    {
        format!("Helm uninstall failed: permission denied. {detail}")
    } else if detail_lower.contains("timed out waiting")
        || detail_lower.contains("deadline exceeded")
        || detail_lower.contains("timeout")
    {
        format!("Helm uninstall failed: operation timed out. {detail}")
    } else {
        format!("Helm uninstall failed: {detail}")
    }
}

/// Keys whose values should be redacted in Helm values display.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "passwd",
    "secret",
    "token",
    "apikey",
    "api_key",
    "apiKey",
    "connectionstring",
    "connection_string",
    "connectionString",
    "private_key",
    "privateKey",
    "private-key",
    "client_secret",
    "clientSecret",
    "client-secret",
    "access_key",
    "accessKey",
    "access-key",
    "secret_key",
    "secretKey",
    "secret-key",
    "credentials",
    "auth",
];

const REDACTED: &str = "●●●●●●●●";

/// Redact sensitive values in a JSON value tree.
///
/// Walks the tree and replaces string values whose key (case-insensitive)
/// contains any of [`SENSITIVE_KEYS`] with a fixed placeholder.
/// If a sensitive key contains an object or array, all nested string values
/// beneath that branch are redacted as well.
pub fn redact_sensitive_values(value: &mut serde_json::Value) {
    redact_sensitive_values_impl(value, false);
}

fn redact_sensitive_values_impl(value: &mut serde_json::Value, force_redact: bool) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map.iter_mut() {
                let key_lower = key.to_lowercase();
                let is_sensitive_key = SENSITIVE_KEYS
                    .iter()
                    .any(|s| key_lower.contains(&s.to_lowercase()));

                if force_redact || is_sensitive_key {
                    if val.is_string() {
                        *val = serde_json::Value::String(REDACTED.to_string());
                    } else {
                        redact_sensitive_values_impl(val, true);
                    }
                } else {
                    redact_sensitive_values_impl(val, false);
                }
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                redact_sensitive_values_impl(item, force_redact);
            }
        }
        _ => {
            if force_redact && value.is_string() {
                *value = serde_json::Value::String(REDACTED.to_string());
            }
        }
    }
}

/// Roll back a Helm release to a specific revision using the `helm` CLI.
pub async fn rollback_release(namespace: &str, name: &str, revision: i32) -> crate::Result<String> {
    validate_k8s_name(namespace)?;
    validate_k8s_name(name)?;
    if revision <= 0 {
        return Err(crate::EngineError::Other(
            "Helm rollback revision must be greater than zero".to_string(),
        ));
    }

    let helm_binary = resolve_helm_binary_path()?;
    let revision_arg = revision.to_string();
    let output = tokio::process::Command::new(&helm_binary)
        .args([
            "rollback",
            name,
            revision_arg.as_str(),
            "--namespace",
            namespace,
        ])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| {
            crate::EngineError::Other(format!(
                "Failed to execute Helm CLI at {}: {e}",
                helm_binary.display()
            ))
        })?;

    if output.status.success() {
        Ok(format!("Rolled back {name} to revision {revision}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            format!("helm exited with status {}", output.status)
        };
        Err(crate::EngineError::Other(format!(
            "Rollback via {} failed: {detail}",
            helm_binary.display()
        )))
    }
}

/// Uninstall a Helm release using the `helm` CLI.
pub async fn helm_uninstall(namespace: &str, name: &str) -> crate::Result<String> {
    validate_k8s_name(namespace)?;
    validate_k8s_name(name)?;

    let helm_binary = resolve_helm_binary_path()?;
    let output = tokio::process::Command::new(&helm_binary)
        .args(["uninstall", name, "-n", namespace])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|error| {
            crate::EngineError::Other(format!(
                "Failed to execute Helm CLI at {}: {error}",
                helm_binary.display()
            ))
        })?;

    if output.status.success() {
        Ok(format!(
            "Uninstalled Helm release {name} from namespace {namespace}"
        ))
    } else {
        let detail = map_helm_uninstall_error(
            String::from_utf8_lossy(&output.stderr).as_ref(),
            String::from_utf8_lossy(&output.stdout).as_ref(),
        );
        Err(crate::EngineError::Other(format!(
            "{detail} (via {})",
            helm_binary.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestTempFile {
        path: PathBuf,
    }

    impl TestTempFile {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "telescope-helm-{name}-{}-{unique}",
                std::process::id()
            ));
            std::fs::write(&path, b"helm-test").unwrap();
            Self { path }
        }
    }

    impl Drop for TestTempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    struct ScopedEnvVar {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl ScopedEnvVar {
        fn set(key: &'static str, value: &std::path::Path) -> Self {
            let previous = std::env::var_os(key);
            // SAFETY: Tests serialize environment mutation with HELM_ENV_LOCK.
            unsafe { std::env::set_var(key, value.as_os_str()) };
            Self { key, previous }
        }
    }

    impl Drop for ScopedEnvVar {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                // SAFETY: Tests serialize environment mutation with HELM_ENV_LOCK.
                unsafe { std::env::set_var(self.key, previous) };
            } else {
                // SAFETY: Tests serialize environment mutation with HELM_ENV_LOCK.
                unsafe { std::env::remove_var(self.key) };
            }
        }
    }

    static HELM_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn helm_env_lock() -> &'static Mutex<()> {
        HELM_ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn create_mock_helm_binary(name: &str, stderr: &str, exit_code: i32) -> TestTempFile {
        let binary = TestTempFile::new(name);

        #[cfg(unix)]
        let script = format!(
            "#!/bin/sh\n[ -n \"$1\" ] >/dev/null 2>&1\necho \"{stderr}\" 1>&2\nexit {exit_code}\n"
        );
        #[cfg(windows)]
        let script = format!("@echo off\r\nif not \"%~1\"==\"\" set _ARG=%~1\r\necho {stderr} 1>&2\r\nexit /b {exit_code}\r\n");

        std::fs::write(&binary.path, script).unwrap();

        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&binary.path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary.path, perms).unwrap();
        }

        binary
    }

    fn make_helm_secret_data(json: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let gzipped = encoder.finish().unwrap();
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &gzipped);
        b64.into_bytes()
    }

    fn make_oversized_helm_secret_data() -> Vec<u8> {
        let json = serde_json::json!({
            "name": "too-large",
            "version": 1,
            "info": {
                "status": "deployed",
                "last_deployed": "2025-01-15T10:30:00Z"
            },
            "chart": {
                "metadata": {
                    "name": "nginx",
                    "version": "1.2.3",
                    "appVersion": "1.25.0"
                }
            },
            "config": {
                "oversized": "x".repeat(MAX_HELM_RELEASE_JSON_BYTES + 1)
            }
        });

        make_helm_secret_data(&json.to_string())
    }

    #[test]
    fn parse_helm_release_decodes_valid_data() {
        let json = r#"{
            "name": "my-release",
            "version": 3,
            "info": {
                "status": "deployed",
                "last_deployed": "2025-01-15T10:30:00Z"
            },
            "chart": {
                "metadata": {
                    "name": "nginx",
                    "version": "1.2.3",
                    "appVersion": "1.25.0"
                }
            }
        }"#;

        let data = make_helm_secret_data(json);
        let release = parse_helm_release(&data).expect("should parse");

        assert_eq!(release.name, "my-release");
        assert_eq!(release.revision, 3);
        assert_eq!(release.status, "deployed");
        assert_eq!(release.chart, "nginx-1.2.3");
        assert_eq!(release.app_version, "1.25.0");
        assert_eq!(release.updated, "2025-01-15T10:30:00Z");
    }

    #[test]
    fn parse_helm_release_handles_missing_fields() {
        let json = r#"{"name": "minimal", "version": 1, "info": {}, "chart": {"metadata": {}}}"#;
        let data = make_helm_secret_data(json);
        let release = parse_helm_release(&data).expect("should parse");

        assert_eq!(release.name, "minimal");
        assert_eq!(release.revision, 1);
        assert_eq!(release.status, "unknown");
        assert_eq!(release.chart, "-");
        assert_eq!(release.app_version, "");
    }

    #[test]
    fn parse_helm_release_rejects_invalid_data() {
        let result = parse_helm_release(b"not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn parse_helm_release_rejects_oversized_decompressed_payload() {
        let err = parse_helm_release(&make_oversized_helm_secret_data()).unwrap_err();
        assert!(err.to_string().contains("exceeds"));
    }

    #[test]
    fn extract_values_returns_yaml() {
        let json = r#"{
            "name": "my-release",
            "version": 1,
            "info": {"status": "deployed"},
            "chart": {"metadata": {"name": "nginx", "version": "1.0.0"}},
            "config": {"replicaCount": 3, "image": {"tag": "latest"}}
        }"#;
        let data = make_helm_secret_data(json);
        let values = extract_values_from_release(&data).expect("should extract values");
        assert!(values.contains("replicaCount"));
        assert!(values.contains("3"));
    }

    #[test]
    fn extract_values_empty_config() {
        let json =
            r#"{"name": "x", "version": 1, "info": {}, "chart": {"metadata": {}}, "config": {}}"#;
        let data = make_helm_secret_data(json);
        let values = extract_values_from_release(&data).expect("should extract");
        assert!(values.contains("No custom values"));
    }

    #[test]
    fn extract_values_rejects_oversized_decompressed_payload() {
        let err = extract_values_from_release(&make_oversized_helm_secret_data()).unwrap_err();
        assert!(err.to_string().contains("exceeds"));
    }

    #[test]
    fn redact_sensitive_values_redacts_password() {
        let mut val = serde_json::json!({
            "database": {
                "host": "db.example.com",
                "password": "s3cret!",
                "port": 5432
            },
            "apiKey": "ak-12345",
            "nested": {
                "auth_token": "tok-xyz"
            }
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["database"]["host"], "db.example.com");
        assert_eq!(val["database"]["password"], REDACTED);
        assert_eq!(val["database"]["port"], 5432);
        assert_eq!(val["apiKey"], REDACTED);
        assert_eq!(val["nested"]["auth_token"], REDACTED);
    }

    #[test]
    fn redact_sensitive_values_handles_arrays() {
        let mut val = serde_json::json!([
            {"name": "svc1", "secret": "hidden"},
            {"name": "svc2", "url": "https://example.com"}
        ]);
        redact_sensitive_values(&mut val);

        assert_eq!(val[0]["name"], "svc1");
        assert_eq!(val[0]["secret"], REDACTED);
        assert_eq!(val[1]["name"], "svc2");
        assert_eq!(val[1]["url"], "https://example.com");
    }

    #[test]
    fn redact_sensitive_values_case_insensitive() {
        let mut val = serde_json::json!({
            "DB_PASSWORD": "foo",
            "AccessKey": "bar",
            "connectionString": "Server=x",
            "normalKey": "visible"
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["DB_PASSWORD"], REDACTED);
        assert_eq!(val["AccessKey"], REDACTED);
        assert_eq!(val["connectionString"], REDACTED);
        assert_eq!(val["normalKey"], "visible");
    }

    #[test]
    fn redact_sensitive_values_redacts_nested_strings_under_sensitive_objects() {
        let mut val = serde_json::json!({
            "password": 12345,
            "auth": true,
            "secret": {
                "nested": "value",
                "enabled": true
            }
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["password"], 12345);
        assert_eq!(val["auth"], true);
        assert_eq!(val["secret"]["nested"], REDACTED);
        assert_eq!(val["secret"]["enabled"], true);
    }

    #[test]
    fn redact_sensitive_values_recursively_redacts_auth_nested_strings() {
        let mut val = serde_json::json!({
            "auth": {
                "username": "admin",
                "password": "secret123",
                "apiKey": "key-xyz",
                "nested": {
                    "token": "tok-abc",
                    "url": "https://example.com"
                }
            },
            "normalField": "visible"
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["auth"]["username"], REDACTED);
        assert_eq!(val["auth"]["password"], REDACTED);
        assert_eq!(val["auth"]["apiKey"], REDACTED);
        assert_eq!(val["auth"]["nested"]["token"], REDACTED);
        assert_eq!(val["auth"]["nested"]["url"], REDACTED);
        assert_eq!(val["normalField"], "visible");
    }

    #[test]
    fn redact_sensitive_values_recursively_redacts_credentials_nested_strings() {
        let mut val = serde_json::json!({
            "database": {
                "credentials": {
                    "username": "dbuser",
                    "password": "dbpass",
                    "connection": {
                        "host": "db.example.com",
                        "port": 5432
                    }
                }
            }
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["database"]["credentials"]["username"], REDACTED);
        assert_eq!(val["database"]["credentials"]["password"], REDACTED);
        assert_eq!(val["database"]["credentials"]["connection"]["host"], REDACTED);
        assert_eq!(val["database"]["credentials"]["connection"]["port"], 5432);
    }

    #[test]
    fn redact_sensitive_values_handles_arrays_under_auth() {
        let mut val = serde_json::json!({
            "auth": [
                {"username": "user1", "password": "pass1"},
                {"username": "user2", "password": "pass2"}
            ]
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["auth"][0]["username"], REDACTED);
        assert_eq!(val["auth"][0]["password"], REDACTED);
        assert_eq!(val["auth"][1]["username"], REDACTED);
        assert_eq!(val["auth"][1]["password"], REDACTED);
    }

    #[test]
    fn redact_sensitive_values_mixed_recursive_and_key_based() {
        let mut val = serde_json::json!({
            "service": {
                "apiKey": "visible-key",
                "auth": {
                    "type": "oauth",
                    "client_id": "id-123"
                }
            },
            "database": {
                "password": "db-secret",
                "host": "localhost"
            }
        });
        redact_sensitive_values(&mut val);

        assert_eq!(val["service"]["apiKey"], REDACTED);
        assert_eq!(val["service"]["auth"]["type"], REDACTED);
        assert_eq!(val["service"]["auth"]["client_id"], REDACTED);
        assert_eq!(val["database"]["password"], REDACTED);
        assert_eq!(val["database"]["host"], "localhost");
    }


    #[test]
    fn resolve_helm_binary_prefers_explicit_absolute_path() {
        let helm_binary = TestTempFile::new("configured");
        let resolved = resolve_helm_binary_path_with(Some(helm_binary.path.clone()), Vec::new())
            .expect("configured path should resolve");

        assert_eq!(resolved, helm_binary.path.canonicalize().unwrap());
    }

    #[test]
    fn resolve_helm_binary_rejects_relative_paths() {
        let err = resolve_helm_binary_path_with(Some(PathBuf::from("helm")), Vec::new())
            .expect_err("relative path should fail");

        assert!(err.to_string().contains("absolute path"));
    }

    #[test]
    fn resolve_helm_binary_uses_trusted_candidate() {
        let helm_binary = TestTempFile::new("trusted");
        let resolved = resolve_helm_binary_path_with(None, vec![helm_binary.path.clone()])
            .expect("trusted candidate should resolve");

        assert_eq!(resolved, helm_binary.path.canonicalize().unwrap());
    }

    #[test]
    fn resolve_helm_binary_requires_explicit_trust_when_not_found() {
        let err = resolve_helm_binary_path_with(None, Vec::new()).expect_err("missing binary");

        assert!(err.to_string().contains(HELM_BINARY_PATH_ENV));
    }

    #[test]
    fn map_helm_uninstall_error_handles_release_not_found() {
        let msg = map_helm_uninstall_error(
            "Error: uninstall: Release not loaded: demo: release: not found",
            "",
        );
        assert!(msg.contains("release not found"));
    }

    #[test]
    fn map_helm_uninstall_error_handles_permission_denied() {
        let msg = map_helm_uninstall_error(
            "Error: uninstall: failed to delete: secrets is forbidden: User cannot delete resource",
            "",
        );
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn map_helm_uninstall_error_handles_timeout() {
        let msg = map_helm_uninstall_error("Error: context deadline exceeded", "");
        assert!(msg.contains("operation timed out"));
    }

    #[tokio::test]
    async fn helm_uninstall_with_valid_release_name_succeeds() {
        let _env_lock = helm_env_lock().lock().unwrap();
        let helm_binary = create_mock_helm_binary("uninstall-success", "", 0);
        let _env = ScopedEnvVar::set(HELM_BINARY_PATH_ENV, &helm_binary.path);

        let result = helm_uninstall("default", "demo-release").await;

        assert_eq!(
            result.unwrap(),
            "Uninstalled Helm release demo-release from namespace default"
        );
    }

    #[tokio::test]
    async fn helm_uninstall_with_nonexistent_release_returns_clear_error() {
        let _env_lock = helm_env_lock().lock().unwrap();
        let helm_binary = create_mock_helm_binary(
            "uninstall-not-found",
            "Error: uninstall: Release not loaded: missing-release: release: not found",
            1,
        );
        let _env = ScopedEnvVar::set(HELM_BINARY_PATH_ENV, &helm_binary.path);

        let err = helm_uninstall("default", "missing-release")
            .await
            .unwrap_err();
        let msg = err.to_string();

        assert!(msg.contains("release not found"));
        assert!(msg.contains("via"));
    }

    #[tokio::test]
    async fn helm_uninstall_with_empty_release_name_returns_validation_error() {
        let err = helm_uninstall("default", "").await.unwrap_err();

        assert!(err.to_string().contains("Invalid name"));
    }
}

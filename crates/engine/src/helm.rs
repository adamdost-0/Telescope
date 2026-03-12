//! Helm release management via Kubernetes Secrets.
//!
//! Helm stores releases as Secrets with type `helm.sh/release.v1`.
//! The secret data is base64-encoded, gzipped JSON containing release metadata.
//! This module parses those secrets directly — no `helm` binary required.

use std::collections::HashMap;
use std::io::Read;

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
fn parse_helm_release(data: &[u8]) -> Result<HelmRelease, Box<dyn std::error::Error>> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)?;
    let mut decoder = GzDecoder::new(&decoded[..]);
    let mut json_str = String::new();
    decoder.read_to_string(&mut json_str)?;

    let release: serde_json::Value = serde_json::from_str(&json_str)?;

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
pub fn extract_values_from_release(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data)?;
    let mut decoder = GzDecoder::new(&decoded[..]);
    let mut json_str = String::new();
    decoder.read_to_string(&mut json_str)?;

    let release: serde_json::Value = serde_json::from_str(&json_str)?;
    let config = &release["config"];
    if config.is_null() || (config.is_object() && config.as_object().unwrap().is_empty()) {
        return Ok("# No custom values configured\n".to_string());
    }
    Ok(serde_yaml::to_string(config)?)
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
        Some((_, data)) => extract_values_from_release(&data)
            .map_err(|e| crate::EngineError::Other(format!("Failed to extract values: {e}"))),
        None => Err(crate::EngineError::Other(format!(
            "Release \"{name}\" not found in namespace \"{namespace}\""
        ))),
    }
}

/// Validate that a string is a valid Kubernetes resource name (RFC 1123 DNS subdomain).
fn validate_k8s_name(name: &str) -> crate::Result<()> {
    let re = regex::Regex::new(r"^[a-z0-9]([a-z0-9\-\.]*[a-z0-9])?$").unwrap();
    if name.is_empty() || name.len() > 253 || !re.is_match(name) {
        return Err(crate::EngineError::Other(format!("Invalid name: {}", name)));
    }
    Ok(())
}

/// Roll back a Helm release to a specific revision using the `helm` CLI.
pub async fn rollback_release(namespace: &str, name: &str, revision: i32) -> crate::Result<String> {
    validate_k8s_name(namespace)?;
    validate_k8s_name(name)?;

    let output = std::process::Command::new("helm")
        .args([
            "rollback",
            name,
            &revision.to_string(),
            "--namespace",
            namespace,
        ])
        .output()
        .map_err(|e| crate::EngineError::Other(format!("helm CLI not found: {e}")))?;

    if output.status.success() {
        Ok(format!("Rolled back {name} to revision {revision}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(crate::EngineError::Other(format!(
            "Rollback failed: {stderr}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    fn make_helm_secret_data(json: &str) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let gzipped = encoder.finish().unwrap();
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &gzipped);
        b64.into_bytes()
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
}

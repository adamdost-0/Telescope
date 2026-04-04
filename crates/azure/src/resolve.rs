//! AKS resource identity resolution.
//!
//! Bridges K8s kubeconfig context with the Azure management plane by resolving
//! the subscription ID, resource group, and cluster name for an AKS cluster.

use std::path::PathBuf;

use crate::types::AksResourceId;
use serde::{Deserialize, Serialize};
use telescope_core::resolve_trusted_binary;
use telescope_core::store::ResourceStore;
use tracing::{debug, warn};

#[cfg(target_os = "windows")]
const TRUSTED_AZ_BINARY_PATHS: &[&str] = &[
    r"C:\Program Files\Azure\CLI2\wbin\az.cmd",
    r"C:\Program Files (x86)\Microsoft SDKs\Azure\CLI2\wbin\az.cmd",
    r"C:\Program Files\Microsoft SDKs\Azure\CLI2\wbin\az.cmd",
    r"C:\ProgramData\chocolatey\bin\az.cmd",
];

#[cfg(not(target_os = "windows"))]
const TRUSTED_AZ_BINARY_PATHS: &[&str] = &[
    "/usr/local/bin/az",
    "/opt/homebrew/bin/az",
    "/usr/bin/az",
    "/snap/bin/az",
];

/// Legacy global preference keys retained only for cleanup after migrating to
/// scoped per-cluster AKS identity overrides.
pub const PREF_AZURE_SUBSCRIPTION: &str = "azure_subscription";
pub const PREF_AZURE_RESOURCE_GROUP: &str = "azure_resource_group";
pub const PREF_AZURE_CLUSTER_NAME: &str = "azure_cluster_name";
const PREF_AKS_IDENTITY_OVERRIDE_PREFIX: &str = "aks_identity_override";

/// Scoped manual AKS identity fields saved for a specific cluster FQDN.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AksIdentityPreferences {
    #[serde(default)]
    pub subscription_id: Option<String>,
    #[serde(default)]
    pub resource_group: Option<String>,
    #[serde(default)]
    pub cluster_name: Option<String>,
}

impl AksIdentityPreferences {
    pub fn has_any(&self) -> bool {
        self.subscription_id.is_some()
            || self.resource_group.is_some()
            || self.cluster_name.is_some()
    }

    pub fn to_resource_id(&self) -> Option<AksResourceId> {
        Some(AksResourceId {
            subscription_id: self.subscription_id.clone()?,
            resource_group: self.resource_group.clone()?,
            cluster_name: self.cluster_name.clone()?,
        })
    }

    fn normalized(self) -> Self {
        Self {
            subscription_id: normalize_saved_value(self.subscription_id),
            resource_group: normalize_saved_value(self.resource_group),
            cluster_name: normalize_saved_value(self.cluster_name),
        }
    }
}

/// Status of saved AKS identity preferences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AksIdentityPreferenceStatus {
    Missing,
    Incomplete { missing_fields: Vec<&'static str> },
    Complete(AksResourceId),
}

/// Resolve the AKS resource identity for a given API server URL.
///
/// Resolution precedence:
/// 1. **Saved preferences** — manual overrides stored per cluster in `ResourceStore`
/// 2. **Azure CLI** — `az aks list` filtered by FQDN
/// 3. **Kubeconfig hints** — FQDN region extraction (partial)
///
/// Returns `None` for non-AKS clusters or when resolution fails entirely.
pub async fn resolve_aks_identity(
    server_url: &str,
    preferred_id: Option<AksResourceId>,
) -> Option<AksResourceId> {
    if !server_url.contains(".azmk8s.io") && !server_url.contains(".azmk8s.us") {
        return None;
    }

    // 1. Check saved preferences
    if let Some(id) = preferred_id {
        debug!("AKS identity resolved from saved preferences");
        return Some(id);
    }

    // 2. Try Azure CLI
    if let Some(id) = resolve_from_az_cli(server_url).await {
        debug!("AKS identity resolved via az CLI");
        return Some(id);
    }

    // 3. Kubeconfig / FQDN hints (partial — region only, no subscription/RG)
    debug!("AKS identity could not be fully resolved; az CLI may not be installed");
    None
}

/// Read saved AKS identity overrides from the resource store.
pub fn resolve_aks_identity_from_preferences(
    store: Option<&ResourceStore>,
    server_url: &str,
) -> Option<AksResourceId> {
    match inspect_aks_identity_preferences(store, server_url) {
        AksIdentityPreferenceStatus::Complete(id) => Some(id),
        AksIdentityPreferenceStatus::Missing | AksIdentityPreferenceStatus::Incomplete { .. } => {
            None
        }
    }
}

/// Read the scoped AKS identity preference draft for a specific cluster URL.
pub fn read_aks_identity_preferences(
    store: Option<&ResourceStore>,
    server_url: &str,
) -> AksIdentityPreferences {
    let Some(store) = store else {
        return AksIdentityPreferences::default();
    };
    let Some(key) = scoped_aks_identity_preference_key(server_url) else {
        return AksIdentityPreferences::default();
    };

    let stored = match store.get_preference(&key) {
        Ok(value) => value,
        Err(error) => {
            warn!("Failed to read AKS identity override for {server_url}: {error}");
            return AksIdentityPreferences::default();
        }
    };

    let Some(serialized) = stored else {
        return AksIdentityPreferences::default();
    };

    match serde_json::from_str::<AksIdentityPreferences>(&serialized) {
        Ok(preferences) => preferences.normalized(),
        Err(error) => {
            warn!("Failed to parse AKS identity override for {server_url}: {error}");
            AksIdentityPreferences::default()
        }
    }
}

/// Save or clear the scoped AKS identity preference draft for a specific cluster URL.
pub fn save_aks_identity_preferences(
    store: &ResourceStore,
    server_url: &str,
    preferences: AksIdentityPreferences,
) -> Result<(), String> {
    let Some(key) = scoped_aks_identity_preference_key(server_url) else {
        return Err("AKS identity overrides can only be saved for AKS clusters".to_string());
    };

    let preferences = preferences.normalized();
    if preferences.has_any() {
        let serialized = serde_json::to_string(&preferences)
            .map_err(|error| format!("Failed to serialize AKS identity override: {error}"))?;
        store
            .set_preference(&key, &serialized)
            .map_err(|error| format!("Failed to save AKS identity override: {error}"))?;
    } else {
        store
            .delete_preference(&key)
            .map_err(|error| format!("Failed to clear AKS identity override: {error}"))?;
    }

    // Clean up the legacy global preference keys so stale values cannot keep
    // lingering in the store after users resave scoped overrides.
    for legacy_key in [
        PREF_AZURE_SUBSCRIPTION,
        PREF_AZURE_RESOURCE_GROUP,
        PREF_AZURE_CLUSTER_NAME,
    ] {
        store
            .delete_preference(legacy_key)
            .map_err(|error| format!("Failed to remove legacy AKS identity preference: {error}"))?;
    }

    Ok(())
}

/// Inspect saved AKS identity preference completeness.
pub fn inspect_aks_identity_preferences(
    store: Option<&ResourceStore>,
    server_url: &str,
) -> AksIdentityPreferenceStatus {
    let preferences = read_aks_identity_preferences(store, server_url);

    if !preferences.has_any() {
        return AksIdentityPreferenceStatus::Missing;
    }

    let subscription = preferences.subscription_id;
    let resource_group = preferences.resource_group;
    let cluster_name = preferences.cluster_name;
    let mut missing_fields = Vec::new();
    if subscription.is_none() {
        missing_fields.push("subscription");
    }
    if resource_group.is_none() {
        missing_fields.push("resource group");
    }
    if cluster_name.is_none() {
        missing_fields.push("cluster name");
    }

    if missing_fields.is_empty() {
        AksIdentityPreferenceStatus::Complete(AksResourceId {
            subscription_id: subscription.expect("subscription present when no fields missing"),
            resource_group: resource_group.expect("resource group present when no fields missing"),
            cluster_name: cluster_name.expect("cluster name present when no fields missing"),
        })
    } else {
        AksIdentityPreferenceStatus::Incomplete { missing_fields }
    }
}

/// Create an actionable error when an AKS cluster cannot be mapped to an ARM resource.
pub fn unresolved_aks_identity_message(
    server_url: &str,
    preference_status: &AksIdentityPreferenceStatus,
) -> String {
    let cluster_hint = extract_fqdn(server_url).unwrap_or_else(|| server_url.to_string());
    let base = format!(
        "Connected cluster looks like AKS, but Telescope could not resolve its Azure resource ID for {cluster_hint}."
    );
    match preference_status {
        AksIdentityPreferenceStatus::Incomplete { missing_fields } => format!(
            "{base} Saved AKS identity settings for this cluster are incomplete; add {} in Settings for this cluster, or sign in with Azure CLI so `az aks list` can resolve the cluster.",
            format_field_list(missing_fields)
        ),
        AksIdentityPreferenceStatus::Missing | AksIdentityPreferenceStatus::Complete(_) => format!(
            "{base} Save Azure subscription, resource group, and cluster name in Settings for this cluster, or sign in with Azure CLI so `az aks list` can resolve the cluster."
        ),
    }
}

fn scoped_aks_identity_preference_key(server_url: &str) -> Option<String> {
    extract_fqdn(server_url).map(|fqdn| {
        format!(
            "{PREF_AKS_IDENTITY_OVERRIDE_PREFIX}:{}",
            fqdn.to_ascii_lowercase()
        )
    })
}

fn normalize_saved_value(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn format_field_list(fields: &[&'static str]) -> String {
    match fields {
        [] => String::new(),
        [single] => single.to_string(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let head = fields[..fields.len() - 1].join(", ");
            format!("{head}, and {}", fields[fields.len() - 1])
        }
    }
}

/// Row shape returned by `az aks list`.
#[derive(Deserialize)]
struct AzAksEntry {
    id: Option<String>,
    #[serde(alias = "resourceGroup")]
    resource_group: Option<String>,
    name: Option<String>,
    #[allow(dead_code)]
    fqdn: Option<String>,
}

/// Attempt resolution via the Azure CLI (`az aks list`).
///
/// Runs `az aks list -o json` and filters by the FQDN extracted from
/// `server_url`. Gracefully returns `None` when `az` is not installed.
async fn resolve_from_az_cli(server_url: &str) -> Option<AksResourceId> {
    let fqdn = extract_fqdn(server_url)?;
    let az_binary = match resolve_az_binary_path() {
        Ok(path) => path,
        Err(error) => {
            debug!("az CLI not available or not trusted: {error}");
            return None;
        }
    };

    let output = match tokio::process::Command::new(&az_binary)
        .args([
            "aks",
            "list",
            "--query",
            &format!("[?fqdn=='{fqdn}']"),
            "-o",
            "json",
        ])
        .kill_on_drop(true)
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            debug!(
                "trusted az CLI could not be executed at {}: {e}",
                az_binary.display()
            );
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("az aks list failed: {stderr}");
        return None;
    }

    let entries: Vec<AzAksEntry> = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(e) => {
            warn!("Failed to parse az aks list output: {e}");
            return None;
        }
    };

    let entry = entries.into_iter().next()?;

    // The `id` field is the full ARM resource ID; extract subscription from it.
    let arm_id = entry.id?;
    let subscription_id = extract_subscription_from_arm_id(&arm_id)?;

    Some(AksResourceId {
        subscription_id,
        resource_group: entry.resource_group?,
        cluster_name: entry.name?,
    })
}

fn resolve_az_binary_path() -> Result<PathBuf, String> {
    resolve_az_binary_path_with("az", TRUSTED_AZ_BINARY_PATHS.iter().map(PathBuf::from))
}

fn resolve_az_binary_path_with<I>(command: &str, trusted_paths: I) -> Result<PathBuf, String>
where
    I: IntoIterator<Item = PathBuf>,
{
    resolve_trusted_binary(command, trusted_paths)
}

/// Extract the FQDN from a server URL.
/// `https://myaks-dns-abc123.hcp.eastus.azmk8s.io:443` → `myaks-dns-abc123.hcp.eastus.azmk8s.io`
pub fn extract_fqdn(server_url: &str) -> Option<String> {
    let url = server_url
        .strip_prefix("https://")
        .or_else(|| server_url.strip_prefix("http://"))?;
    let host = url.split(':').next()?;
    Some(host.to_string())
}

/// Extract subscription ID from an ARM resource ID.
/// `/subscriptions/SUB_ID/resourceGroups/...` → `SUB_ID`
fn extract_subscription_from_arm_id(arm_id: &str) -> Option<String> {
    let parts: Vec<&str> = arm_id.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if part.eq_ignore_ascii_case("subscriptions") {
            return parts.get(i + 1).map(|s| s.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use telescope_core::store::ResourceStore;

    #[test]
    fn extract_fqdn_from_aks_url() {
        let url = "https://myaks-dns-abc123.hcp.eastus.azmk8s.io:443";
        assert_eq!(
            extract_fqdn(url).unwrap(),
            "myaks-dns-abc123.hcp.eastus.azmk8s.io"
        );
    }

    #[test]
    fn extract_fqdn_no_port() {
        let url = "https://myaks.hcp.westus2.azmk8s.io";
        assert_eq!(extract_fqdn(url).unwrap(), "myaks.hcp.westus2.azmk8s.io");
    }

    #[test]
    fn extract_fqdn_non_url_returns_none() {
        assert!(extract_fqdn("not-a-url").is_none());
    }

    #[test]
    fn extract_subscription_from_valid_arm_id() {
        let arm_id = "/subscriptions/abc-123/resourceGroups/my-rg/providers/Microsoft.ContainerService/managedClusters/my-aks";
        assert_eq!(extract_subscription_from_arm_id(arm_id).unwrap(), "abc-123");
    }

    #[test]
    fn extract_subscription_invalid_returns_none() {
        assert!(extract_subscription_from_arm_id("/something/else").is_none());
    }

    #[test]
    fn resolve_az_binary_path_accepts_trusted_binary_name() {
        let executable = std::env::current_exe().expect("current test executable");
        let name = executable
            .file_name()
            .and_then(|value| value.to_str())
            .expect("binary name");

        let resolved = resolve_az_binary_path_with(name, vec![executable.clone()])
            .expect("trusted az path should resolve");

        assert_eq!(resolved, executable.canonicalize().expect("canonical path"));
    }

    #[test]
    fn resolve_az_binary_path_rejects_relative_paths() {
        let err = resolve_az_binary_path_with("./az", Vec::<PathBuf>::new())
            .expect_err("relative az path should be blocked");

        assert!(err.contains("relative or qualified path"));
    }

    #[test]
    fn non_aks_url_returns_none() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let result = rt.block_on(resolve_aks_identity("https://k8s.example.com:6443", None));
        assert!(result.is_none());
    }

    #[test]
    fn unresolved_identity_message_mentions_missing_fields_for_partial_preferences() {
        let message = unresolved_aks_identity_message(
            "https://demo.hcp.eastus.azmk8s.io:443",
            &AksIdentityPreferenceStatus::Incomplete {
                missing_fields: vec!["resource group", "cluster name"],
            },
        );

        assert!(message.contains("demo.hcp.eastus.azmk8s.io"));
        assert!(message.contains("resource group and cluster name"));
        assert!(message.contains("`az aks list`"));
        assert!(message.contains("for this cluster"));
    }

    #[test]
    fn unresolved_identity_message_mentions_settings_for_missing_preferences() {
        let message = unresolved_aks_identity_message(
            "https://demo.hcp.eastus.azmk8s.io:443",
            &AksIdentityPreferenceStatus::Missing,
        );

        assert!(message.contains("Save Azure subscription, resource group, and cluster name"));
    }

    #[test]
    fn scoped_saved_preferences_only_apply_to_matching_cluster() {
        let store = ResourceStore::open(":memory:").expect("open store");
        save_aks_identity_preferences(
            &store,
            "https://demo-one.hcp.eastus.azmk8s.io:443",
            AksIdentityPreferences {
                subscription_id: Some("sub-one".to_string()),
                resource_group: Some("rg-one".to_string()),
                cluster_name: Some("cluster-one".to_string()),
            },
        )
        .expect("save scoped preferences");

        let matching = resolve_aks_identity_from_preferences(
            Some(&store),
            "https://demo-one.hcp.eastus.azmk8s.io:443",
        );
        let different = resolve_aks_identity_from_preferences(
            Some(&store),
            "https://demo-two.hcp.eastus.azmk8s.io:443",
        );

        assert_eq!(
            matching,
            Some(AksResourceId {
                subscription_id: "sub-one".to_string(),
                resource_group: "rg-one".to_string(),
                cluster_name: "cluster-one".to_string(),
            })
        );
        assert_eq!(different, None);
    }

    #[test]
    fn partial_scoped_preferences_report_incomplete_for_that_cluster() {
        let store = ResourceStore::open(":memory:").expect("open store");
        save_aks_identity_preferences(
            &store,
            "https://demo.hcp.eastus.azmk8s.io:443",
            AksIdentityPreferences {
                subscription_id: Some("sub-demo".to_string()),
                resource_group: None,
                cluster_name: None,
            },
        )
        .expect("save partial preferences");

        assert_eq!(
            inspect_aks_identity_preferences(Some(&store), "https://demo.hcp.eastus.azmk8s.io:443"),
            AksIdentityPreferenceStatus::Incomplete {
                missing_fields: vec!["resource group", "cluster name"],
            }
        );
        assert_eq!(
            inspect_aks_identity_preferences(
                Some(&store),
                "https://other.hcp.eastus.azmk8s.io:443"
            ),
            AksIdentityPreferenceStatus::Missing
        );
    }
}
